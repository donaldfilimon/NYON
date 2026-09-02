# NYON Galaxy Workshop Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the pure deterministic core for validated Workshop catalogs, recorded galaxy creation, 10 Hz industrial simulation, immutable branch history, and canonical archives.

**Architecture:** `nyon-workshop-core` is a platform-free workspace member depended on by the root client. It owns all authoritative bytes, IDs, validation, state transitions, digests, replay, and archive validation; it never imports root `nyon`, wgpu, winit, Tokio, Web APIs, wall clocks, or storage.

**Tech Stack:** Rust nightly-2026-09-01, edition 2024, `serde`, `serde_json`, `sha2`, `thiserror`, native and `wasm32-unknown-unknown` targets.

**Spec:** `docs/superpowers/specs/2026-09-02-nyon-v2-design.md`

## Global Constraints

- Complete verified NYON identity restoration before this plan.
- Keep RulesV1 source, tests, goldens, direct paths, storage, and timing behavior unchanged.
- Keep `nyon-workshop-core` `#![forbid(unsafe_code)]` and free of platform/client dependencies.
- Use only ordered collections, checked integer arithmetic, explicit versioned encoding, and domain-separated SHA-256.
- Enforce all count and byte caps before allocation or mutation where possible.
- A failed command, step, replay, or decode never partially changes valid state.
- Use one catalog grammar, one canonical encoder, one command representation, and one replay path.
- Do not add network identity, permissions, signatures, scripts, remote content, or deferred simulation concepts.

---

### Task 1: Freeze RulesV1 and establish the workspace boundary

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/lib.rs`
- Create: `src/classic.rs`
- Create: `src/workshop/mod.rs`
- Create: `crates/nyon-workshop-core/Cargo.toml`
- Create: `crates/nyon-workshop-core/src/lib.rs`
- Create: `tests/rules_v1_facade.rs`
- Create: `tests/workshop_dependency_boundary.rs`

**Interfaces:**
- Consumes: existing public `nyon::game`, `nyon::scenario`, `nyon::engine::time`, and renderer layouts.
- Produces: `nyon::classic` re-exports and an empty, compilable `nyon_workshop_core` public root.

- [ ] **Step 1: Write the failing RulesV1 façade test**

~~~rust
use nyon::classic::{RulesV1, Simulation, DEFAULT_SEED};

#[test]
fn rules_v1_facade_preserves_default_oracles() {
    let simulation = Simulation::new(DEFAULT_SEED, RulesV1::default());
    assert_eq!(simulation.campaign().worlds.len(), 7);
    assert_eq!(RulesV1::default().base_fleet_speed, 23);
    assert_eq!(simulation.state_digest(), 0x67D9_6E98_3D6C_9330);
    assert_eq!(nyon::engine::primitives::Vertex::LAYOUT.array_stride, 36);
}
~~~

Use current constructors and field accessors exactly as defined on disk. Do not add compatibility methods merely to simplify the test.

- [ ] **Step 2: Run the focused test and confirm the missing façade**

~~~bash
cargo test --test rules_v1_facade
~~~

Expected: compile failure because `nyon::classic` does not exist.

- [ ] **Step 3: Add a dependency-boundary test**

Read `crates/nyon-workshop-core/Cargo.toml` as TOML text and reject `nyon`, `wgpu`, `winit`, `tokio`, `web-sys`, `wasm-bindgen`, `web-time`, and path dependencies. Also assert the pure crate has no `unsafe` token outside comments and is compiled with `#![forbid(unsafe_code)]`.

- [ ] **Step 4: Add the two-member workspace**

~~~toml
[workspace]
members = [".", "crates/nyon-workshop-core"]
default-members = ["."]
resolver = "3"
~~~

Add the pure crate as a root dependency. Its runtime dependency list is exactly:

~~~toml
[dependencies]
serde = { version = "1.0.219", features = ["derive"] }
serde_json = "1.0.143"
sha2 = "=0.10.9"
thiserror = "2.0.16"
~~~

- [ ] **Step 5: Add re-export-only module roots**

`src/classic.rs` re-exports current RulesV1 types without moving them. `src/workshop/mod.rs` re-exports the pure crate. `crates/nyon-workshop-core/src/lib.rs` declares `ids`, `pack`, `model`, `command`, `simulation`, `history`, and `archive`.

- [ ] **Step 6: Run native, wasm, dependency, and RulesV1 gates**

~~~bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
git diff --check
~~~

- [ ] **Step 7: Commit the reviewed boundary**

~~~bash
git add Cargo.toml Cargo.lock crates/nyon-workshop-core src/classic.rs src/workshop src/lib.rs tests/rules_v1_facade.rs tests/workshop_dependency_boundary.rs
git commit -m "refactor(workspace): isolate the workshop core"
~~~

### Task 2: Implement strict validated data packs

**Files:**
- Create: `crates/nyon-workshop-core/src/pack.rs`
- Create: `crates/nyon-workshop-core/tests/pack.rs`
- Create: `assets/workshop/core-pack-v1.json`
- Modify: `crates/nyon-workshop-core/src/lib.rs`

**Interfaces:**
- Consumes: raw `&[u8]` bounded to 1 MiB.
- Produces: `CatalogId`, `PackValidationError`, private-field `ValidatedCatalogPackV1`, `CatalogHash`, `decode_catalog_pack`, and `encode_catalog_pack`.

- [ ] **Step 1: Write failing parser-boundary tests**

Create tests named:

- `pack_rejects_oversized_input_before_parse`
- `pack_rejects_bom_invalid_utf8_and_trailing_value`
- `pack_rejects_duplicate_keys_at_every_depth`
- `pack_rejects_unknown_fields_floats_and_excess_depth`
- `pack_rejects_bad_ids_counts_ranges_and_references`
- `pack_rejects_executable_or_remote_fields`

The duplicate-key test must include both a duplicated root key and a duplicated recipe quantity key. The oversized case is exactly 1,048,577 bytes and must return `PackTooLarge { actual, limit: 1_048_576 }` without calling serde.

- [ ] **Step 2: Run focused tests and verify failures**

~~~bash
cargo test -p nyon-workshop-core --test pack
~~~

Expected: compile failure because pack types and decoder are absent.

- [ ] **Step 3: Implement bounded duplicate-aware JSON decoding**

Use a serde visitor that rejects duplicate keys while constructing raw schema types with `#[serde(deny_unknown_fields)]`. Run a byte-level BOM/UTF-8/depth preflight before deserialization. Reject trailing values by requiring the deserializer to end after one root value.

Define:

~~~rust
pub const MAX_PACK_BYTES: usize = 1_048_576;

pub fn decode_catalog_pack(
    bytes: &[u8],
) -> Result<ValidatedCatalogPackV1, PackValidationError>;

pub fn encode_catalog_pack(
    pack: &ValidatedCatalogPackV1,
) -> Result<Vec<u8>, PackValidationError>;
~~~

Only the public decoder constructs `ValidatedCatalogPackV1`.

- [ ] **Step 4: Implement exact semantic validation**

Enforce all ID, name, description, count, quantity, duration, recipe, extractor, solar-source, and reference rules in the design. Validate in stable path order and return field-addressable errors without including untrusted raw documents in `Display`.

- [ ] **Step 5: Write canonicalization and hash tests**

~~~rust
#[test]
fn pack_hash_ignores_source_collection_order() {
    let first = decode_catalog_pack(FIRST_ORDER.as_bytes()).unwrap();
    let second = decode_catalog_pack(SECOND_ORDER.as_bytes()).unwrap();
    assert_eq!(first.catalog_hash(), second.catalog_hash());
    assert_eq!(encode_catalog_pack(&first).unwrap(), encode_catalog_pack(&second).unwrap());
}

#[test]
fn pack_round_trip_preserves_hash() {
    let first = decode_catalog_pack(CORE_PACK).unwrap();
    let bytes = encode_catalog_pack(&first).unwrap();
    let second = decode_catalog_pack(&bytes).unwrap();
    assert_eq!(first.catalog_hash(), second.catalog_hash());
}
~~~

- [ ] **Step 6: Add and freeze the core pack**

The committed JSON includes only energy, ore, alloy, yellow-dwarf, rocky-world, solar-array, extractor, foundry, and ion-storm with the exact quantities from the design. Decode it through the public path in a test and freeze the resulting SHA-256 catalog hash.

- [ ] **Step 7: Run focused, full, wasm, and diff gates**

~~~bash
cargo test -p nyon-workshop-core --test pack
cargo test --workspace --all-targets
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
git diff --check
~~~

- [ ] **Step 8: Commit the validated catalog**

~~~bash
git add assets/workshop crates/nyon-workshop-core
git commit -m "feat(catalog): add validated workshop data packs"
~~~

### Task 3: Implement authoritative IDs, model, and atomic creator batches

**Files:**
- Create: `crates/nyon-workshop-core/src/ids.rs`
- Create: `crates/nyon-workshop-core/src/model.rs`
- Create: `crates/nyon-workshop-core/src/command.rs`
- Create: `crates/nyon-workshop-core/tests/creator.rs`
- Modify: `crates/nyon-workshop-core/src/lib.rs`

**Interfaces:**
- Consumes: `ValidatedCatalogPackV1`, genesis seed, current cursor, tick, ordinal, and `CreatorBatchV1`.
- Produces: `WorkshopStateV1`, `WorkshopAuthority`, `CreatorReceiptV1`, `CreatorRejectionV1`, canonical state bytes, and `StateDigest`.

- [ ] **Step 1: Write failing coordinate, identity, and atomicity tests**

Tests must prove:

- Plus or minus 131,072 parsecs is accepted and one unit outside is rejected.
- Distance uses checked `i128` intermediates and deterministic integer square root.
- Same parent, tick, ordinal, and batch produce identical revision and entity IDs.
- Local references resolve only to an earlier compatible create operation.
- Duplicate local IDs reject.
- Stale cursor and stale tick reject.
- A failed operation leaves bytes, digest, ordinal, entity counts, inventory, reserves, and history length unchanged.
- `RemoveObject` lists all current dependents and never cascades.
- Every entity and command cap returns its typed capacity error.

Capture the entire pre-submit authority with `encode_authority_for_test()` and assert exact byte equality after rejection.

- [ ] **Step 2: Define stable IDs and canonical model**

Use transparent fixed-size wrappers for IDs and `BTreeMap<EntityId, T>` for every authoritative collection. Define distinct entity structs for faction, system, star, world, lane, deposit, industry, route, shipment, and hazard. Store no float, wall-clock, backend, camera, or display-layout value.

- [ ] **Step 3: Define the complete creator API**

Implement `CreatorBatchV1` and every `CreatorOpV1` variant in the design. Define:

~~~rust
pub fn submit(
    &mut self,
    batch: CreatorBatchV1,
) -> Result<CreatorReceiptV1, CreatorRejectionV1>;
~~~

The receipt contains revision, tick, ordinal, sorted local-ID mappings, and post-batch digest.

- [ ] **Step 4: Implement validate-then-swap**

Reject over 128 operations or over 65,536 canonical bytes before cloning state. Validate local-reference direction and kind, then apply to temporary authority. Compute the revision hash before entity IDs. Detect truncated-ID collision before swapping. Increment ordinal only after every operation, invariant, canonical encoding, and digest succeeds.

- [ ] **Step 5: Add canonical-order invariance tests**

Construct equivalent states through different insertion orders and assert identical canonical bytes and `StateDigest`. Ensure names, branch labels, camera, speed, and client diagnostics are absent from state encoding where the design excludes them.

- [ ] **Step 6: Run focused and full gates**

~~~bash
cargo test -p nyon-workshop-core --test creator
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
git diff --check
~~~

- [ ] **Step 7: Commit atomic recorded creation**

~~~bash
git add crates/nyon-workshop-core
git commit -m "feat(workshop): add recorded galaxy creation"
~~~

### Task 4: Implement the deterministic 10 Hz industrial simulation

**Files:**
- Create: `crates/nyon-workshop-core/src/simulation.rs`
- Create: `crates/nyon-workshop-core/tests/simulation.rs`
- Create: `crates/nyon-workshop-core/tests/two_system_forge.rs`
- Modify: `crates/nyon-workshop-core/src/lib.rs`

**Interfaces:**
- Consumes: validated catalog and `WorkshopStateV1`.
- Produces: `WorkshopSimulation::step()`, canonical tick events, shipments, inventories, reserve changes, and post-step digest.

- [ ] **Step 1: Write failing phase and boundary tests**

Prove exact phase order:

1. Hazard boundary transition.
2. Shipment delivery by shipment ID.
3. Solar and extractor execution by industry ID.
4. Foundry execution by industry ID.
5. Route dispatch by route ID.
6. Event finalization, digest, and tick increment.

Test that delivery is usable in the same tick, newly produced inventory may dispatch in phase 5, and no shipment arrives on its dispatch tick.

- [ ] **Step 2: Write arithmetic and pacing-invariance tests**

Test checked overflow at every addition, subtraction, multiplication, tick increment, arrival calculation, and reserve/inventory transfer. A fault must preserve pre-step state. Compare 100 direct `step()` calls with every grouping used by 1x, 4x, and 20x session pacing and assert identical final bytes and digest.

- [ ] **Step 3: Implement lane distance and travel**

Store lane distance in 1/1024-parsec units at lane creation. Same-world transport is one tick. Inter-system route travel is `max(1, ceil(distance / 256))` and requires one direct lane.

- [ ] **Step 4: Implement the built-in economy**

Use catalog definitions rather than hard-coded UI behavior. Solar produces 4 energy. Extractor consumes 1 energy and up to 2 reserve units to produce matching ore. Foundry consumes 2 energy and 3 ore to produce 1 alloy. When inputs are insufficient, the industry performs no partial recipe.

- [ ] **Step 5: Implement ion-storm semantics**

An ion storm is active on its exact half-open interval. It applies only to its target lane and halves route batch capacity with integer floor division. A resulting zero capacity dispatches nothing and consumes nothing.

- [ ] **Step 6: Add the headless Two-System Forge test**

Build the acceptance galaxy only through creator batches, step until alloy exists under a bounded maximum tick, schedule the storm, and assert the exact affected shipment sequence. The test must also create a world while the simulation tick is greater than zero.

- [ ] **Step 7: Run focused and full gates**

~~~bash
cargo test -p nyon-workshop-core --test simulation
cargo test -p nyon-workshop-core --test two_system_forge
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
git diff --check
~~~

- [ ] **Step 8: Commit deterministic simulation**

~~~bash
git add crates/nyon-workshop-core
git commit -m "feat(workshop): add deterministic galaxy authority"
~~~

### Task 5: Implement immutable branches, replay, and canonical archives

**Files:**
- Create: `crates/nyon-workshop-core/src/history.rs`
- Create: `crates/nyon-workshop-core/src/archive.rs`
- Create: `crates/nyon-workshop-core/tests/history.rs`
- Create: `crates/nyon-workshop-core/tests/archive.rs`
- Modify: `crates/nyon-workshop-core/src/lib.rs`

**Interfaces:**
- Consumes: accepted revisions, genesis, catalog hash, pure simulation, and canonical batches.
- Produces: `WorkshopHistoryV1`, `BranchRefV1`, `ActiveViewV1`, undo/redo/branch selection, verified checkpoints, `encode_workshop_archive`, and `decode_workshop_archive`.

- [ ] **Step 1: Write failing immutable-history tests**

Build `A -> B`, advance to tick `T`, undo to `A`, and compare with clean replay of `A` through `T`. Redo `B` and require the exact prior digest. Undo again, submit `C`, and assert `B` and `C` are siblings while the original branch head remains `B`.

- [ ] **Step 2: Write branching and limit tests**

Assert:

- Multiple redo children require explicit `RevisionId` selection.
- Automatic names use the smallest unused positive `Branch N`.
- Branch names, branch creation order, selection, and checkpoint cadence do not change authority digest.
- The 65th branch and 10,001st revision return capacity errors without moving the active cursor or head.
- Undo rejects while running or while a load/save replacement flag is active.

- [ ] **Step 3: Implement immutable revision storage and materialization**

Store revisions by content-addressed `RevisionId` with parent, tick, ordinal, and canonical batch. Branch refs are mutable pointers only. Materialization chooses a verified ancestor checkpoint, replays revisions at recorded ticks, and advances empty ticks to the branch tick.

Create a derived checkpoint after 600 ticks or 128 accepted revisions, whichever occurs first. A checkpoint stores authoritative bytes plus digest and is discarded if verification fails.

- [ ] **Step 4: Write failing archive integrity tests**

Reject archive bytes over 16 MiB, unknown fields or versions, duplicate revision IDs, hash mismatch, missing parent, cycles, disconnected revisions, invalid tick/ordinal ordering, invalid operation references, invalid branch heads, missing catalog, and final digest mismatch. Failed decode must not mutate a supplied current session.

- [ ] **Step 5: Implement canonical archive encoding**

`encode_workshop_archive` emits stable minified JSON containing format identity, rules version, catalog hash, genesis seed, revisions in ID order, branch refs in branch-ID order, active view, per-branch ticks, and final digest. Checkpoints are excluded.

`decode_workshop_archive` validates structure and all hashes, materializes every referenced branch head in temporary state, verifies the active digest, then returns one fully validated value.

- [ ] **Step 6: Prove replay and archive invariance**

Save/reload, export/import, full replay, and replay from every permissible checkpoint boundary must produce equal authority bytes and digest. Re-encoding a decoded archive must reproduce identical bytes.

- [ ] **Step 7: Run complete core gates**

~~~bash
cargo test -p nyon-workshop-core --test history
cargo test -p nyon-workshop-core --test archive
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
git diff --check
~~~

- [ ] **Step 8: Review dependency and authority boundaries**

Search the pure crate for `f32`, `f64`, `Instant`, `SystemTime`, `HashMap`, platform crates, rendering types, storage calls, and unsafe code. Every match must be removed or, for parser-only floating-value rejection types, justified in the review report without entering authority.

- [ ] **Step 9: Commit history and archives**

~~~bash
git add crates/nyon-workshop-core
git commit -m "feat(workshop): add immutable branch history"
~~~

## Core Completion

The core plan is complete when the pure crate passes native and wasm compilation, every rejected operation and fault preserves valid state byte-for-byte, Two-System Forge produces alloy deterministically, undo/redo/branching recover exact digests, archive re-encoding is byte-stable, and all RulesV1 oracles remain unchanged.
