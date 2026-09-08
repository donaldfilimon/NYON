# NYON Galaxy Workshop Client Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate the pure Workshop core into a crash-safe, accessible native and browser client with real creator tools, immutable-history navigation, procedural presentation, and explicit WebGPU/WebGL2 artifacts.

**Architecture:** `ClientRuntime` owns sibling Classic and Workshop sessions without genericizing `AppCore`. A typed action mailbox is the only UI-to-Workshop mutation path; asynchronous store adapters persist canonical archives; presentation extracts immutable scene frames; the browser selects a separately built backend artifact before graphics initialization.

**Tech Stack:** Rust nightly-2026-09-01, winit 0.30.12, wgpu 30.0.1, native files, IndexedDB, DOM accessibility mirror, WebGPU, and WebGL2.

**Spec:** `docs/superpowers/specs/2026-09-02-nyon-v2-design.md`

## Checklist status (recorded 2026-09-08)

**0 of 39 boxes are checked and that does not mean nothing landed.** Every file
this plan names exists and is substantial — `src/workshop/session.rs`,
`src/workshop/store.rs`, `src/workshop/store/web.rs`, `src/app/client_runtime.rs`,
`src/presentation/workshop.rs`, `src/engine/backend.rs`, `web/loader.js`, and the
three `tools/` scripts — landed through `fc38672` and refactored by `1c32f26`,
`52b8844`, `e0b5235`. What is *not* closed is tracked in
`docs/superpowers/reviews/2026-09-04-workshop-v1-baseline-review.md`, whose
findings 2, 5, 7, 8 and 9 are still `Status: Open`. Read that review, not these
boxes.

## Global Constraints

- Complete the core plan before client integration.
- Treat core state, revisions, archives, and digests as immutable API inputs and outputs.
- Do not make renderer, UI, filesystem, IndexedDB, camera, animation, or backend data authoritative.
- Do not modify RulesV1 `AppCore`, `FixedClock`, `ScenarioStore`, or `PreferencesStore` behavior.
- Do not add empty multiplayer, account, social, voice, ranked, AI, mobile, or cloud screens.
- Keep save failures recoverable and retain the last valid in-memory session and stored generation.
- Make every action available through non-spatial semantic controls.
- Keep source, bundle, live runtime, accessibility, performance, and manual evidence distinct.

---

### Task 1: Implement Workshop session pacing and the typed action mailbox

**Files:**
- Create: `src/workshop/session.rs`
- Create: `src/workshop/mod.rs`
- Create: `tests/workshop_session.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `WorkshopHistoryV1`, pure creator batches, and frame-duration inputs.
- Produces: `WorkshopSession`, `WorkshopSpeed`, `WorkshopAction`, `WorkshopClock`, and immutable `WorkshopSessionSnapshot`.

- [ ] **Step 1: Write failing clock and action-order tests**

Test that:

- Pause performs zero `step()` calls.
- Step performs exactly one authoritative tick.
- `1x`, `4x`, and `20x` call the same core `step()` in order.
- A frame delta above 250 ms is clamped.
- One frame runs at most 20 steps.
- Creator actions are drained before simulation steps.
- Store completions and presentation extraction happen after authority work.
- Speed, wall-time grouping, and frame interpolation do not change the resulting digest.

- [ ] **Step 2: Define the client-only action API**

~~~rust
pub enum WorkshopSpeed {
    Paused,
    One,
    Four,
    Twenty,
}

pub enum WorkshopAction {
    Submit(CreatorBatchV1),
    Pause,
    Resume(WorkshopSpeed),
    StepOnce,
    Undo,
    Redo(RevisionId),
    SelectBranch(BranchId),
    RequestSave,
    RequestLoad(SlotId),
    RequestExport,
    RequestImport(Box<[u8]>),
}
~~~

`WorkshopSession::enqueue` is the only public mutation request path used by views.

- [ ] **Step 3: Implement deterministic event-loop ordering**

`WorkshopSession::update(frame_delta)` drains actions in FIFO order, submits creator batches, executes bounded core steps, polls existing store jobs, and then publishes one immutable snapshot. It retains at most 100 typed rejection diagnostics.

- [ ] **Step 4: Run focused and full gates**

~~~bash
cargo test --test workshop_session
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
git diff --check
~~~

- [ ] **Step 5: Commit the session boundary**

~~~bash
git add src/workshop src/lib.rs tests/workshop_session.rs
git commit -m "feat(workshop): add the client session boundary"
~~~

### Task 2: Implement memory, native, and IndexedDB stores

**Files:**
- Create: `src/workshop/store.rs`
- Create: `src/workshop/store/memory.rs`
- Create: `src/workshop/store/native.rs`
- Create: `src/workshop/store/web.rs`
- Create: `tests/workshop_store.rs`
- Modify: `src/workshop/session.rs`
- Modify: target dependencies in `Cargo.toml` only where IndexedDB bindings require them

**Interfaces:**
- Consumes: canonical pack and Workshop archive bytes.
- Produces: object-safe `WorkshopStore`, request/job/result types, generation compare-and-swap, slot metadata, and recovery metadata.

- [ ] **Step 1: Define the store contract in a failing conformance suite**

~~~rust
pub trait WorkshopStore {
    fn start(
        &mut self,
        request: WorkshopStoreRequest,
    ) -> Result<StoreJobId, WorkshopStoreError>;

    fn poll(&mut self, job: StoreJobId) -> StoreJobState;
}

pub struct StoreJobId(pub u64);
pub struct SlotId(pub u64);
pub struct SaveGeneration(pub u64);
pub struct SlotName(Box<str>);

pub struct SlotSummary {
    pub id: SlotId,
    pub name: SlotName,
    pub generation: SaveGeneration,
    pub archived: bool,
    pub selected_for_continue: bool,
}

pub enum StoreIoCode {
    Open,
    Read,
    Write,
    Flush,
    Sync,
    Replace,
}

pub enum WorkshopStoreError {
    Busy,
    UnknownJob(StoreJobId),
    UnknownSlot(SlotId),
    InvalidSlotName,
    Conflict { expected: SaveGeneration, actual: SaveGeneration },
    Unavailable,
    QuotaExceeded,
    SchemaMismatch,
    CorruptGeneration {
        slot: SlotId,
        generation: SaveGeneration,
        recovery: Option<SaveGeneration>,
    },
    InvalidArchive { code: &'static str },
    Io { code: StoreIoCode },
}

pub enum StoreJobState {
    Pending,
    Complete(Result<WorkshopStoreResult, WorkshopStoreError>),
}

pub enum WorkshopStoreResult {
    Slots(Vec<SlotSummary>),
    Created { slot: SlotId, generation: SaveGeneration },
    Loaded { slot: SlotId, generation: SaveGeneration, archive: Box<[u8]> },
    Committed { slot: SlotId, generation: SaveGeneration },
    Renamed { slot: SlotId },
    Archived { slot: SlotId },
    ContinueSelected { slot: SlotId },
    PackStored { hash: CatalogHash },
    Pack(Box<[u8]>),
    Packs(Vec<CatalogHash>),
}

pub enum WorkshopStoreRequest {
    ListSlots,
    CreateSlot { name: SlotName, archive: Box<[u8]> },
    LoadSlot { slot: SlotId },
    CommitSlot {
        slot: SlotId,
        expected_generation: SaveGeneration,
        archive: Box<[u8]>,
    },
    RenameSlot { slot: SlotId, name: SlotName },
    ArchiveSlot { slot: SlotId },
    SelectContinue { slot: SlotId },
    PutPack { canonical_pack: Box<[u8]> },
    GetPack { hash: CatalogHash },
    ListPacks,
}
~~~

`SlotName` accepts 1 through 64 printable ASCII bytes without leading, trailing, or repeated spaces. Adapters translate platform errors to the bounded codes above and retain detailed operating-system text only in redacted local diagnostics, never in serialized store state or imported-content logs.

Run the same behavioral assertions against memory, fault-injected native, and fault-injected IndexedDB implementations.

- [ ] **Step 2: Test generation and failure semantics before implementation**

Cover:

- Commit with the current generation succeeds and returns the next generation.
- Stale generation returns `Conflict` and changes neither side.
- Crash before pointer replacement preserves the former complete generation.
- Crash after replacement selects the new complete generation.
- IndexedDB transaction abort preserves the former transaction.
- Quota denial, unavailable database, schema mismatch, and eviction remain recoverable.
- Corrupt latest generation offers the prior valid generation.
- Invalid imported archive never replaces the in-memory Workshop.
- Slot rename and archive are non-destructive.
- `CONTINUE` is absent without one explicitly selected valid unarchived slot.

- [ ] **Step 3: Implement the synchronous memory reference adapter**

The memory adapter completes jobs only through `poll`, even when the result is immediately available. Use it as the behavioral oracle for slot ordering, generation increments, selected-continue semantics, pack hashing, and archived-slot filtering.

- [ ] **Step 4: Implement native crash-safe generations**

Use the platform application-data directory `NYON/workshop-v1/`. For each commit:

1. Validate archive bytes through the pure decoder.
2. Write a same-directory temporary generation.
3. Flush and `sync_all` the file.
4. Read it back and verify archive hash.
5. Atomically replace the generation pointer.
6. Sync the parent directory where supported.
7. Retain the newest two complete generations.

Never delete the former valid generation before the new pointer is durable.

- [ ] **Step 5: Implement IndexedDB transactions**

Open database `nyon.workshop.v1` with `slots`, `archives`, and `packs` stores. Store archive bytes under immutable generation keys; update archive and slot ref in one read-write transaction. Do not report browser unload as a completed save.

- [ ] **Step 6: Integrate autosave coalescing**

Allow one commit and one load/import operation in flight. Mark dirty on accepted batches and branch changes. Debounce 250 ms, commit every 600 ticks, and commit on clean pause or menu exit. A new dirty event during a commit schedules one following commit using the returned generation.

- [ ] **Step 7: Run focused, native, wasm, and full gates**

~~~bash
cargo test --test workshop_store
cargo test --workspace --all-targets
cargo check --target wasm32-unknown-unknown --lib
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
git diff --check
~~~

- [ ] **Step 8: Commit persistence**

~~~bash
git add Cargo.toml Cargo.lock src/workshop tests/workshop_store.rs
git commit -m "feat(storage): add crash-safe workshop saves"
~~~

### Task 3: Add ClientRuntime, menu, Workshop tools, and recovery

**Files:**
- Create: `src/app/client_runtime.rs`
- Create: `src/ui/workshop.rs`
- Create: `tests/workshop_client.rs`
- Modify: `src/app.rs` or its current lifecycle module only at the wrapper boundary
- Modify: `src/ui/mod.rs`

**Interfaces:**
- Consumes: current Classic `AppCore`, `WorkshopSession`, `WorkshopStore`, and immutable session snapshots.
- Produces: `ClientRuntime`, `ClientScreen`, `ActiveSession`, menu capability state, typed focus/actions, and Recovery presentation.

- [ ] **Step 1: Write failing shell-state tests**

Assert:

- Startup enters `MainMenu` without constructing online or Workshop authority.
- `CLASSIC SECTOR` uses the current `AppCore` and frozen RulesV1 output.
- `CONTINUE` is enabled only for the selected validated slot.
- Corrupt latest generation enters `RecoverableError` and offers only the prior generation or return-to-menu.
- No view owns a mutable reference to core authority.
- Pointer and keyboard activation of the same control enqueue identical `WorkshopAction` values.
- Undo/redo is disabled while running or while replacement load/save work is active.
- Submitting behind a head creates and selects the new branch returned by the session.

- [ ] **Step 2: Define the runtime shell**

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

`ClientRuntime` owns the current `AppCore` only on the Classic path. Do not add Workshop variants to the existing RulesV1 `AppMode`.

- [ ] **Step 3: Build only capability-backed menu routes**

Add `NEW WORKSHOP`, `CONTINUE`, `CLASSIC SECTOR`, `SETTINGS`, `CREDITS`, and native `QUIT`. Do not create future screens.

- [ ] **Step 4: Implement Workshop semantic tools**

Provide:

- Tool palette for every creator operation.
- Hierarchy/outliner grouped by system and entity kind.
- Inspector with field validation and affected-dependency lists.
- Timeline with tick, speed, pause, step, undo, redo, and child selection.
- Branch chooser with immutable head and active-cursor distinction.
- Save state, generation, dirty, conflict, and recovery indicators.
- Production, inventory, route, shipment, deposit, and hazard status.
- Backend and catalog diagnostics.

Removal confirmation builds an explicit ordered batch after showing all blockers. It never asks the core to cascade.

- [ ] **Step 5: Add keyboard/pointer parity tests**

Drive the Two-System Forge creator sequence through action-level pointer and keyboard inputs and compare every queued batch byte-for-byte. Verify focus remains on a meaningful control after apply, cancel, modal close, branch switch, save completion, and recovery.

- [ ] **Step 6: Run focused and full gates**

~~~bash
cargo test --test workshop_client
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
git diff --check
~~~

- [ ] **Step 7: Commit the product shell**

~~~bash
git add src/app src/ui src/workshop tests/workshop_client.rs
git commit -m "feat(shell): add the Galaxy Workshop experience"
~~~

### Task 4: Add immutable Workshop scene extraction and backend diagnostics

**Files:**
- Create: `src/presentation/workshop.rs`
- Create: `src/engine/backend.rs`
- Create: `tests/workshop_presentation.rs`
- Modify: `src/presentation/mod.rs`
- Modify: current renderer only to consume `WorkshopSceneFrame`

**Interfaces:**
- Consumes: `WorkshopSessionSnapshot` and selected `BackendKind`.
- Produces: stable `WorkshopSceneFrame`, GPU instance arrays, and a display-only backend label.

- [ ] **Step 1: Write failing one-way extraction tests**

Clone canonical authority bytes before and after extraction and assert equality. Build equivalent state in different map insertion orders and require equal scene ordering. Assert backend selection, camera, interpolation, reduced motion, and high contrast do not change the core digest.

- [ ] **Step 2: Define derived frame types**

`WorkshopSceneFrame` contains sorted systems, stars, worlds, lanes, shipments, industry status, selection decorations, labels, and semantic object bounds. It owns display floats derived from integer positions and tick but contains no mutable core handle.

- [ ] **Step 3: Define backend diagnostics**

~~~rust
pub enum BackendKind {
    Metal,
    Dx12,
    Vulkan,
    Gl,
    WebGpu,
    WebGl2Low,
}
~~~

Map the actual selected wgpu backend and browser artifact to one visible label. Never infer a backend from requested configuration alone.

- [ ] **Step 4: Lock GPU layouts**

Every new vertex, instance, and uniform structure uses `#[repr(C)]`. Add compile-time size, offset, stride, and alignment assertions matching WGSL. Add `align(16)` only to structures whose actual buffer contract requires it.

- [ ] **Step 5: Keep the Workshop shader subset WebGL2-compatible**

Use no compute, storage textures, indirect draws, subgroups, or GPU-authoritative readback. WebGL2 Low disables unsupported bloom and advisory effects without changing authoritative output.

- [ ] **Step 6: Run presentation, shader, and full gates**

~~~bash
cargo test --test workshop_presentation
cargo test --test shaders
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
git diff --check
~~~

- [ ] **Step 7: Commit presentation extraction**

~~~bash
git add src/presentation src/engine tests/workshop_presentation.rs
git commit -m "feat(render): add Workshop scene extraction"
~~~

### Task 5: Produce separate WebGPU and WebGL2 artifacts

**Files:**
- Create: `tools/build-web-webgpu.sh`
- Create: `tools/build-web-webgl.sh`
- Create: `tools/check-workshop.sh`
- Create: `web/loader.js`
- Modify: `web/index.html`
- Modify: `Cargo.toml` feature declarations
- Create: `tests/workshop_web.rs`

**Interfaces:**
- Consumes: one shared client source with mutually selected wgpu web features.
- Produces: exact `dist/webgpu` and `dist/webgl` artifacts plus preflight/fallback loader behavior.

- [ ] **Step 1: Write failing artifact and loader tests**

Assert exact output names:

~~~text
dist/webgpu/nyon.js
dist/webgpu/nyon_bg.wasm
dist/webgl/nyon.js
dist/webgl/nyon_bg.wasm
~~~

In loader tests, stub `navigator.gpu` and adapter requests. Prove:

- `?backend=webgl2` loads only WebGL2.
- Missing `navigator.gpu` loads WebGL2.
- Adapter request returning null loads WebGL2.
- Successful adapter preflight loads WebGPU.
- WebGPU initialization rejection tears down and retries WebGL2 once.
- Failure of both artifacts renders a recoverable error.
- Final canvas context acquisition happens after artifact choice.

- [ ] **Step 2: Add mutually selected web features**

Configure the WebGPU script to compile wgpu with its `webgpu` path and WebGL2 script with `webgl`. Each script uses the repository-pinned target and wasm-bindgen CLI, clears only its own output directory, and fails when expected JS/WASM files are absent or empty.

- [ ] **Step 3: Implement the loader state machine**

The loader records requested backend, adapter-preflight result, loaded artifact, initialization result, retry result, and selected backend. It exposes only opaque error codes and browser capability facts, not imported Workshop data.

- [ ] **Step 4: Add static Workshop checks**

`tools/check-workshop.sh` builds both artifacts, scans the WebGL2 Workshop shader path for forbidden capabilities, verifies backend labels, and runs `cargo check --target wasm32-unknown-unknown --lib`.

- [ ] **Step 5: Run web and full gates**

~~~bash
cargo test --test workshop_web
./tools/build-web-webgpu.sh
./tools/build-web-webgl.sh
./tools/check-workshop.sh
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
git diff --check
~~~

- [ ] **Step 6: Commit explicit fallback**

~~~bash
git add Cargo.toml Cargo.lock web tools tests/workshop_web.rs
git commit -m "feat(web): add explicit WebGL2 fallback"
~~~

### Task 6: Implement semantic accessibility adapters

**Files:**
- Create: `src/ui/accessibility.rs`
- Create: `src/platform/web_accessibility.rs`
- Create: `src/platform/native_accessibility.rs`
- Create: `tests/workshop_accessibility.rs`
- Modify: `src/ui/workshop.rs`
- Modify: current platform module declarations

**Interfaces:**
- Consumes: immutable Workshop UI semantic tree and focus/action events.
- Produces: DOM semantic mirror on web, native accessibility nodes, status announcements, modal focus handling, reduced-motion and high-contrast behavior.

- [ ] **Step 1: Write failing semantic-tree tests**

Require stable roles, names, values, descriptions, enabled state, selected state, and action IDs for every palette, outliner, inspector, timeline, branch, save, and recovery control. Assert every `CreatorOpV1` is reachable without canvas coordinates.

- [ ] **Step 2: Test focus and announcement behavior**

Cover forward/backward traversal, modal trap, restored invoker focus, disabled undo/redo, branch-child chooser, save conflict, completed save, import validation, and recovery. Announcements use error codes and field paths without echoing raw imported content.

- [ ] **Step 3: Implement one platform-neutral semantic tree**

Derive semantic nodes from the same Workshop UI model that drives pointer rendering. Platform adapters translate those nodes and send actions back through the typed mailbox. They do not implement separate business logic.

- [ ] **Step 4: Implement web and native adapters**

Web creates a DOM mirror associated with the canvas and an `aria-live` status region. Native exposes equivalent roles, focus, values, and actions through the repository's selected accessibility integration. Both honor reduced motion and high contrast; status and destructive confirmation never rely on color alone.

- [ ] **Step 5: Run accessibility and full gates**

~~~bash
cargo test --test workshop_accessibility
cargo test --workspace --all-targets
cargo check --target wasm32-unknown-unknown --lib
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
git diff --check
~~~

- [ ] **Step 6: Commit semantic access**

~~~bash
git add Cargo.toml Cargo.lock src/ui src/platform tests/workshop_accessibility.rs
git commit -m "feat(accessibility): expose Workshop semantic controls"
~~~

## Client Completion

The client plan is complete when store conformance and failure injection pass, no UI path holds mutable core state, Two-System Forge is possible by pointer and keyboard, renderer extraction is one-way, both browser artifacts build under distinct feature paths, every action has equivalent semantic access, and RulesV1 remains unchanged.
