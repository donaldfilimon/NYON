# NYON Living Galaxy Authority and Persistence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a strictly versioned Living Galaxy V2 canonical authority, immutable replay history, separate native/browser persistence, one cross-authority Continue coordinator, and a bounded import router while preserving Classic RulesV1 and WorkshopV1 byte-for-byte.

**Architecture:** `nyon-workshop-core::living` is a pure deterministic authority island with distinct identifiers, canonical JSON, catalog, genesis/state, creator queue, receipts, history, replay, and archives. Root modules provide a separate `LivingStoreV2`, neutral Library coordinator, and read-only import router; they never widen WorkshopV1 or create an implicit V1-to-V2 conversion.

**Tech Stack:** Rust nightly-2026-09-01, edition 2024, serde/serde_json, SHA-256 via `sha2 = 0.10.9`, thiserror, native filesystem transactions, browser IndexedDB through web-sys, and the existing two-member workspace.

**Spec:** `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-design.md`, `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md`, `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-experience.md`, and `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-qualification.md`.

## Checklist status (recorded 2026-09-08)

**⚠️ REWRITTEN 2026-09-08 19:4x. The paragraph that stood here contradicted itself
and the tree, and it is the first thing an implementer of the next task reads.**
It said "Tasks 2, 3 and 4 are not started" and "Tasks 2-4 have no files on disk —
no `living/catalog.rs`, `living/model.rs` … and no `assets/living/core-pack-v2.json`",
and then stated two lines later that Task 2 had landed. All three of those files
exist. It also wrote the paths as `src/living/…`; they are under
`crates/nyon-workshop-core/src/living/`, a different crate from `nyon`.

Measured against the tree rather than carried forward:

| Task | State | Evidence |
|---|---|---|
| 1 — canonical V2 wire identities | **landed** | `1283e55`, plus `4f3d28e` isolation-guard hardening. `crates/nyon-workshop-core/src/living/{mod,ids,wire}.rs`, `tests/living_wire.rs`, `tests/fixtures/living-v2/vectors.json` |
| 2 — validated rules catalog | **landed** | `71e5b6f`. `living/catalog.rs`, `assets/living/core-pack-v2.json`, 24 tests |
| 3a — the V2 authority state schema | **landed and reviewed** | `50c50f9` (1,848 lines + 50 integration tests), review `b6f33ab` APPROVE WITH FINDINGS; F1 closed by `2b7bfbc`, F2 by `06873f6`, F3 recorded in the program plan |
| 3b — `living/genesis.rs` | **landed** | `f569437` (genesis manifest and validated genesis), `4fc7f4b` (envelope field order pin). `living/genesis.rs`, `tests/living_genesis.rs`. *Corrected 2026-09-17: this row said "open, no file on disk" after both commits.* |
| 3c — genesis and state vectors | open, **blocked** on `DECISIONS-PENDING.md` entry 1 (genesis manifest wire schema) | derived outside this crate, never from its output. Task 5 does not wait on it and is the next unblocked task |
| 4 — commands, receipts, events | **landed** | `2f6561a` publishes the entity-kind and phase registries in `living/ids.rs` and the rules spec, and re-derives the five affected corpus rows outside this crate with `tools/living-v2-vectors.py`; `bbc3da6` adds `living/receipt.rs` (the distinct payload record, the normative derivation order, 26 event kinds); `ab1f081` adds `living/command.rs` (28 creator operations, envelope, revision). The entity-kind renumbering note the spec asked for is in its status block. 633 tests / 46 suites |

**Two things 3b must not re-derive, both of which already exist.**
`LivingGalaxyStateV2::validate(&ValidatedLivingCatalogPackV2)` landed with 3a —
roughly 520 lines of referential-integrity and range checking — so **3b's
"validate" step is a call to that method, not a second implementation**. And
`LivingGenesisGeneratorV2` is **provenance only**, an ID-and-version record: the
task wording reads as though it writes a generator and it does not, since the spec
forbids rerunning generator code on decode. The actual generators are Experience
Task 4.

The box counts below were never maintained and should be read as intent, not state.

**The three normative gaps that blocked Tasks 3 and 4 are CLOSED as of
2026-09-08.** They were resolved on the owner's decision and written into
`docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` as normative
text: `canonical_receipt_bytes` now encodes a distinct `LivingReceiptPayloadV2`
with an explicit derivation order; `accepted_sequence` and `branch_sequence` are
split into frozen in-state history and durable allocator high-water marks that
are never inputs to `state_digest`, with two new archive fields; and the phase
and intent ordinal registries are published, with two new queue limits tied to
the declared work-unit poll bound. Tasks 3 and 4 may now freeze canonical
vectors.

Two decisions differ from the review's proposal and matter to anyone reading it:
**phases are numbered from 1, not 0** (the step list is numbered from 1, and the
committed vectors label phase 8 as the AI fleet case and phase 9 as the route
shipment case, which is only correct one-based), and the `LivingEventKindV2`
ordinal is **deliberately unassigned**, because tagged enums travel the wire as
snake_case strings and `event_digest` consumes an emission ordinal rather than a
kind, so no byte or hash depends on it.

Vectors must still be derived from spec text, never from running the
implementation, as Task 1's were.

## Global Constraints

- Preserve `#![forbid(unsafe_code)]`, the two-member workspace, Classic RulesV1, WorkshopV1 byte formats, storage identities, limits, phase order, fixtures, and goldens.
- Living V2 uses rules version 2, 10 ticks/second, native `living-v2`, browser `nyon.living.v2`, 64 slots, 256 packs, 32 MiB archives, 2 MiB packs, 64 branches, and 10,000 creator revisions.
- Authority maxima are 16 civilizations, 64 systems, 128 stars, 512 worlds, 256 lanes, 1,024 deposits, 2,048 industries, 2,048 routes, 4,096 shipments, 256 fleets, 2,048 hulls, 16 hulls/fleet, and 128 hazards.
- Canonical V2 JSON is UTF-8 without BOM/whitespace/floats. Every field is present and ordered; optional values are explicit null/value; map-like data is a sorted array. Reject duplicate/unknown/reordered fields, invalid UTF-8, noncanonical escapes, depth above 32, unsorted/duplicate keys, and decode/re-encode inequality.
- V2 IDs/hashes use distinct wrappers and lowercase fixed-width hex. Domain literals and little-endian framing are exactly those in the rules spec; never reinterpret a V1 wrapper as V2.
- `state.tick` counts completed boundaries. A step from `t` evaluates `T=t+1` and commits state, revisions, receipts, counters, and digest atomically or changes nothing.
- Rejected/stale commands consume no sequence, branch ordinal, entity ID, queue mutation, or resource. Accepted and branch sequences are document-global, monotonic, persisted, and never reused.
- Replay, decode, validation, import, and stores process at most 1,024 declared work units per poll and never publish partial authority.
- Checkpoints are derived caches excluded from archives. Archives embed validated genesis and creator revisions only; autonomous decisions replay deterministically.
- No failed load/import/save/coordinator operation replaces a valid session. No V2 operation writes Workshop or Classic keys, paths, manifests, packs, or archives.
- Presentation, audio, GPU, platform, wall clock, map insertion, and UI preferences never affect authority or digests.
- Before each task, require a clean index and stage only its exact path list. Preserve all unrelated dirty/untracked work.

## Frozen Public Handoff

```rust
pub const LIVING_RULES_VERSION: u32 = 2;
pub const LIVING_TICK_HZ: u32 = 10;

pub struct LivingCommandCursorV2 {
    pub committed_revision: Option<LivingRevisionIdV2>,
    pub tick: LivingTickV2,
    pub pending_sequence: u64,
}

pub trait LivingStoreV2 {
    fn begin(&mut self, owner: LivingOwnerEpochV2, request: LivingStoreRequestV2)
        -> Result<LivingStoreJobIdV2, LivingStoreErrorV2>;
    fn poll(&mut self, owner: LivingOwnerEpochV2, job: LivingStoreJobIdV2, budget: u16)
        -> Result<LivingStorePollV2, LivingStoreErrorV2>;
    fn cancel(&mut self, owner: LivingOwnerEpochV2, job: LivingStoreJobIdV2)
        -> Result<LivingStoreCancelV2, LivingStoreErrorV2>;
}

pub trait LibraryCoordinatorV1 {
    fn begin(&mut self, owner: LibraryOwnerEpochV1, request: LibraryCoordinatorRequestV1)
        -> Result<LibraryCoordinatorJobIdV1, LibraryCoordinatorErrorV1>;
    fn poll(&mut self, owner: LibraryOwnerEpochV1, job: LibraryCoordinatorJobIdV1)
        -> Result<LibraryCoordinatorPollV1, LibraryCoordinatorErrorV1>;
    fn cancel(&mut self, owner: LibraryOwnerEpochV1, job: LibraryCoordinatorJobIdV1)
        -> Result<LibraryCoordinatorCancelV1, LibraryCoordinatorErrorV1>;
}

pub fn preflight_import(bytes: &[u8])
    -> Result<ImportPreflightV1, ImportRouteErrorV1>;
```

### Task 1: Add canonical wire primitives, V2 identities, and hash domains

**Files:** Create `crates/nyon-workshop-core/src/living/{mod,wire,ids}.rs`, `crates/nyon-workshop-core/tests/living_wire.rs`, `crates/nyon-workshop-core/tests/fixtures/living-v2/vectors.json`; modify `crates/nyon-workshop-core/src/lib.rs`.

- [ ] Write failing exact-vector tests for catalog/state/archive/receipt domains, fixed-width wrappers, revision/entity/root/fork/event/claim framing, optional tags, and identity-tuple reuse.
- [ ] Run `cargo test -p nyon-workshop-core --test living_wire`; expect unresolved `living` APIs.
- [ ] Implement `LivingRevisionIdV2`, `LivingEntityIdV2`, `LivingBranchIdV2`, `LivingCatalogHashV2`, `LivingStateDigestV2`, `LivingReceiptDigestV2`, `LivingEventIdV2`, and `LivingTickV2` without V1 conversions.
- [ ] Implement strict type-directed `decode_canonical_v2`/`encode_canonical_v2` with byte/depth bounds and exact re-encoding.
- [ ] Freeze rules-spec domains `NYON-LIVING-{PACK,STATE,ARCHIVE-INTEGRITY,RECEIPT,REVISION,CREATOR-ENTITY,AUTO-ENTITY,ROOT-BRANCH,FORK-BRANCH,EVENT,CLAIM}-V2\0` in reviewed vectors.
- [ ] Run the focused test, wasm core check, fmt, and workspace clippy.
- [ ] Commit exact paths with `feat(living): add canonical V2 wire identities`.

### Task 2: Add the validated catalog and built-in rules pack

**Files:** Create `crates/nyon-workshop-core/src/living/catalog.rs`, `crates/nyon-workshop-core/tests/living_catalog.rs`, `crates/nyon-workshop-core/tests/fixtures/living-v2/minimal-pack.json`, `assets/living/core-pack-v2.json`; modify Living exports/vectors.

- [ ] Write failing exact-byte/hash tests and assertions for energy/ore/alloy; solar/extractor/foundry/shipyard/battery; scout/ark/escort; ion storm; and five policies.
- [ ] Implement `ValidatedLivingCatalogPackV2` with private validated data, canonical bytes/hash, and typed lookup methods.
- [ ] Require `NYON_LIVING_GALAXY_DATA`, format/rules version 2, ordered arrays, unique IDs, valid references, nonzero periods/durations, and declared ranges.
- [ ] Mutation-test wrong kind/version, unknown/missing/reordered fields, duplicate/unsorted definitions, dangling resource references, floats, BOM, depth 33, and 2 MiB plus one.
- [ ] Run catalog, wasm, fmt, and clippy gates.
- [ ] Commit exact paths with `feat(living): validate the V2 rules catalog`.

### Task 3: Freeze the complete state schema and validated genesis

**Files:** Create `crates/nyon-workshop-core/src/living/{model,genesis}.rs`, common fixtures/helpers, `living_genesis.rs`, `minimal-genesis.json`, `minimal-state.json`, and the decoder fuzz target; modify exports/vectors/fuzz manifest.

- [ ] Write failing tick-zero byte/digest and invalid-genesis atomicity tests.
- [ ] Implement `LivingGalaxyStateV2` with ordered systems, stars, worlds, lanes, civilizations, deposits, colonies, facilities/jobs, fleets/hulls, routes/shipments, relations/agreements/wars, observations, hazards, claims, occupations, and counters.
- [ ] Implement `LivingGenesisGeneratorV2`, `LivingGenesisManifestV2`, and `ValidatedLivingGenesisV2`; materialization clones the embedded validated state and never reruns a generator.
- [ ] Validate global uniqueness, stable ordering, all references, collection maxima, inventory cap 10,000, colony/facility/fleet/route rules, end-exclusive lifecycles, clocks, and checked arithmetic.
- [ ] Add boundary/one-over tests for every maximum and a mutation for every reference/invariant family.
- [ ] Complete a fuzz target that round-trips successful pack/genesis decodes exactly.
- [ ] Run genesis, wasm, fuzz fmt/clippy, workspace fmt/clippy.
- [ ] Commit exact paths with `feat(living): validate V2 genesis and state`.

### Task 4: Freeze creator commands, revisions, receipts, and events

**Files:** Create `crates/nyon-workshop-core/src/living/{command,receipt}.rs`, `living_command.rs`, command/revision/creator-receipt/tick-receipt fixtures; modify exports/vectors.

- [ ] Write failing exact-byte and identity tests for command, revision, receipt, event, and tagged-enum ordering.
- [ ] Implement `LivingCommandV2::CreatorBatch` and typed operations covering system/star/world/lane/deposit, inventory/colony/facility/jobs, civilization/policy/relations/diplomacy, fleets/orders, routes, hazards, shipment disposal, and explicit cascades.
- [ ] Require unique backward-only `batch_local_id` references and complete exact cascade dispositions.
- [x] Implement `LivingAcceptedCommandV2`, `LivingRevisionV2`, `LivingTickReceiptV2`, provenance, and the frozen event kinds from the rules spec. **Landed 2026-09-08 (`bbc3da6`, `ab1f081`).** ⚠️ **This line used to end "with explicit `u16` ordinals", and that was wrong** — the rules spec at line 469 *deliberately leaves* `LivingEventKindV2`'s ordinal unassigned, and this plan's own prose at lines 56-62 says so too, so line 159 contradicted its own document. The implementer followed the spec over the plan, correctly, and the review confirmed the plan was the stale half.
- [ ] Centralize event and autonomous identity allocation in `LivingStepContextV2`; subsystem code cannot fabricate IDs.
- [ ] Test unknown variants, wrong field order, forward/duplicate local IDs, oversized batches, V1 bytes, stable ordering, and authority-only digest fields.
- [ ] Run command, wasm, fmt, and clippy gates.
- [ ] Commit exact paths with `feat(living): freeze V2 commands and receipts`.

### Task 5: Implement the tail-aware creator queue and atomic boundary orchestration

**Files:** Create `crates/nyon-workshop-core/src/living/simulation.rs`, `crates/nyon-workshop-core/tests/living_queue.rs`; modify Living command/receipt/exports. Civilization phase modules are supplied by the civilization plan.

- [ ] Write failing tests for two commands chained against successive pending tails, stale/future tail rejection, projected conflicts, fault atomicity, and non-reused sequences.
- [ ] Implement `LivingGalaxyAuthorityV2::{from_genesis,state,catalog,published_cursor,submit,apply_paused,step}`.
- [ ] Submission compares committed revision, completed tick, and pending tail before allocation; validates against a private projection containing earlier accepted same-boundary commands.
- [ ] Paused application requires an empty running queue, commits at the current completed tick, and still records a revision/receipt atomically.
- [ ] Permanently order phases: creator commands; lifecycles; arrivals; combat/occupation/claims; jobs/recipes; observations; diplomacy; intent generation/reservation; dispatch/freight; validation/digests/commit.
- [ ] Clone candidate state/history, execute every phase, validate, compute receipts/digests, then swap all authority fields once. On fault retain queue and last valid state, pause, and return typed diagnostics.
- [ ] Test that newly accepted orders cannot depart on the same boundary, insertion order cannot change output, and an empty/no-civilization galaxy continues without terminal victory.
- [ ] Run queue/command/civilization/wasm/fmt/clippy gates.
- [ ] Commit exact paths with `feat(living): apply queued commands atomically`.

### Task 6: Implement immutable history, bounded replay, and canonical archives

**Files:** Create `crates/nyon-workshop-core/src/living/{history,replay,archive}.rs`, tests `living_{history,replay,archive}.rs`, `minimal-archive.json`; modify exports/vectors/fuzz target.

- [ ] Write failing sibling-branch and same-tick Undo/Redo tests: undo removes the latest creator intervention at the viewed tick; redo restores its exact digest; historical edits preserve both futures.
- [ ] Implement `LivingBranchV2`, `LivingActiveViewV2`, and `LivingHistoryV2::{new,submit,step,undo,redo_to,select_branch}`. Allocate fork ordinals only after accepted validation and never reuse them.
- [ ] Implement `LivingReplayJobV2::poll` with `min(requested,1024)` units and private continuation `(revision,tick,phase,item)` so a poll may stop inside a tick without publishing partial state.
- [ ] Encode `NYON_LIVING_GALAXY_ARCHIVE`, format/rules 2, catalog hash, seed, generator provenance, embedded genesis, creator revisions, branches, active view, final tick/digest, and integrity. Never serialize checkpoints or pack bytes.
- [ ] Preflight validates only bounded canonical envelope/integrity/catalog identity; full decode validates genesis, revision graph, replay, final tick, and final digest.
- [ ] Test 18,000 save/reload to 36,000 parity, insertion invariance, one-unit vs 1,024-unit polling, cancellation, graph cycles, generator-code drift, catalog/integrity/final mismatch, 32 MiB plus one, and V1 wrong-kind rejection.
- [ ] Extend fuzzing to archive preflight/full decode and exact round-trip; run a real 60-second `cargo fuzz run living_decode` separately from compile/lint evidence.
- [ ] Run focused, workspace, wasm, fuzz compile, and real fuzz gates.
- [ ] Commit exact paths with `feat(living): add immutable V2 replay archives`.

### Task 7: Define LivingStoreV2 and an in-memory conformance oracle

**Files:** Create `src/living/{mod,store/mod,store/memory}.rs`, `tests/living_store.rs`; modify `src/lib.rs`.

- [ ] Write failing owner-epoch, bounded-progress, cancel-before/after-commit, generation, and capacity tests.
- [ ] Implement distinct owner/job/slot/save-generation/pack-generation/name wrappers; typed requests for list/load/create/save/rename/archive/delete/read/put-pack/archive-pack/delete-pack; pending/ready/failed polls; `Cancelled` vs `CommitAlreadyWon`.
- [ ] Descriptors include byte length, ordinary SHA-256, catalog hash, generation, lifecycle, retained-generation state, and item-level corruption diagnostics.
- [ ] `begin` performs bounded top-level preflight only. `poll` incrementally validates/replays and caps work at 1,024. Mutation publishes only at the final commit state.
- [ ] Enforce generation CAS, exactly two valid slot generations, 64 slots/256 packs, content-hash pack deduplication, reversible archive without reclaimed capacity, archived-only slot deletion, and archived+unreferenced pack deletion across all retained slots.
- [ ] For every request test success, not found, stale, wrong owner, one-unit progress, cancellation boundaries, size/kind/canonical/integrity/catalog/capacity errors, and no partial mutation.
- [ ] Run living-store/archive/fmt/clippy gates.
- [ ] Commit exact paths with `feat(living): define the V2 storage protocol`.

### Task 8: Implement native transactional persistence and recovery

**Files:** Create `src/living/store/native.rs`, `tests/living_store_native.rs`; modify store exports.

- [ ] Write failing platform-root and recovery tests. Roots end in macOS `Library/Application Support/NYON/living-v2`, Windows `NYON/living-v2`, and Linux `NYON/living-v2` under XDG or `.local/share`; never `workshop-v1`.
- [ ] Implement two independently canonical/hash-described manifests plus an atomic head marker. Persist/fsync data and candidate manifest/directories before replacing the head; head replacement is the commit point.
- [ ] Lock, refresh, and compare generations per transaction. Recovery validates head, then alternate retained manifest; chooses the highest independently valid referenced generation; diagnoses unreachable staging; returns `CorruptRecovery` if neither is valid.
- [ ] Add fault injection before/after archive, pack, manifest, head, and delete commits. Reopen after every fault and prove the old or complete new transaction wins, never a mixture.
- [ ] Test cancellation during validation, after staging, before head, and after head; post-head cancellation reports `CommitAlreadyWon`.
- [ ] Run native and in-memory conformance, fmt, and clippy.
- [ ] Commit exact paths with `feat(living): persist V2 galaxies natively`.

### Task 9: Implement browser IndexedDB transactions

**Files:** Create `src/living/store/web.rs`, `tests/living_store_web.rs`; modify store exports.

- [ ] Write failing identity and abort tests for database `nyon.living.v2`, version 1, and metadata/slots/archives/packs stores; assert it differs from `nyon.workshop.v1`.
- [ ] Implement a deterministic host transaction model with Abort, QuotaDenied, Unavailable, SchemaMismatch, and Evicted faults; only successful completion publishes cloned candidate state.
- [ ] Implement real IndexedDB jobs. Incremental validation/replay precedes one final read-write transaction covering all affected records; only transaction completion publishes `Ready`; invalidate handles on version changes.
- [ ] Test abort/quota/unavailable/schema/eviction/stale/archive/reference/delete-race/owner-replacement and commit-won cancellation parity.
- [ ] Run host model, store conformance, root/core wasm checks, and wasm clippy.
- [ ] Commit exact paths with `feat(living): persist V2 galaxies in IndexedDB`.

### Task 10: Add the version-independent global Continue coordinator

**Files:** Create `src/library/{mod,selection,native,web}.rs`, `tests/library_selection{,_native,_web}.rs`; modify `src/lib.rs`.

- [ ] Write failing canonical encode/decode, cleared-generation, and stale-CAS tests.
- [ ] Implement neutral `LibraryBranchIdV1([u8;16])`, `LibraryRevisionIdV1([u8;32])`, `LibraryDigestV1([u8;32])`, `LibraryAuthorityV1::{LivingV2,WorkshopV1}`, target, record, job-shaped `Load`/`CompareAndSwap`, and owner-epoch Begin/Poll/Cancel.
- [ ] Canonical fields are kind, format_version, generation, authority, slot_id, branch_id, revision_id, tick, digest, integrity. A cleared selection is a new valid generation with every target field explicit null.
- [ ] Implement checked conversions at authority adapters; never admit neutral/Workshop wrappers directly into Living authority methods.
- [ ] On first V2-capable launch only, an absent coordinator may migrate one valid Workshop selected slot to generation 1. Once any coordinator generation exists, never choose Continue by adapter recency/enumeration. Classic is never a target.
- [ ] Native coordinator uses independent A/B generations and head outside both authority roots. Browser uses `nyon.library.v1` and one read-write transaction.
- [ ] Test both authorities locally selected, absent/cleared/corrupt coordinator, target missing/archived/corrupt/missing catalog, digest/branch/generation mismatch, update failure, and refusal to silently choose another save.
- [ ] Run all coordinator, wasm, fmt, and clippy gates.
- [ ] Commit exact paths with `feat(library): coordinate cross-authority Continue`.

### Task 11: Add bounded import routing and prove V1 isolation

**Files:** Create `src/library/import_router.rs`, `tests/import_router.rs`, `tests/living_v1_protection.rs`, `crates/nyon-workshop-core/tests/living_v1_isolation.rs`; modify Library exports.

- [ ] Write failing routing tests for Living archive/pack V2 and Workshop archive/pack V1 plus a no-store-mutation assertion.
- [ ] Implement bounded canonical top-level recognition of kind/format/rules/byte length only. Preflight does not decode, save, convert, overwrite, validate full integrity, or establish authority.
- [ ] Prove LivingStore rejects V1 as wrong authority, router never submits V2 to WorkshopStore, direct wrong-authority calls mutate nothing, and import has no overwrite path.
- [ ] Freeze V1 oracles: RulesV1 digest and seven-world behavior; Workshop canonical pack/archive/digests; 16-slot, 16 MiB archive, 1 MiB pack, 256-pack limits; `workshop-v1`; `nyon.workshop.v1`; legacy/primary precedence.
- [ ] Run import, V1 protection, Classic, Workshop store/core, workspace, wasm, fuzz, fmt, and clippy gates.
- [ ] Diff frozen V1 implementation paths and require no changes. Scan completed Living source/tests for unfinished markers.
- [ ] Commit exact paths with `feat(library): route imports without changing V1`.

## Cross-Plan Dependency Order

```text
Authority Tasks 1-4
        |
        +--> Civilization Tasks 1-10 implement phase modules
        |
        v
Authority Tasks 5-6
        |
        +--> Experience may integrate frozen authority/history
        |
        v
Authority Tasks 7-11
        |
        v
Whole-product qualification
```

Authority owns core Living schema/orchestration/history/archive, root Living stores, Library coordinator/router, built-in pack, and canonical fixtures. Civilization owns only deterministic phase behavior modules and scenario witnesses. Foundation owns layout/renderer contracts. Experience owns client session, views, import/export UX, starters, audio, and qualification orchestration.

Implementation is complete only after focused, workspace, native filesystem, host IndexedDB-model, wasm, compatibility, and real fuzz gates pass. Live browser durability and native/browser product acceptance remain separate evidence.
