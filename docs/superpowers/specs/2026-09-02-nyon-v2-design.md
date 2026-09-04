# NYON Galaxy Workshop V1 Design

| Metadata | Value |
| --- | --- |
| Developer | Donald Filimon |
| Date | 2026-09-02 |
| Status | Accepted for implementation |
| Product | NYON Galaxy Workshop V1 |
| Active roadmap | `docs/superpowers/plans/2026-09-02-nyon-v2.md` |

The accepted [Workshop V1 Library and Portable Transfer Addendum](2026-09-04-nyon-workshop-library-addendum.md) supplies implementation authority for the multi-slot Library, reversible archive lifecycle, exact-generation Continue selection, and portable native/browser transfer boundaries. It supersedes only earlier notes that those surfaces required a further design decision.

## Authority and Current State

This design replaces the prior mega-platform roadmap with one executable product program: an offline creative galaxy sandbox. The historical [Intergalactic Warfare design](2026-09-02-intergalactic-warfare-design.md) remains the source of the frozen RulesV1 contracts, but it is not current execution authority.

The canonical checkout is `/Users/donaldfilimon/Public/NYON` on local `main` with no remote. The verified RulesV1 source tree was reconstructed there, and commit `cdfb903` restored the breaking NYON package, crate, binary, browser, product, and primary-storage identity with read-only legacy fallback. Repository relocation, baseline reconstruction, identity restoration, and promotion are completed historical operations and must not be repeated.

The following evidence classes remain distinct:

1. Source behavior and tests.
2. Native and WebAssembly compilation.
3. Generated native and browser artifacts.
4. Live native GPU runtime.
5. Live browser runtime.
6. Accessibility semantics and manual interaction.
7. Cross-platform performance and release qualification.

A stronger evidence label may never be inferred from a weaker one.

## Product Definition

The player is a **galaxy architect**. The first release is not a fleet-pilot game, multiplayer service, social space, or infrastructure platform.

The four product pillars are:

1. **Shape:** create and edit systems, stars, worlds, factions, deposits, industries, lanes, routes, and deterministic hazards.
2. **Observe:** pause, step, or run a fixed-tick industrial simulation and inspect why resources moved or production changed.
3. **Iterate:** record every accepted creator batch, navigate immutable history, and create alternate branches without destroying prior work.
4. **Share:** import and export validated non-executable content packs and complete Workshop archives.

### Two-System Forge

The first playable and release acceptance journey is named **Two-System Forge**:

1. Start a blank or deterministic seeded Workshop.
2. Create two systems.
3. Add one star and at least one world to each system.
4. Create two visual factions and assign ownership.
5. Add an ore deposit.
6. Place a solar array, extractor, and foundry.
7. Connect the systems with one lane.
8. Create energy and ore routes.
9. Run until at least one unit of alloy is produced.
10. Add or edit a world while the simulation continues to run.
11. Schedule an ion storm and observe its deterministic logistics effect.
12. Pause and undo the last accepted creator batch.
13. Redo it and recover the exact former digest.
14. Undo again, create an alternate edit, and preserve both sibling histories.
15. Switch between branches and recover each branch's prior digest.
16. Save, quit, and continue from the last explicitly selected valid branch.
17. Export and re-import both the content pack and Workshop archive.
18. Recover the same catalog hash, revision graph, selected branch, authoritative tick, and state digest.

There is no victory condition in Galaxy Workshop V1.

## Goals

- Preserve RulesV1 behavior, wire formats, deterministic oracles, and direct public paths.
- Add one pure `nyon-workshop-core` crate for deterministic Workshop authority.
- Support recorded live creation whether the Workshop is paused or running.
- Provide a bounded deterministic industrial simulation at 10 authoritative ticks per second.
- Provide immutable revision history with branch-based undo and explicit redo selection.
- Validate all imported data before it becomes constructible as simulation input.
- Provide crash-safe native and browser saves with generation conflict detection.
- Keep every Workshop action available without spatial canvas interaction.
- Produce distinct WebGPU and WebGL2 browser artifacts.
- Qualify native macOS, Windows, and Linux plus current Chrome, Edge, Safari, and Firefox using live runtime evidence.

## Non-Goals

Do not create crates, routes, schemas, placeholder screens, deployment files, configuration, or feature flags for:

- Accounts, OAuth, cloud identity, or cloud saves.
- Networking, workers, authoritative servers, lobbies, multiplayer, co-editing, or ranked play.
- Social graphs, chat, child-safety services, moderation services, or synchronized privacy state.
- Voice or LiveKit.
- AI empires, LLMs, recommendation services, or native model inference.
- Mobile clients.
- Kubernetes, Helm, Docker Compose, databases, Redis, or hosted deployment.
- Arbitrary scripting, JavaScript, WASM mods, executable plug-ins, runtime shaders, or remote assets.
- Detailed planetary cells, factory-floor simulation, fleet combat, pilot mode, diplomacy, branch merging, cherry-picking, or history rewriting.

These are separately approved future programs, not scaffolding targets.

## RulesV1 Compatibility Island

The root package remains the executable client. Existing RulesV1 implementation stays in place and receives a `nyon::classic` re-export façade. Workshop code must not move, trait-generalize, or silently alter:

- `RulesV1`, `GameCommand`, `CommandEnvelope`, or `Simulation`.
- Seven-world generation and generator draw order.
- Default seed `0x4947_5731_2026_0902`.
- RulesV1 base fleet speed `23`.
- Fixed 60 Hz RulesV1 phase order.
- Default digest `0x67D9_6E98_3D6C_9330`.
- Existing canonical scenario bytes and fingerprint domains.
- The 36-byte `Vertex` ABI.
- Existing `FixedClock`.
- `ScenarioStore`, `PreferencesStore`, and the `nyon.preferences.v1` key after identity restoration.
- Primary-read with non-destructive legacy fallback.

Legacy data remains read-only. A present primary NYON slot wins even when corrupt. Workshop implementation must not add copy-on-read migration, import markers, import reports, or legacy rewriting.

## Architecture

The workspace has exactly two members:

~~~toml
[workspace]
members = [".", "crates/nyon-workshop-core"]
default-members = ["."]
resolver = "3"
~~~

Dependency direction is one-way:

~~~text
nyon executable/client
    |-- existing RulesV1 implementation
    |-- rendering, input, platform, UI, and persistence adapters
    \-- nyon-workshop-core

nyon-workshop-core
    \-- serde, serde_json, sha2, thiserror
~~~

`nyon-workshop-core` is `#![forbid(unsafe_code)]`, contains no wgpu, winit, Web APIs, Tokio, networking, wall-clock, advisory, or platform-storage dependency, and compiles for native and `wasm32-unknown-unknown`.

The root client adds these boundaries:

- `src/classic.rs`: RulesV1 re-exports only.
- `src/workshop/mod.rs`: Workshop façade and pure-core re-exports.
- `src/workshop/session.rs`: pacing, active view, store-job coordination, and UI-action mailbox.
- `src/workshop/store.rs`: object-safe store interface plus memory, native, and browser adapters.
- `src/app/client_runtime.rs`: sibling Classic and Workshop session selection.
- `src/presentation/workshop.rs`: immutable presentation-frame extraction.
- `src/engine/backend.rs`: selected backend diagnostics.

The pure crate owns `ids`, `pack`, `model`, `command`, `simulation`, `history`, and `archive`.

### Runtime Shell

`AppCore` remains the Classic implementation. A new wrapper owns the product shell:

~~~rust
pub enum ClientScreen {
    MainMenu,
    ClassicSector,
    GalaxyWorkshop,
    Settings,
    Loading,
    RecoverableError,
}

pub enum ActiveSession {
    None,
    Classic,
    Workshop(WorkshopSession),
}
~~~

The menu contains only `NEW WORKSHOP`, valid `CONTINUE`, `CLASSIC SECTOR`, `SETTINGS`, `CREDITS`, and native-only `QUIT`. It does not expose inert future-product screens.

## Authoritative Types and Limits

Workshop authority uses integer state, checked arithmetic, `BTreeMap` collections, stable typed IDs, and canonical serialization.

~~~rust
pub struct WorkshopTick(pub u64);
pub struct RevisionId(pub [u8; 32]);
pub struct EntityId(pub [u8; 16]);
pub struct BranchId(pub [u8; 16]);
pub struct CatalogHash(pub [u8; 32]);
pub struct StateDigest(pub [u8; 32]);
pub struct BatchLocalId(pub u16);

pub struct CatalogId(Box<str>);
pub struct ObjectName(Box<str>);

pub struct GalaxyCoord(pub i64); // 1 / 1024 parsec

pub struct GalaxyPointV1 {
    pub x: GalaxyCoord,
    pub y: GalaxyCoord,
}

pub enum ObjectRefV1 {
    Existing(EntityId),
    Local(BatchLocalId),
}
~~~

Coordinates are bounded to plus or minus 131,072 parsecs on each axis. Distance calculations use checked `i128` intermediates and deterministic integer square root. A same-system route takes one tick. An inter-system route requires a direct lane:

~~~text
travel_ticks = max(1, ceil(distance_in_1_over_1024_parsec_units / 256))
~~~

Multi-hop routing is not part of V1.

`CatalogId` and `ObjectName` have private fields and can be obtained only through the validators in this design. The root branch ID is the first 128 bits of a domain-separated SHA-256 over catalog hash and genesis seed. A forked branch ID is the first 128 bits of a domain-separated SHA-256 over its source branch ID, fork cursor, and first new revision. Branch-ID collision is an atomic typed fault.

Hard limits are:

| Resource | Limit |
| --- | ---: |
| Factions | 16 |
| Systems | 64 |
| Stars | 128 |
| Worlds | 512 |
| Inter-system lanes | 256 |
| Deposits | 1,024 |
| Industries | 2,048 |
| Routes | 2,048 |
| In-flight shipments | 4,096 |
| Scheduled or active hazards | 128 |
| Operations per creator batch | 128 |
| Canonical creator-batch bytes | 64 KiB |
| History branches | 64 |
| Accepted revisions | 10,000 |
| Save slots | 16 |
| Canonical Workshop archive | 16 MiB |
| Raw data-pack input | 1 MiB |

Every limit returns a typed atomic error. A failure may not evict content, discard history, or partially mutate state.

## Built-In Content

The shipped validated core pack contains:

- Resources `energy`, `ore`, and `alloy`.
- `solar-array`: produces 4 energy units per tick.
- `extractor`: consumes 1 energy and converts up to 2 linked reserve units into 2 ore units per tick.
- `foundry`: consumes 2 energy plus 3 ore and produces 1 alloy per tick.
- Star archetype `yellow-dwarf`.
- World archetype `rocky-world`.
- Hazard `ion-storm`, which halves lane route capacity for its declared duration using integer floor division.

Factions are visual ownership labels. They have no autonomous policy, diplomacy, economy behavior, or victory logic.

## Recorded Creator Authority

One atomic creator batch is the unit of validation, history, replay, and undo:

~~~rust
pub struct CreatorBatchV1 {
    pub expected_cursor: Option<RevisionId>,
    pub expected_tick: WorkshopTick,
    pub operations: Vec<CreatorOpV1>,
}

pub enum CreatorOpV1 {
    CreateFaction { local: BatchLocalId, name: ObjectName, color_rgb: [u8; 3] },
    CreateSystem { local: BatchLocalId, name: ObjectName, position: GalaxyPointV1 },
    CreateStar {
        local: BatchLocalId,
        system: ObjectRefV1,
        name: ObjectName,
        archetype_id: CatalogId,
    },
    CreateWorld {
        local: BatchLocalId,
        system: ObjectRefV1,
        primary: ObjectRefV1,
        name: ObjectName,
        archetype_id: CatalogId,
        orbit_radius_milli_au: u32,
        orbit_period_ticks: u64,
        phase_millidegrees: u32,
    },
    ConnectLane { local: BatchLocalId, a: ObjectRefV1, b: ObjectRefV1 },
    CreateDeposit {
        local: BatchLocalId,
        world: ObjectRefV1,
        resource_id: CatalogId,
        reserve_units: u64,
    },
    SetOwner { target: ObjectRefV1, faction: Option<ObjectRefV1> },
    PlaceIndustry {
        local: BatchLocalId,
        world: ObjectRefV1,
        definition_id: CatalogId,
        linked_deposit: Option<ObjectRefV1>,
    },
    ConnectRoute {
        local: BatchLocalId,
        source: ObjectRefV1,
        destination: ObjectRefV1,
        resource_id: CatalogId,
        batch_units: u64,
    },
    SetIndustryEnabled { industry: ObjectRefV1, enabled: bool },
    RenameObject { target: ObjectRefV1, name: ObjectName },
    RemoveObject { target: ObjectRefV1 },
    ScheduleHazard {
        local: BatchLocalId,
        lane: ObjectRefV1,
        hazard_id: CatalogId,
        start_tick: WorkshopTick,
        duration_ticks: u64,
    },
    CancelHazard { hazard: ObjectRefV1 },
}
~~~

Local references may point only to a compatible create operation earlier in the same batch. The authority validates and applies to temporary state before swapping. `RemoveObject` is non-cascading and reports every blocking dependency.

The authority compares both expected cursor and tick. Accepted batches apply at the current authoritative tick even while paused. Multiple batches at one tick receive checked ascending ordinals. A rejected batch consumes no ordinal, identity, inventory, reserve, route slot, revision, or digest change.

A revision ID hashes the domain tag, rules version, catalog hash, genesis seed, parent revision, tick, ordinal, and canonical batch bytes. Created entity IDs are the first 128 bits of a domain-separated SHA-256 over revision ID, entity kind, and local ID. Collision is a typed atomic fault.

The receipt returns revision ID, tick, ordinal, local-to-entity mapping, and post-batch digest. The client may keep 100 recent rejection diagnostics, but rejections never enter canonical history.

## Deterministic Simulation

Workshop runs at 10 authoritative ticks per second. One `step()` performs:

1. Activate or expire hazards whose boundary equals the current tick.
2. Deliver shipments whose arrival equals the current tick, ordered by shipment ID.
3. Run solar arrays and extractors in industry-ID order.
4. Run foundries in industry-ID order.
5. Dispatch routes in route-ID order.
6. Record canonical events, calculate the digest, and increment the tick.

Hazards are active on the half-open interval `start_tick <= tick < start_tick + duration_ticks`. Resources delivered in phase 2 are available in phases 3 and 4. Resources produced in phases 3 and 4 may dispatch in phase 5. Every shipment arrives at least one tick after dispatch.

Every collection traversal and conflict rule uses typed-ID order. Arithmetic is checked. Overflow, exhausted identity or tick space, and invariant failures return `DeterministicFault` without wrapping, saturating, partial commit, or silent loss.

Orbit fields are authoritative integers. Floating-point positions are derived only in presentation.

Pause means the session does not call `step()`. Speeds `1x`, `4x`, and `20x` are bounded repeated steps. Speed, pause duration, interpolation, camera, frame cadence, backend, and input timestamps never enter canonical bytes or digests. The Workshop clock clamps one frame delta to 250 ms and runs at most 20 steps per rendered frame.

## Immutable History and Branching

Each accepted creator batch creates one immutable content-addressed revision.

A branch ref stores stable branch ID, mutable display name, immutable head, and last authoritative tick. The active view stores selected branch, a view cursor that may precede the head, materialized state, authoritative tick, and digest.

- Undo is available only while paused and no load or save replacement is active.
- Undo moves the view cursor to its parent and rematerializes that prefix through the same tick.
- Undo never applies inverse commands, mutates authority backward, deletes history, or rewrites a checkpoint.
- Redo selects an existing child. Multiple children require explicit selection.
- Browsing an ancestor does not move the original branch head.
- Submitting behind the selected head creates `Branch N` using the smallest unused positive integer and appends a sibling revision there.
- The former branch and redo lineage remain addressable.
- Branch labels, IDs, selection, speed, camera, and checkpoint placement do not enter `StateDigest`.
- Merge, cherry-pick, revision deletion, and destructive compaction are outside V1.

Create a verified derived checkpoint every 600 ticks or 128 accepted revisions, whichever comes first. Checkpoints are excluded from canonical export. Materialization uses the nearest verified ancestor checkpoint, replays selected revisions at their recorded ticks, and advances empty ticks to the requested branch tick. Changing checkpoint cadence must not alter state, digest, or archive bytes.

## Validated Data Packs

A data pack is one UTF-8 JSON file ending in `.nyonpack.json`. Its root contains exactly:

- `kind` equal to `NYON_WORKSHOP_DATA`.
- `format_version` equal to `1`.
- `pack_id`, `pack_version`, and `title`.
- `star_archetypes`, `world_archetypes`, `resources`, `industry_definitions`, and `hazard_definitions`.

Validation happens before construction of `ValidatedCatalogPackV1`:

- Enforce the 1 MiB byte cap before parsing.
- Reject BOMs, invalid UTF-8, trailing values, duplicate keys at every depth, unknown fields, floats, depth over 32, and negative unsigned values.
- `pack_id` matches `[a-z][a-z0-9._-]{0,47}`.
- Catalog IDs match `[a-z][a-z0-9_-]{0,31}`.
- Names are 1 through 64 printable ASCII bytes without leading, trailing, or repeated spaces.
- Descriptions are at most 512 printable ASCII bytes.
- Allow at most 32 stars, 64 worlds, 32 resources, 64 industries, and 32 hazards.
- Require globally unique IDs and resolved same-pack references.
- Quantities are `1..=1_000_000_000`.
- Recipe and hazard durations are `1..=36_000` ticks.
- Processor recipes contain 1 through 8 unique inputs and 1 through 4 unique outputs.
- Only explicit `solar` sources may produce without input.
- Extractors name one output resource and require a linked deposit of that resource.

Packs cannot alter tick rate, capacity limits, phase order, commands, persistence, RulesV1, shader source, or executable behavior. They cannot contain paths, URLs, textures, audio, fonts, shaders, scripts, WASM, JavaScript, model files, dynamic expressions, or downloaded content.

Canonical encoding sorts definitions by ID and emits one minified stable field order. `CatalogHash` is SHA-256 over `b"NYON-WORKSHOP-PACK-V1\0"` plus canonical bytes. Export always uses canonical bytes. Decode, validate, encode, and re-decode preserve the hash. The committed built-in pack passes through this same validator and has a frozen golden hash.

The hash proves integrity and compatibility, not authorship. Unknown versions are rejected rather than migrated.

## Archives and Persistence

The synchronous one-slot `ScenarioStore` remains untouched. Workshop uses an object-safe asynchronous job API:

~~~rust
pub trait WorkshopStore {
    fn start(
        &mut self,
        request: WorkshopStoreRequest,
    ) -> Result<StoreJobId, WorkshopStoreError>;

    fn poll(&mut self, job: StoreJobId) -> StoreJobState;
}
~~~

Requests include listing, creating, loading, committing, renaming, archiving, and selecting save slots plus storing and retrieving canonical packs. `CommitSlot` includes `expected_generation` and performs compare-and-swap.

Client-local store identifiers are transparent integer wrappers: `SlotId(u64)`, `SaveGeneration(u64)`, and `StoreJobId(u64)`. A new store allocates the smallest unused positive slot ID; every successful slot commit increments its generation with checked arithmetic; and each adapter allocates job IDs monotonically for its process lifetime. These identifiers are local persistence metadata and never enter `StateDigest`.

A `.nyonworkshop.json` export contains format identity, rules version, catalog hash, genesis seed, immutable revisions, branch refs, active view, authoritative ticks, final digest, and integrity hashes. Derived checkpoints are excluded.

Load validates the byte limit, canonical structure, catalog, revision hashes, DAG connectivity, operation references, branch refs, tick monotonicity, and final materialized digest in temporary state before replacing the active session.

Store behavior is:

- One in-flight commit and one load/import; dirty commits coalesce.
- Autosave accepted batches and branch operations after a 250 ms debounce, every 600 ticks, and on clean pause or menu exit.
- Browser unload is never recorded as durable success.
- Generation conflict preserves both stored and in-memory state.
- Native data lives under platform application data at `NYON/workshop-v1/`.
- Native commit writes a same-directory temporary generation, flushes and syncs it, atomically replaces the pointer, and syncs the parent directory where supported.
- Preserve the last two valid generations.
- Browser storage uses IndexedDB database `nyon.workshop.v1` with `slots`, `archives`, and `packs` stores.
- Browser archive and ref replacement happens in one read-write transaction.
- Quota denial, abort, schema mismatch, unavailable IndexedDB, conflict, and eviction are recoverable typed failures.
- Corrupt or interrupted load never clears the current valid in-memory session.
- `CONTINUE` selects only the last explicitly selected, unarchived, validated slot and branch.
- A corrupt latest generation opens Recovery and offers the prior valid generation without guessing another slot.
- Slot removal is non-destructive archive, not deletion.
- Logs contain only error codes, byte counts, and short opaque hash prefixes.

## Presentation, Web, and Accessibility

Presentation flow is one-way:

~~~text
authoritative integer state
    -> immutable WorkshopSceneFrame
    -> backend-specific GPU buffers
~~~

No renderer, shader, interpolation, camera value, animation, GPU readback, or UI geometry may mutate authority.

Workshop shaders must run on WebGL2 and therefore use no compute shaders, storage textures, indirect draws, subgroup operations, or authoritative GPU readback.

The web build produces distinct artifacts:

~~~text
dist/webgpu/nyon.js
dist/webgpu/nyon_bg.wasm
dist/webgl/nyon.js
dist/webgl/nyon_bg.wasm
~~~

The loader honors `?backend=webgl2`, otherwise requires both `navigator.gpu` and a successfully requested adapter before loading WebGPU. If WebGPU initialization still fails, it tears down the incomplete surface and retries WebGL2 once. Artifact selection occurs before the final canvas context is acquired. Diagnostics report `WEBGPU`, `WEBGL2 LOW`, `METAL`, `DX12`, `VULKAN`, or `GL`. Failure of both paths produces a recoverable error, not a blank canvas.

Every cross-shader structure uses `#[repr(C)]` plus compile-time size, offset, stride, and alignment assertions. Use `#[repr(C, align(16))]` only when the actual buffer contract requires it; `#[repr(C)]` alone does not promise 16-byte alignment.

Every creator action is available through the hierarchy/outliner and inspector without spatial clicking. Keyboard focus reaches tools, outliner, inspector, timeline, playback, branch chooser, save, and recovery controls in stable order. Modal focus is trapped and restored. Completed jobs and errors announce through a semantic status region. Web uses a DOM semantic mirror; native uses the platform accessibility adapter. Reduced motion and high contrast are required, and no destructive confirmation or status relies only on color.

## Qualification

All source slices run:

~~~text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
git diff --check
~~~

Web work additionally runs both dedicated build scripts and `tools/check-workshop.sh`.

Live qualification covers:

| Host | Native | Browser |
| --- | --- | --- |
| Current macOS | Metal | Safari natural and forced WebGL2; Chrome natural and forced WebGL2 |
| Windows 11 | DX12 | Edge natural and forced WebGL2; Chrome natural and forced WebGL2 |
| Ubuntu 24.04 | Vulkan and forced GL | Firefox natural and forced WebGL2 |

Each row records commit, artifact hash, OS, browser, GPU/driver, requested and selected backend, catalog hash, seed, archive hash, final revision and digest, save/reload, fallback, keyboard-only, network-disabled, and performance results.

Performance acceptance at declared entity caps is:

- Authoritative `step()` p95 at or below 5 ms on every native qualification host.
- Native and WebGPU Two-System Forge p95 frame time at or below 16.7 ms at 1920 by 1080.
- WebGL2 Low p95 frame time at or below 33.3 ms at 1920 by 1080.

Completion labels are non-interchangeable:

1. **Implemented:** source behavior exists and focused plus workspace tests pass.
2. **Artifact-qualified:** native release and both web artifacts build and pass static inspection.
3. **Runtime-qualified:** the live platform matrix and Two-System Forge pass with recorded evidence.
4. **Release-qualified:** runtime, accessibility, performance, recovery, offline, documentation, provenance, and clean intended diff all pass.

Unavailable Windows or Linux hosts leave those rows pending and prevent release-qualified status.
