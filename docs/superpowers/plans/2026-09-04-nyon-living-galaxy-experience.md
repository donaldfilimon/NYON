# NYON Living Galaxy Experience and Qualification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the Living Galaxy V2 authority into a complete offline product experience: dependable session control, cross-authority Library/Continue, validated starters, unrestricted creator tooling, truthful inspection and Chronicle, same-tick history experiments, portable transfer, isolated presentation preferences, and release-grade qualification.

**Architecture:** A `LivingClientSessionV2` owns one authority/history view and exposes immutable `LivingClientSnapshotV2` frames plus typed intents. UI controllers consume authority state, receipts, reasons, and foundation-owned layout/diorama contracts; they never infer simulation outcomes or mutate canonical state. A version-neutral `LibraryClientV1` coordinates V1/V2 selection through bounded jobs. Native and browser shells provide transfer, audio, and persistence capabilities behind deterministic controller traits whose state is excluded from authority digests.

**Tech Stack:** Rust nightly-2026-09-01, edition 2024, existing winit/wgpu/WebGL2 application shell, AccessKit and browser semantics, bundled Inter SDF assets, serde/serde_json, native filesystem dialogs, browser File/Blob/download APIs, `rfd = 0.15.4`, `rodio = 0.21.1`, and Playwright `1.55.0` for browser acceptance.

**Spec:** `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-design.md`, `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md`, `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-experience.md`, and `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-qualification.md`.

## Global Constraints and Cross-Plan Handoffs

- Authority and civilization plans land first. This plan consumes `LivingGalaxyAuthorityV2`, `LivingHistoryV2`, bounded replay/archive jobs, canonical state, receipts, recorded reasons, and the four validated strategic fixtures. It never creates a second simulation model.
- Foundation lands before visual integration. This plan consumes `WorkshopLayout`, `VisibleWindow`, `WorkshopViewState`, `PlatformUiFrame`, `build_platform_ui_batch`, `DioramaFrame`, and the already-defined `RenderScene::Diorama(&DioramaFrame)` variant.
- `src/engine/render_frame.rs`, `RenderScene`, and the renderer scene enum remain entirely foundation-owned. No task in this plan modifies or shadows them.
- The Library coordinator uses neutral `LibraryBranchIdV1([u8; 16])`, `LibraryRevisionIdV1([u8; 32])`, and `LibraryDigestV1([u8; 32])` wrappers. Conversions to Living or Workshop identifiers are explicit, checked, and adapter-local.
- `LibraryCoordinatorV1` is job-shaped and owner-epoch-scoped. Loading, compare-and-swap, replay, store mutation, import, and export never block an event loop or publish partial state.
- A cleared Library selection is a new valid generation with authority, slot, branch, revision, tick, and digest all explicitly absent. No recency fallback chooses between V1 and V2.
- Creator operations are free but structurally validated and atomic. Autonomous actions pay catalog costs. Every visible narrative distinguishes creator, civilization, natural, and scheduled provenance.
- Presentation, camera, wall clock, render cadence, audio, focus, semantic state, preferences, and graphics backend are excluded from authority bytes and digests.
- Preserve Classic RulesV1 and WorkshopV1 bytes, fixtures, paths, browser keys, save limits, and behavior. A legacy session is adapted, not silently converted.
- Before Task 1, establish the explicitly accepted authority/civilization/foundation baseline in the isolated workspace. Every task stages only its listed files and runs `git diff --check`.
- Dependency additions require license/source-size/native-web compatibility review before editing manifests. If `rfd`, `rodio`, or Playwright cannot satisfy the offline/platform matrix, stop that task with evidence instead of weakening acceptance.

## Frozen Experience Interfaces

```rust
pub struct LivingClientSnapshotV2 {
    pub owner_epoch: LivingOwnerEpochV2,
    pub mode: LivingClientModeV2,
    pub cursor: LivingCommandCursorV2,
    pub selected: Option<LivingEntityIdV2>,
    pub focused: LivingFocusV2,
    pub save: LivingSaveStatusV2,
    pub replay: Option<LivingReplayProgressV2>,
}

pub enum LivingClientModeV2 {
    Paused,
    Running { speed: LivingSpeedV2 },
    ViewingHistory { branch: LivingBranchIdV2, tick: LivingTickV2 },
    Recovering,
    Faulted,
}

pub enum LivingSpeedV2 { One, Four, Twenty }

pub trait LivingClientHostV2 {
    fn now(&self) -> HostInstantV1;
    fn request_redraw(&mut self);
    fn announce(&mut self, announcement: LivingAnnouncementV2);
}
```

Library integration consumes the authority plan's exact contract:

```rust
pub struct LibraryBranchIdV1(pub [u8; 16]);
pub struct LibraryRevisionIdV1(pub [u8; 32]);
pub struct LibraryDigestV1(pub [u8; 32]);

pub trait LibraryCoordinatorV1 {
    fn begin(&mut self, owner: LibraryOwnerEpochV1, request: LibraryCoordinatorRequestV1)
        -> Result<LibraryCoordinatorJobIdV1, LibraryCoordinatorErrorV1>;
    fn poll(&mut self, owner: LibraryOwnerEpochV1, job: LibraryCoordinatorJobIdV1)
        -> Result<LibraryCoordinatorPollV1, LibraryCoordinatorErrorV1>;
    fn cancel(&mut self, owner: LibraryOwnerEpochV1, job: LibraryCoordinatorJobIdV1)
        -> Result<LibraryCoordinatorCancelV1, LibraryCoordinatorErrorV1>;
}
```

---

### Task 1: Implement session pacing, holds, and owner-epoch safety

**Files:**
- Create: `src/living/client/mod.rs`
- Create: `src/living/client/session.rs`
- Create: `src/living/client/pacing.rs`
- Modify: `src/living/mod.rs`
- Test: `tests/living_client_session.rs`

**Interfaces:**
- Consumes: `LivingHistoryV2`, `LivingCommandCursorV2`, immutable authoritative receipts, and host elapsed time.
- Produces: `LivingClientSessionV2`, `LivingClientSnapshotV2`, `LivingSpeedV2`, `LivingHoldTokenV2`, and bounded `update` outcomes.

- [ ] **Step 1: Write failing pacing and continuity tests**

```rust
#[test]
fn pacing_never_skips_authoritative_ticks() {
    let mut session = running_session(first_expansion_fixture(), LivingSpeedV2::Twenty);
    session.update(Duration::from_millis(117)).unwrap();
    assert_eq!(session.snapshot().cursor.tick, LivingTickV2(23));
    assert_eq!(session.pending_simulation_debt(), Duration::from_millis(2));
}

#[test]
fn nested_holds_pause_without_destroying_the_requested_speed() {
    let mut session = running_session(blank_fixture(), LivingSpeedV2::Four);
    let import = session.acquire_hold(LivingHoldReasonV2::Import);
    let history = session.acquire_hold(LivingHoldReasonV2::History);
    assert_eq!(session.effective_mode(), LivingClientModeV2::Paused);
    session.release_hold(import).unwrap();
    assert_eq!(session.effective_mode(), LivingClientModeV2::Paused);
    session.release_hold(history).unwrap();
    assert_eq!(session.effective_mode(), LivingClientModeV2::Running { speed: LivingSpeedV2::Four });
}
```

Add zero/one/many-civilization continuity, pause, single-step, exact 1x/4x/20x accumulation, bounded catch-up, focus-loss behavior, rejected foreign hold token, fault atomicity, and replaced-owner tests.

- [ ] **Step 2: Run the focused test and record missing client APIs**

```bash
cargo test --test living_client_session -- --nocapture
```

Expected: unresolved `nyon::living::client` and fixture adapter symbols.

- [ ] **Step 3: Implement the session state machine**

Use integer nanosecond accumulation against `LIVING_TICK_HZ`; never derive authority progress from rendered frames. Limit one UI update to 1,024 authority work units and retain exact debt. `Pause`, `Run(speed)`, `Step`, `AcquireHold`, and `ReleaseHold` are presentation intents. A fault preserves the last valid authority snapshot, changes mode to `Faulted`, and exposes typed recovery actions.

Owner epoch changes on new/open/import/Continue and invalidates outstanding holds, store jobs, replay jobs, transfer jobs, confirmation drafts, and stale UI intents. No job may publish into a different epoch.

- [ ] **Step 4: Run focused and authority regressions**

```bash
cargo test --test living_client_session
cargo test -p nyon-workshop-core --test living_queue
cargo test -p nyon-workshop-core --test living_replay
cargo fmt --all --check
git diff --check
```

- [ ] **Step 5: Commit the client session boundary**

```bash
git add src/living/client/mod.rs src/living/client/session.rs src/living/client/pacing.rs src/living/mod.rs tests/living_client_session.rs
git commit -m "feat(living): add continuous client session pacing"
```

### Task 2: Build the client runtime and exact global Continue flow

**Files:**
- Create: `src/living/client/runtime.rs`
- Create: `src/library/client.rs`
- Modify: `src/library/mod.rs`
- Modify: `src/app/client_runtime.rs`
- Modify: `src/app/core/session.rs`
- Test: `tests/living_client_runtime.rs`
- Test: `tests/library_continue.rs`

**Interfaces:**
- Consumes: job-shaped `LibraryCoordinatorV1`, Living/Workshop stores, authority-specific checked identifier adapters, and Task 1 sessions.
- Produces: `ClientRuntimeV2`, `LibraryClientV1`, `ContinueResolutionV1`, explicit recovery state, and owner-scoped operation polling.

- [ ] **Step 1: Write failing cross-authority Continue tests**

```rust
#[test]
fn continue_opens_only_the_coordinator_target() {
    let mut fixture = library_with_selected_workshop_and_newer_living();
    fixture.coordinator_selects(LibraryAuthorityV1::WorkshopV1, 3, workshop_cursor());
    let opened = fixture.client.continue_last().complete().unwrap();
    assert_eq!(opened.authority(), LibraryAuthorityV1::WorkshopV1);
    assert_eq!(opened.slot_id(), 3);
}

#[test]
fn invalid_continue_target_opens_recovery_without_fallback() {
    let mut fixture = library_with_missing_selected_living_slot();
    let result = fixture.client.continue_last().complete().unwrap();
    assert!(matches!(result, ContinueResolutionV1::Recovery { recorded_authority: LibraryAuthorityV1::LivingV2, .. }));
    assert!(!fixture.any_session_opened());
}
```

Add absent coordinator first-launch Workshop migration, cleared selection, corrupt A/B generations, missing/archived slot, missing catalog, branch/revision/tick/digest mismatch, owner replacement, cancellation, stale CAS, V2 selected while V1 is locally newer, and no valid selection cases.

- [ ] **Step 2: Run the focused tests**

```bash
cargo test --test library_continue -- --nocapture
cargo test --test living_client_runtime -- --nocapture
```

- [ ] **Step 3: Implement an explicit runtime transition graph**

```rust
pub enum ClientRuntimeStateV2 {
    Shell,
    ResolvingContinue { job: LibraryCoordinatorJobIdV1 },
    Loading { authority: LibraryAuthorityV1, operation: LibraryLoadOperationV1 },
    Living(LivingClientSessionV2),
    Workshop(WorkshopClientSessionV1),
    Recovery(LibraryRecoveryV1),
}
```

`LibraryClientV1` polls one coordinator/store/replay stage at a time. It converts neutral IDs only at the selected adapter, verifies the loaded cursor/digest before replacing Shell, and leaves a current session intact on failed open. A successful new/open/import first durably saves the authority target, then compare-and-swaps the global coordinator; if that CAS fails, the recoverable slot remains visible but is not silently made Continue.

- [ ] **Step 4: Run coordinator, runtime, Workshop, and Classic regressions**

```bash
cargo test --test living_client_runtime
cargo test --test library_continue
cargo test --test library_selection
cargo test --test workshop_client
cargo test --test campaign
cargo fmt --all --check
git diff --check
```

- [ ] **Step 5: Commit runtime integration**

```bash
git add src/living/client/runtime.rs src/library/client.rs src/library/mod.rs src/app/client_runtime.rs src/app/core/session.rs tests/living_client_runtime.rs tests/library_continue.rs
git commit -m "feat(library): resolve one global Continue target"
```

### Task 3: Implement Library lifecycle, durable save status, and deliberate reclamation

**Files:**
- Create: `src/library/view_model.rs`
- Create: `src/living/client/save.rs`
- Modify: `src/library/client.rs`
- Modify: `src/living/client/mod.rs`
- Modify: `src/living/client/runtime.rs`
- Test: `tests/library_lifecycle.rs`
- Test: `tests/living_save_client.rs`

**Interfaces:**
- Consumes: Living/Workshop store descriptors and jobs plus coordinator CAS.
- Produces: version-labeled Library rows, `LivingSaveStatusV2`, typed lifecycle previews, confirmation state, and recovery actions.

- [ ] **Step 1: Write failing save and reclamation tests**

```rust
#[test]
fn saved_is_reported_only_after_store_commit_wins() {
    let mut fixture = save_fixture_with_three_poll_commit();
    fixture.client.request_save().unwrap();
    assert_eq!(fixture.client.save_status(), LivingSaveStatusV2::Saving);
    fixture.client.poll_once().unwrap();
    assert_eq!(fixture.client.save_status(), LivingSaveStatusV2::Saving);
    fixture.complete_save();
    assert!(matches!(fixture.client.save_status(), LivingSaveStatusV2::SavedLocally { generation: 2 }));
}

#[test]
fn archive_does_not_free_slot_capacity_but_confirmed_delete_does() {
    let mut fixture = full_library_fixture();
    fixture.archive_slot(7).complete().unwrap();
    assert_eq!(fixture.create(), Err(LibraryClientErrorV1::SlotCapacity));
    fixture.delete_slot(7, typed_name("Vale Archive")).complete().unwrap();
    assert!(fixture.create().is_ok());
}
```

Cover `UnsavedChanges`, `Saving`, `SavedLocally`, `SaveFailed { code }`; rename; archive/unarchive; permanent slot deletion; pack archive/delete blockers; archived references; stale generations; selected-target coordinator-clear failure; slot-delete failure after clear; typed-name mismatch; active-session protection; exactly one reclaimed entry; and no cascade.

- [ ] **Step 2: Implement view models and ordered mutation workflows**

Library rows expose authority/version, slot, lifecycle, catalog identity, byte size, generation, last verified cursor, corruption state, and Continue marker. Permanent deletion is available only for archived, nonactive items. Its preview names every dependent pack/galaxy, Continue effect, irreversibility, and export recommendation.

For a selected target: compare-and-swap the coordinator to a cleared record, verify success, then delete the authority-local slot. If deletion fails, retain the slot and offer explicit reselection. Pack deletion requires archived and unreferenced across every retained ordinary or archived galaxy.

- [ ] **Step 3: Run lifecycle/store contracts**

```bash
cargo test --test library_lifecycle
cargo test --test living_save_client
cargo test --test living_store
cargo test --test workshop_store
cargo fmt --all --check
git diff --check
```

- [ ] **Step 4: Commit Library lifecycle behavior**

```bash
git add src/library/view_model.rs src/library/client.rs src/living/client/save.rs src/living/client/mod.rs src/living/client/runtime.rs tests/library_lifecycle.rs tests/living_save_client.rs
git commit -m "feat(library): manage durable galaxy lifecycle"
```

### Task 4: Add validated starters, scenario branches, guide, and Experiments

**Files:**
- Create: `assets/living/starters/vale-confluence-v2.json`
- Create: `assets/living/starters/blank-galaxy-v2.json`
- Create: `assets/living/starters/seeded-default-v2.json`
- Create: `src/living/content.rs`
- Create: `src/living/client/experiments.rs`
- Create: `src/ui/living_start.rs`
- Modify: `src/living/mod.rs`
- Modify: `src/app/onboarding.rs`
- Test: `tests/living_content.rs`
- Test: `tests/living_experiments.rs`

**Interfaces:**
- Consumes: authority pack/genesis validators and civilization-owned Commerce, Frontier Friction, Interrupted Corridor, and First Expansion fixtures.
- Produces: `LivingStarterCatalogV2`, starter cards, deterministic seed options, three-item guide, `ExperimentEvidenceV2`, and hideable notebook state.

- [ ] **Step 1: Write failing content and evidence tests**

```rust
#[test]
fn vale_confluence_is_validated_content_not_a_privileged_code_path() {
    let starter = LivingStarterCatalogV2::bundled().find("vale-confluence").unwrap();
    let genesis = starter.decode_and_validate(core_pack()).unwrap();
    assert_eq!((genesis.systems().len(), genesis.worlds().len(), genesis.civilizations().len()), (4, 8, 3));
    assert_eq!(genesis.direct_lanes().len(), 6);
}

#[test]
fn experiment_completion_requires_a_supporting_receipt() {
    let mut notebook = ExperimentNotebookV2::default();
    notebook.observe(bookmark_without_matching_receipt());
    assert!(!notebook.entry(ExperimentIdV2::CorridorDisruption).is_complete());
    notebook.observe(bookmark_with_matching_receipt());
    assert!(notebook.entry(ExperimentIdV2::CorridorDisruption).evidence().is_some());
}
```

Assert Vale names/policies/homes, six lanes and unsettled worlds; Blank contains no actors/entities; Seeded defaults to 12 systems/24 worlds/four civilizations with reproducible options; fixture witness digests match civilization ownership; no starter grants privileged resources at runtime; no guide lock; and all seven experiment evidence links contain branch/revision/tick/digest/receipt.

- [ ] **Step 2: Implement one validated content registry**

Starter assets carry kind/version/catalog hash/seed/generator provenance/genesis and optional validated fixture branches. `LivingStarterCatalogV2::decode_bundled` uses the same validators as imported content. The Main Vale branch is unscripted; fixture branches embed their declared initial manifests and witness records rather than injecting client events.

The three-item guide introduces selection, time, and creation, persists only presentation completion, is dismissible immediately, and never gates commands. Experiments observe real receipts and bookmarks, grant no resources, impose no timer, and may be hidden.

- [ ] **Step 3: Run content, fixture, and isolation gates**

```bash
cargo test --test living_content
cargo test --test living_experiments
cargo test -p nyon-workshop-core --test living_strategic_fixtures
cargo test --test preferences
cargo fmt --all --check
git diff --check
```

- [ ] **Step 4: Commit validated starting experiences**

```bash
git add assets/living/starters/vale-confluence-v2.json assets/living/starters/blank-galaxy-v2.json assets/living/starters/seeded-default-v2.json src/living/content.rs src/living/client/experiments.rs src/ui/living_start.rs src/living/mod.rs src/app/onboarding.rs tests/living_content.rs tests/living_experiments.rs
git commit -m "feat(living): add validated galaxy starters"
```

### Task 5: Build the Living Navigator and complete truthful Inspector

**Files:**
- Create: `src/ui/living/mod.rs`
- Create: `src/ui/living/navigator.rs`
- Create: `src/ui/living/inspector.rs`
- Create: `src/presentation/living.rs`
- Modify: `src/ui.rs`
- Modify: `src/ui/workshop_view.rs`
- Test: `tests/living_ui_model.rs`
- Test: `tests/living_inspector.rs`
- Test: `tests/living_accessibility.rs`

**Interfaces:**
- Consumes: immutable `LivingGalaxyStateV2`, selected entity, civilization observations, receipts/reasons, `VisibleWindow`, and `WorkshopViewState`.
- Produces: stable `LivingNavigatorRowV2`, complete `LivingInspectorSectionV2`, omniscient/intelligence lenses, and semantic reveal targets.

- [ ] **Step 1: Write failing completeness and reachability tests**

```rust
#[test]
fn inspector_exposes_every_applicable_world_fact_and_blocker() {
    let model = LivingUiModelV2::extract(blocked_world_fixture(), selected_world());
    assert_eq!(model.section_titles(), ["Overview", "Civilization", "Economy", "Connections", "Environment", "History"]);
    assert!(model.economy_rows().any(|row| row.inputs_and_outputs_are_explicit()));
    assert!(model.economy_rows().any(|row| row.blockers().contains(&LivingUiBlockerV2::OutputFull)));
    assert!(model.economy_rows().any(|row| row.blockers().contains(&LivingUiBlockerV2::NoValidRoute)));
}

#[test]
fn filtered_deep_selection_materializes_its_ancestors_and_semantics() {
    let state = capacity_fixture();
    let model = LivingNavigatorV2::build(&state, "frontier", collapsed_view());
    let revealed = model.reveal_entity(last_hull_id()).unwrap();
    assert!(revealed.rows().any(|row| row.entity_id() == Some(last_hull_id())));
    assert!(revealed.semantic_focus_target().is_some());
}
```

Cover empty galaxy invitation, first/last/deepest entity, 512 worlds, 2,048 industries/routes, 4,096 shipments, 256 fleets/2,048 hulls, long names, collapsed ancestors, empty/no-match search, changing ownership, removed selection, and every inspector row for system/star/world/colony/facility/civilization/lane/route/shipment/fleet/hull/hazard/agreement/war.

- [ ] **Step 2: Define presentation-only row contracts**

```rust
pub struct LivingNavigatorRowV2 {
    pub row_id: LivingRowIdV2,
    pub entity_id: Option<LivingEntityIdV2>,
    pub depth: u8,
    pub label: String,
    pub secondary: Option<String>,
    pub expanded: Option<bool>,
    pub state: LivingRowStateV2,
}

pub struct LivingInspectorSectionV2 {
    pub id: LivingInspectorSectionIdV2,
    pub title: String,
    pub rows: Vec<LivingInspectorRowV2>,
}

pub enum LivingKnowledgeLensV2 {
    Omniscient,
    Civilization(LivingEntityIdV2),
}
```

Extraction must be pure and sorted by explicit presentation keys plus stable entity ID. Civilization lens reads only authoritative observations and labels unknown/stale facts; it cannot leak rival private inventories. Facility state exposes all simultaneous causes, not a single derived green badge. Reasons are shown only when the receipt stored them.

- [ ] **Step 3: Integrate foundation virtualization without parallel geometry**

Use `VisibleWindow` for every unbounded row collection and `WorkshopViewState` for search, collapse, scroll, drawer, and reveal. The row model supplies content only. Foundation layout supplies bounds, clipping, focus, pointer, and semantic geometry. Removed focused objects select their surviving parent or clear to galaxy without changing the document.

- [ ] **Step 4: Run model/accessibility regressions**

```bash
cargo test --test living_ui_model
cargo test --test living_inspector
cargo test --test living_accessibility
cargo test --test workshop_accessibility
cargo fmt --all --check
git diff --check
```

- [ ] **Step 5: Commit inspection models**

```bash
git add src/ui/living/mod.rs src/ui/living/navigator.rs src/ui/living/inspector.rs src/presentation/living.rs src/ui.rs src/ui/workshop_view.rs tests/living_ui_model.rs tests/living_inspector.rs tests/living_accessibility.rs
git commit -m "feat(ui): expose the complete living galaxy"
```

### Task 6: Implement conventional forms and the unrestricted creator matrix

**Files:**
- Create: `src/ui/living/form.rs`
- Create: `src/ui/living/creator.rs`
- Create: `src/living/client/creator.rs`
- Modify: `src/ui/living/mod.rs`
- Modify: `src/ui/creator.rs`
- Modify: `src/app/input_router.rs`
- Test: `tests/living_form_editing.rs`
- Test: `tests/living_creator_matrix.rs`
- Test: `tests/living_creator_accessibility.rs`

**Interfaces:**
- Consumes: foundation `TextEditState`, complete authority `LivingCreatorOpV2` variants, catalog choices, pending-tail cursor, and command validation preview.
- Produces: contextual tools, command palette entries, atomic creator preview, `LivingCommandV2::CreatorBatch`, and identical pointer/keyboard/semantic submission.

- [ ] **Step 1: Write failing conventional-edit tests**

```rust
#[test]
fn unsupported_paste_rejects_atomically_and_preserves_caret() {
    let mut field = LivingTextFieldV2::from_ascii("Aster Vale", 5..5);
    let before = field.clone();
    assert_eq!(field.paste(" Nacre ★"), Err(LivingFieldErrorV2::UnsupportedAscii));
    assert_eq!(field, before);
}

#[test]
fn invalid_numeric_intermediate_remains_editable_until_commit() {
    let mut field = LivingNumberFieldV2::from_committed(12);
    field.select_all();
    field.insert("-").unwrap();
    assert_eq!(field.draft(), "-");
    assert!(field.commit().is_err());
    assert_eq!(field.draft(), "-");
}
```

Cover caret/selection, Shift selection, select-all, copy/paste/replacement, Backspace/Delete, Home/End, word movement, cancel, focus traversal, resize preservation, shortcut suppression while typing, 1-64-byte ASCII names, spaces validation, searchable choices, and no unsupported glyph reaching SDF.

- [ ] **Step 2: Generate a complete creator capability test table**

For every supported authority operation, assert discovery in context and command search, readable preview, pointer submission, keyboard submission, semantic action, creator provenance, zero resource charge, structural validation, invalid atomic rejection, Undo/Redo identity, and save/reload replay. Include topology, archetypes, deposits/inventory, colony ownership, facilities, construction/hull queues, civilizations/policies, directed relations, agreements/war/peace, fleets/hulls/split/merge, orders, freight routes, hazards, and complete cascades.

```rust
#[test]
fn every_creator_operation_has_all_access_paths_and_provenance() {
    for case in living_creator_capability_cases() {
        assert_creator_case(case).has_contextual_entry()
            .has_palette_entry()
            .previews_exact_dependencies()
            .submits_by_pointer_keyboard_and_semantics()
            .is_free_but_structurally_valid()
            .records_creator_provenance()
            .round_trips_history_and_save();
    }
}
```

- [ ] **Step 3: Implement schema-driven creator forms**

`LivingCreatorFormV2` is a typed draft; it cannot emit an operation until every field validates. Choices are labeled lists with search when large. Multi-object edits allocate backward-only batch-local IDs and preview affected objects, cascade dispositions, application boundary, validation errors, and dependencies. Canvas creation only edits a ghost/snapped draft; explicit Apply submits. Dragging and selection never commit.

Submissions use the current published pending-tail cursor. Stale rejection preserves the draft and offers refresh/repreview; it never retries against changed authority without review. Creator cost display says `Creator: free` while listing ordinary catalog cost separately.

- [ ] **Step 4: Run matrix and authority isolation gates**

```bash
cargo test --test living_form_editing
cargo test --test living_creator_matrix
cargo test --test living_creator_accessibility
cargo test -p nyon-workshop-core --test living_command
cargo test -p nyon-workshop-core --test living_creator_authority
cargo fmt --all --check
git diff --check
```

- [ ] **Step 5: Commit complete creator tooling**

```bash
git add src/ui/living/form.rs src/ui/living/creator.rs src/living/client/creator.rs src/ui/living/mod.rs src/ui/creator.rs src/app/input_router.rs tests/living_form_editing.rs tests/living_creator_matrix.rs tests/living_creator_accessibility.rs
git commit -m "feat(creator): expose every living intervention"
```

### Task 7: Build a bounded, truthful Chronicle

**Files:**
- Create: `src/living/client/chronicle.rs`
- Create: `src/ui/living/chronicle.rs`
- Modify: `src/ui/living/mod.rs`
- Modify: `src/living/client/mod.rs`
- Test: `tests/living_chronicle.rs`
- Test: `tests/living_chronicle_accessibility.rs`

**Interfaces:**
- Consumes: canonical tick receipts/events, recorded actors/reasons/provenance, bounded replay, and selection/follow/filter state.
- Produces: `ChronicleEntryV2`, stable entity links, 100-tick aggregate summaries, latest-500 window, paging/reconstruction jobs, follow/filter, and boundary-safe auto-pause.

- [ ] **Step 1: Write failing truthfulness and memory tests**

```rust
#[test]
fn chronicle_text_links_only_to_recorded_facts() {
    let receipt = freight_delivered_receipt();
    let entry = ChronicleEntryV2::from_event(&receipt.events[0]).unwrap();
    assert_eq!(entry.provenance, ChronicleProvenanceV2::Civilization(vale_id()));
    assert_eq!(entry.entity_links, vec![vale_id(), crucible_id(), shipment_id()]);
    assert_eq!(entry.reason, receipt.events[0].recorded_reason.clone());
}

#[test]
fn chronicle_keeps_a_bounded_visible_window() {
    let chronicle = chronicle_from_100_000_ticks();
    assert!(chronicle.visible_entries().len() <= 500);
    assert!(chronicle.older_entries_require_bounded_replay());
}
```

Cover production/freight aggregation by exact 100-tick buckets; individual settlement/treaty/war/occupation/dormancy/creator events; missing reason omission; creator/civilization/natural/scheduled labels; filters; followed removed entity; entity-link selection; high-speed coalescing; replay cancellation; and no unlimited receipt retention.

- [ ] **Step 2: Implement deterministic entry projection**

Entry identity derives from the authoritative event ID. Text is a reviewed exhaustive match over event kind and uses authoritative names/resources/amounts. Unknown future event kinds render a typed unsupported-event diagnostic, not an invented story. Filters affect presentation only. Auto-pause requests a hold after the completed boundary producing a matching event and never interrupts a tick transaction.

- [ ] **Step 3: Run Chronicle and receipt coverage**

```bash
cargo test --test living_chronicle
cargo test --test living_chronicle_accessibility
cargo test -p nyon-workshop-core --test living_receipt_coverage
cargo fmt --all --check
git diff --check
```

- [ ] **Step 4: Commit the Chronicle**

```bash
git add src/living/client/chronicle.rs src/ui/living/chronicle.rs src/ui/living/mod.rs src/living/client/mod.rs tests/living_chronicle.rs tests/living_chronicle_accessibility.rs
git commit -m "feat(living): add a truthful bounded Chronicle"
```

### Task 8: Add same-tick history browsing, branching, and comparison

**Files:**
- Create: `src/living/client/history.rs`
- Create: `src/ui/living/history.rs`
- Modify: `src/living/client/mod.rs`
- Modify: `src/ui/living/mod.rs`
- Test: `tests/living_history_client.rs`
- Test: `tests/living_branch_compare.rs`
- Test: `tests/living_history_accessibility.rs`

**Interfaces:**
- Consumes: `LivingHistoryV2`, bounded replay jobs, immutable branch graph, accepted creator revisions, and state digests.
- Produces: `Viewing history`, Return/Branch/Undo/Redo actions, bookmarks, same-tick snapshots, and explicit branch differences.

- [ ] **Step 1: Write failing history identity tests**

```rust
#[test]
fn undo_and_redo_materialize_counterfactuals_at_the_same_tick() {
    let mut client = history_fixture_at_tick(900);
    let after = client.current_digest();
    let parent = client.undo().complete().unwrap();
    assert_eq!(parent.tick, LivingTickV2(900));
    assert_ne!(parent.digest, after);
    let restored = client.redo_to(intervention_revision()).complete().unwrap();
    assert_eq!((restored.tick, restored.digest), (LivingTickV2(900), after));
}

#[test]
fn historical_edit_preserves_both_futures() {
    let mut client = branched_history_fixture();
    client.undo().complete().unwrap();
    let sibling = client.submit(different_intervention()).complete().unwrap();
    assert!(client.branches().contains_revision(original_revision()));
    assert!(client.branches().contains_revision(sibling));
}
```

Cover more than four branches, same-tick comparison, common-tick labeling, changed creator inputs, stockpile/ownership/relationship/delivery differences, bookmarks with branch/revision/tick/digest, earlier bookmark versus later counterfactual distinction, cancellation, progress, past-view hold, step-back as separate navigation, and removed branch target recovery.

- [ ] **Step 2: Define history presentation contracts**

```rust
pub struct LivingBookmarkV2 {
    pub name: String,
    pub branch: LivingBranchIdV2,
    pub revision: Option<LivingRevisionIdV2>,
    pub tick: LivingTickV2,
    pub digest: LivingStateDigestV2,
}

pub struct LivingBranchComparisonV2 {
    pub left: LivingMaterializedViewV2,
    pub right: LivingMaterializedViewV2,
    pub common_tick: LivingTickV2,
    pub changed_creator_inputs: Vec<LivingRevisionSummaryV2>,
    pub differences: Vec<LivingStateDifferenceV2>,
}
```

Comparison materializes both sides at the user-selected common tick through bounded replay. Diff only explicit comparable fields; do not label a correlation causal. While browsing, show mode and Return to present persistently. Branch here submits against the historical cursor and retains the old future.

- [ ] **Step 3: Run history/replay/accessibility gates**

```bash
cargo test --test living_history_client
cargo test --test living_branch_compare
cargo test --test living_history_accessibility
cargo test -p nyon-workshop-core --test living_history
cargo test -p nyon-workshop-core --test living_replay
cargo fmt --all --check
git diff --check
```

- [ ] **Step 4: Commit history experimentation**

```bash
git add src/living/client/history.rs src/ui/living/history.rs src/living/client/mod.rs src/ui/living/mod.rs tests/living_history_client.rs tests/living_branch_compare.rs tests/living_history_accessibility.rs
git commit -m "feat(living): compare immutable galaxy futures"
```

### Task 9: Integrate semantic zoom, stable focus, and spatial creation

**Files:**
- Create: `src/living/client/navigation.rs`
- Create: `src/presentation/living_diorama.rs`
- Modify: `src/presentation/living.rs`
- Modify: `src/presentation/interaction.rs`
- Modify: `src/app/input_router.rs`
- Test: `tests/living_navigation.rs`
- Test: `tests/living_diorama.rs`
- Test: `tests/living_spatial_creation.rs`

**Interfaces:**
- Consumes: foundation diorama camera/projection/picking, immutable Living UI model, selection, scope framing cache, and creator draft controller.
- Produces: Galaxy/System/World focus, `DioramaFrame`, breadcrumbs, pointer/keyboard/button-equivalent camera actions, and noncommitting spatial previews.

- [ ] **Step 1: Write failing selection/focus/navigation tests**

```rust
#[test]
fn selection_survives_all_semantic_zoom_levels() {
    let mut nav = LivingNavigationV2::new(capacity_fixture(), selected_world());
    nav.focus_galaxy();
    nav.focus_system(parent_system());
    nav.focus_world(selected_world());
    assert_eq!(nav.selected(), Some(selected_world()));
}

#[test]
fn ordinary_authority_changes_never_auto_fit_the_camera() {
    let mut nav = framed_navigation_fixture();
    let before = nav.camera();
    nav.replace_authority(next_tick_with_new_colony());
    assert_eq!(nav.camera(), before);
}

#[test]
fn drag_updates_a_ghost_but_apply_commits_the_batch() {
    let mut creator = spatial_creator_fixture();
    creator.drag_ghost_to(Vec2::new(120.0, 88.0));
    assert_eq!(creator.submitted_batches(), 0);
    creator.apply().unwrap();
    assert_eq!(creator.submitted_batches(), 1);
}
```

Cover first load fit, explicit Home fit, invalid-target recovery, framing memory per scope, deleted focused object parent fallback, galaxy/system/world decluttering priority, click-select, double-click-focus, empty drag pan, modified drag orbit, wheel/pinch cursor-centered zoom, F, Home, breadcrumbs, visible button equivalents, offscreen non-picking, aggregation retaining inspectability, and Navigator/Inspector alternatives.

- [ ] **Step 2: Project Living state into the foundation frame**

`LivingDioramaProjectorV2::project(&LivingUiModelV2, &LivingNavigationV2) -> DioramaFrame` emits stable sorted systems/worlds/links/traffic/effects/semantic entities. Presentation seed and archetype determine world appearance; ownership changes crest/trim/pattern only. Direction, hazards, conflict, selection, and labels survive Low quality and reduced motion. Interpolation never enters authority or AI observations.

This task uses the foundation renderer contracts without editing `src/engine/render_frame.rs`. It does not add another scene enum or renderer path.

- [ ] **Step 3: Use one validated spatial interaction route**

Canvas picks dispatch the same select/focus actions as Navigator semantics. Create ghost snapping calls authority preview validation but does not queue commands. Multi-object previews remain one atomic creator batch. Blank-area drag can only pan. When scene focus is absent, F/Home shortcuts do not steal text or global navigation.

- [ ] **Step 4: Run navigation/diorama regressions**

```bash
cargo test --test living_navigation
cargo test --test living_diorama
cargo test --test living_spatial_creation
cargo test --test diorama_frame
cargo test --test renderer_contracts
cargo fmt --all --check
git diff --check
```

- [ ] **Step 5: Commit Living spatial interaction**

```bash
git add src/living/client/navigation.rs src/presentation/living_diorama.rs src/presentation/living.rs src/presentation/interaction.rs src/app/input_router.rs tests/living_navigation.rs tests/living_diorama.rs tests/living_spatial_creation.rs
git commit -m "feat(living): navigate the semantic galaxy diorama"
```

### Task 10: Wire Living views into the app, SDF chrome, and semantics

**Files:**
- Create: `src/ui/living/platform.rs`
- Create: `src/ui/living/semantics.rs`
- Modify: `src/app.rs`
- Modify: `src/app/core.rs`
- Modify: `src/ui/platform.rs`
- Modify: `src/ui/platform_native.rs`
- Modify: `src/ui/platform_web.rs`
- Modify: `src/ui/living/mod.rs`
- Test: `tests/living_app.rs`
- Test: `tests/living_ui_layout.rs`
- Test: `tests/living_accessibility.rs`
- Test: `tests/living_web.rs`

**Interfaces:**
- Consumes: Tasks 1-9 client/view models, foundation `WorkshopLayout`, `PlatformUiFrame`, `build_platform_ui_batch`, and `RenderScene::Diorama(&DioramaFrame)`.
- Produces: complete responsive Living workspace, one visible/semantic model, and native/browser action parity.

- [ ] **Step 1: Write failing app composition tests**

```rust
#[test]
fn living_app_selects_the_foundation_diorama_scene() {
    let frame = living_app_fixture().build_render_frame().unwrap();
    assert!(matches!(frame.scene, Some(RenderScene::Diorama(_))));
}

#[test]
fn all_visible_living_actions_have_matching_semantics_and_bounds() {
    let frame = living_platform_fixture(Vec2::new(723.0, 802.0), 1.30);
    for control in &frame.controls {
        let node = frame.semantic_nodes.iter().find(|node| node.action_id == control.action_id).unwrap();
        assert_eq!(node.bounds, control.bounds);
        assert!(frame.layout.viewport.contains_rect(control.bounds));
    }
}
```

Cover top bar, tool rail, Navigator, diorama, Inspector, transport, Chronicle, history, Library, command search, settings, modals, long errors, drawers, title/primary-action persistence at short heights, 723x802/1280x720/1440x900/1920x1080 at 100%/130% and 723x802 at 115%, 200% accessibility scale, high contrast, reduced motion, empty/populated/capacity models, and resize during form/import/delete/history.

- [ ] **Step 2: Build one platform input and action map**

```rust
pub struct LivingPlatformInput<'a> {
    pub snapshot: &'a LivingClientSnapshotV2,
    pub ui: &'a LivingUiModelV2,
    pub viewport: Vec2,
    pub ui_scale: f32,
    pub view: &'a WorkshopViewState,
    pub focus: Option<&'a LivingFocusTargetV2>,
}

pub fn build_living_platform_frame(input: LivingPlatformInput<'_>) -> PlatformUiFrame;
```

The builder maps every visible control to one stable action ID and semantic node. Drawing, clipping, focus, pointer hit testing, native AccessKit, and browser semantics reuse foundation bounds. Virtualized focus first expands/materializes/reveals the row. Closed/offscreen controls have neither hits nor semantic activation.

- [ ] **Step 3: Integrate into the existing app frame**

The app polls the client runtime, projects a `DioramaFrame`, and passes it through the already-landed `RenderScene::Diorama` variant. It builds one SDF `UiBatch` from the `PlatformUiFrame`; primitive overlays are limited to decoration/focus/caret/selection/diagnostics. Only intentional modals may scrim the scene. App actions route through Task 1 owner epoch and never bypass client validation.

Do not modify `src/engine/render_frame.rs`; Foundation owns that file and the enum definition. If the expected variant is absent, stop and reconcile the foundation baseline instead of locally duplicating it.

- [ ] **Step 4: Run app, layout, semantics, renderer, and web tests**

```bash
cargo test --test living_app
cargo test --test living_ui_layout
cargo test --test living_accessibility
cargo test --test living_web
cargo test --test renderer_contracts
cargo test --test workshop_ui
cargo fmt --all --check
git diff --check
```

- [ ] **Step 5: Commit the complete Living workspace**

```bash
git add src/ui/living/platform.rs src/ui/living/semantics.rs src/app.rs src/app/core.rs src/ui/platform.rs src/ui/platform_native.rs src/ui/platform_web.rs src/ui/living/mod.rs tests/living_app.rs tests/living_ui_layout.rs tests/living_accessibility.rs tests/living_web.rs
git commit -m "feat(ui): integrate the living galaxy workspace"
```

### Task 11: Implement portable native and browser import/export workflows

**Files:**
- Create: `src/library/transfer/mod.rs`
- Create: `src/library/transfer/native.rs`
- Create: `src/library/transfer/web.rs`
- Create: `src/ui/library_transfer.rs`
- Modify: `src/library/mod.rs`
- Modify: `src/library/client.rs`
- Modify: `src/platform/native.rs`
- Modify: `src/platform/web.rs`
- Modify: `Cargo.toml`
- Test: `tests/library_transfer.rs`
- Test: `tests/library_transfer_native.rs`
- Test: `tests/library_transfer_web.rs`

**Interfaces:**
- Consumes: bounded `preflight_import`, authority-specific validators/stores, standalone catalog packs, referencing archives, and platform handoff capabilities.
- Produces: `TransferControllerV1`, preview-first import, new-slot-only commit, pack registration flow, ordinary SHA-256 evidence, and honest handoff/completion states.

- [ ] **Step 1: Review dependencies before manifest edits**

Record `rfd 0.15.4` license, source, native feature closure, wasm behavior, binary impact, and offline behavior. Use it only if it supports the required macOS/Windows/Linux dialog matrix without network or unsafe code in NYON. Browser uses web APIs directly; it must not pretend a download handoff proves filesystem persistence.

- [ ] **Step 2: Write failing atomic transfer tests**

```rust
#[test]
fn import_preview_never_mutates_stores_or_active_session() {
    let mut fixture = transfer_fixture();
    let before = fixture.snapshot_all();
    let preview = fixture.controller.preflight(valid_living_archive_bytes()).complete().unwrap();
    assert_eq!(preview.route.authority(), LibraryAuthorityV1::LivingV2);
    assert_eq!(fixture.snapshot_all(), before);
}

#[test]
fn missing_catalog_requires_exact_pack_before_new_slot_commit() {
    let mut fixture = transfer_fixture();
    let preview = fixture.preflight(archive_requiring_pack_a()).complete().unwrap();
    assert!(matches!(preview.status, ImportPreviewStatusV1::MissingCatalog(pack_a_hash())));
    assert!(fixture.register_pack(pack_b_bytes()).complete().is_err());
    fixture.register_pack(pack_a_bytes()).complete().unwrap();
    let imported = fixture.commit_new_slot(preview).complete().unwrap();
    assert_ne!(imported.slot_id, fixture.source_slot());
}
```

Cover archive/pack V2 and Workshop V1 routing, unknown/wrong kind/version, byte limits, cancellation at each stage, catalog/integrity/final-digest mismatch, duplicate/capacity/stale errors, native chooser cancel, native atomic destination write, browser file cancel, Blob construction failure, download handoff, source preservation, active-session preservation, and imported graph/branch/tick/digest parity.

- [ ] **Step 3: Implement bounded transfer jobs and explicit platform evidence**

```rust
pub enum ExportCompletionV1 {
    Persisted { path: PathBuf, byte_len: u64, sha256: [u8; 32] },
    BrowserHandoff { filename: String, byte_len: u64, sha256: [u8; 32] },
}

pub enum TransferRequestV1 {
    PreflightBytes { bytes: TransferBytesV1 },
    RegisterPack { bytes: TransferBytesV1 },
    ImportIntoNewSlot { preview: ImportPreviewIdV1, name: String },
    ExportPack { catalog: LivingCatalogHashV2 },
    ExportArchive { slot: LivingSlotIdV2, generation: LivingSaveGenerationV2 },
}
```

Import preflight is read-only. Full validation/replay occurs before the final new-slot transaction. No overwrite mode exists. Failure/cancel leaves source, active session, coordinator, and stores unchanged. Archive import without a registered catalog stays in preview and offers the exact pack picker. Export records artifact kind/hash/bytes; native reports persisted only after close/sync succeeds, browser reports handoff only.

- [ ] **Step 4: Run transfer/store/wasm gates**

```bash
cargo test --test library_transfer
cargo test --test library_transfer_native
cargo test --test library_transfer_web
cargo test --test import_router
cargo check --target wasm32-unknown-unknown --lib
cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings
cargo fmt --all --check
git diff --check
```

- [ ] **Step 5: Commit portable transfer**

```bash
git add Cargo.toml Cargo.lock src/library/transfer/mod.rs src/library/transfer/native.rs src/library/transfer/web.rs src/ui/library_transfer.rs src/library/mod.rs src/library/client.rs src/platform/native.rs src/platform/web.rs tests/library_transfer.rs tests/library_transfer_native.rs tests/library_transfer_web.rs
git commit -m "feat(library): transfer portable galaxy archives"
```

### Task 12: Add isolated audio, notifications, and Living presentation preferences

**Files:**
- Create: `src/living/client/presentation.rs`
- Create: `src/living/client/audio.rs`
- Create: `src/platform/audio.rs`
- Modify: `src/living/client/mod.rs`
- Modify: `src/preferences/mod.rs`
- Modify: `src/preferences/store.rs`
- Modify: `src/app/settings.rs`
- Modify: `src/platform/mod.rs`
- Modify: `Cargo.toml`
- Test: `tests/living_presentation_isolation.rs`
- Test: `tests/living_audio.rs`
- Test: `tests/living_preferences.rs`

**Interfaces:**
- Consumes: authoritative Chronicle entries, host audio capability, existing preferences store, graphics quality, contrast, motion, and mute settings.
- Produces: rate-limited/coalesced sound cues, visual-equivalent notifications, Living presentation preferences, and digest-isolation witnesses.

- [ ] **Step 1: Review audio dependency and asset policy**

Record `rodio 0.21.1` license, native platform closure, no-device behavior, thread/lifecycle implications, binary impact, wasm exclusion, and offline operation. Audio assets must be bundled, licensed, short, and nonessential. Browser audio uses existing web capability or remains visibly unavailable; it may not block startup.

- [ ] **Step 2: Write failing isolation and rate-limit tests**

```rust
#[test]
fn every_presentation_setting_preserves_authority_and_receipt_digests() {
    let baseline = run_with_presentation(LivingPresentationPreferencesV2::default());
    for preferences in presentation_variants() {
        let run = run_with_presentation(preferences);
        assert_eq!(run.state_digest, baseline.state_digest);
        assert_eq!(run.receipt_digest, baseline.receipt_digest);
    }
}

#[test]
fn high_speed_events_are_coalesced_and_visual_information_remains() {
    let output = audio_fixture().emit(200, Duration::from_secs(1));
    assert!(output.played_cues.len() <= LIVING_AUDIO_CUES_PER_SECOND as usize);
    assert_eq!(output.visual_notifications.len(), output.informational_groups.len());
}
```

Cover mute from launch, toggle mute, no device, device loss/recovery, reduced motion, high contrast, 100%/130%/200% scale, High/Low quality, volume bounds, category toggles, notification coalescing, auto-pause independence, preference corruption recovery, native/browser serialization parity, and zero authority reads of preferences.

- [ ] **Step 3: Implement presentation-only controllers**

Map reviewed event categories to cues exhaustively. Coalesce by category/tick window and cap simultaneous/rate events; settlement/treaty/conflict/creator cues have visible Chronicle equivalents. Audio failure returns a nonfatal diagnostic once and disables retry storms. Preferences persist outside authority stores and contain no seed, tick, command, policy, or autonomous decision input.

- [ ] **Step 4: Run audio, preference, digest, and wasm tests**

```bash
cargo test --test living_presentation_isolation
cargo test --test living_audio
cargo test --test living_preferences
cargo test --test preferences
cargo check --target wasm32-unknown-unknown --lib
cargo fmt --all --check
git diff --check
```

- [ ] **Step 5: Commit isolated presentation feedback**

```bash
git add Cargo.toml Cargo.lock src/living/client/presentation.rs src/living/client/audio.rs src/platform/audio.rs src/living/client/mod.rs src/preferences/mod.rs src/preferences/store.rs src/app/settings.rs src/platform/mod.rs tests/living_presentation_isolation.rs tests/living_audio.rs tests/living_preferences.rs
git commit -m "feat(living): add isolated presentation feedback"
```

### Task 13: Automate whole-product, compatibility, and browser qualification

**Files:**
- Create: `tests/living_end_to_end.rs`
- Create: `tests/living_compatibility.rs`
- Create: `tests/living_release_contract.rs`
- Create: `tools/check-living-galaxy.sh`
- Create: `tools/browser/package.json`
- Create: `tools/browser/package-lock.json`
- Create: `tools/browser/playwright.config.ts`
- Create: `tools/browser/living-galaxy.spec.ts`
- Create: `tools/browser/fixtures/.gitkeep`
- Modify: `tools/build-web-webgpu.sh`
- Modify: `tools/build-web-webgl.sh`
- Test: `tests/browser_contract.rs`

**Interfaces:**
- Consumes: all landed authority, civilization, foundation, and experience contracts plus their fixtures/goldens.
- Produces: one source gate, native deterministic journey, browser WebGPU/WebGL2 journeys, artifact/evidence manifest, and explicit `PASS`/`FAIL`/`SKIP` boundaries.

- [ ] **Step 1: Review and pin browser-test dependencies**

Record Playwright `1.55.0` and transitive licenses, lockfile integrity, browser-binary provenance, install size, and offline-cache behavior. Do not let an unavailable browser download masquerade as application failure or silently skip a required runtime row. The browser runner must target the exact freshly built artifact, not a dev server with stale files.

- [ ] **Step 2: Write failing release-contract coverage**

```rust
#[test]
fn every_living_acceptance_id_has_an_owned_automated_or_manual_witness() {
    let manifest = LivingQualificationManifestV2::load_checked_in().unwrap();
    for id in 1..=27 {
        let key = format!("LG-{id:02}");
        assert!(manifest.criteria.contains_key(&key), "missing {key}");
        assert!(!manifest.criteria[&key].witnesses.is_empty(), "unowned {key}");
    }
}

#[test]
fn classic_and_workshop_oracles_are_byte_identical() {
    assert_eq!(current_classic_oracle(), include_bytes!("fixtures/classic-rules-v1.oracle"));
    assert_eq!(current_workshop_oracle(), include_bytes!("fixtures/workshop-v1.oracle"));
}
```

Add exact V2 wire corpus round-trip; pack/archive kind routing; native/wasm digest parity; tick 18,000 save to 36,000 continuation; 100,000-tick bounded memory; insertion/render-cadence/1x-4x-20x invariance; creator tail queue; all civilization rule fixtures; all creator matrix rows; coordinator lifecycle; transfer; presentation digest isolation; zero/one/many civilizations; and no terminal victory.

- [ ] **Step 3: Implement one fail-fast source gate with honest evidence**

`tools/check-living-galaxy.sh` uses `set -euo pipefail`, prints toolchain/commit/artifact hashes, verifies the index/worktree scope expected by its caller, and runs:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
cargo check --target wasm32-unknown-unknown --lib
cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings
cargo fmt --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all --check
cargo clippy --locked --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all-targets -- -D warnings
./tools/build-web-webgpu.sh
./tools/build-web-webgl.sh
git diff --check
```

Real fuzz runs, GPU runtime, browser runtime, performance, screen-reader, and other-host evidence are separate rows. The script never prints PASS for a row it did not execute.

- [ ] **Step 4: Implement browser acceptance with backend assertions**

Playwright tests open a disposable profile and verify Blank and Vale start, scene non-occlusion, selection/inspection, conventional form editing, time controls, Chronicle, history/branch compare, save/reload/Continue, export handoff, import into a new slot, Legacy open, keyboard-only journey, high contrast, reduced motion, muted behavior, compact drawers, and visible fallback errors. Run Chrome natural and forced WebGL2; emit selected backend, console errors, screenshots, artifact hash, slot/branch/tick/digest, and `SKIP` if a browser/runtime prerequisite is absent.

```bash
npm --prefix tools/browser ci --ignore-scripts
npm --prefix tools/browser exec playwright test -- --project=chromium-webgpu
npm --prefix tools/browser exec playwright test -- --project=chromium-webgl2
```

- [ ] **Step 5: Run the automated qualification slice**

```bash
cargo test --test living_end_to_end
cargo test --test living_compatibility
cargo test --test living_release_contract
./tools/check-living-galaxy.sh
npm --prefix tools/browser ci --ignore-scripts
npm --prefix tools/browser exec playwright test
git diff --check
```

Record actual test counts and failures from this run; never reuse design-time or historical counts.

- [ ] **Step 6: Commit automated qualification**

```bash
git add tests/living_end_to_end.rs tests/living_compatibility.rs tests/living_release_contract.rs tests/browser_contract.rs tools/check-living-galaxy.sh tools/browser/package.json tools/browser/package-lock.json tools/browser/playwright.config.ts tools/browser/living-galaxy.spec.ts tools/browser/fixtures/.gitkeep tools/build-web-webgpu.sh tools/build-web-webgl.sh
git commit -m "test(living): qualify the complete galaxy workflow"
```

### Task 14: Execute platform, accessibility, performance, offline, and delivery qualification

**Files:**
- Create: `docs/qualification/living-galaxy/README.md`
- Create: `docs/qualification/living-galaxy/results.json`
- Create: `docs/qualification/living-galaxy/manual-checklist.md`
- Create: `docs/qualification/living-galaxy/performance.md`
- Create: `docs/qualification/living-galaxy/limitations.md`
- Modify: `docs/PLAYER-MANUAL.md`
- Modify: `README.md`

**Interfaces:**
- Consumes: exact release artifacts, acceptance IDs LG-01 through LG-27, disposable slots/profiles, four strategic fixtures, backend/platform matrix, and measured benchmark output.
- Produces: traceable artifact-qualified/runtime-qualified evidence, current player documentation, and explicit gaps. It does not turn compilation into runtime proof.

- [ ] **Step 1: Freeze the artifacts and evidence schema**

Build the actual native binary target plus WebGPU and WebGL2 web outputs. Record commit, dirty-state disclosure, artifact paths and SHA-256, toolchain, catalog hash, fixture seed, host OS/browser/GPU/driver, requested/selected backend, window/scale, branch/revision/tick/digest, command, exit status, timestamps, screenshot paths, measured samples, and operator notes. Launch only those hashes.

`results.json` has one entry for each LG-01 through LG-27 and each required host/backend row with status `pass`, `fail`, or `skip`; `skip` requires a reason and never satisfies release qualification.

- [ ] **Step 2: Execute the full end-to-end release journey**

Using a disposable recorded slot and no competing process against its save:

1. Open Vale paused; inspect the real shortage; add solar or correct a route; run and verify authoritative change.
2. Run First Expansion through tick 411 and inspect exact autonomous construction, scouting, ark, and settlement witnesses.
3. Inspect Commerce deliveries, Frontier Friction declaration/combat/continued simulation, and Interrupted Corridor dispatch/production effects.
4. Inspect one recorded AI reason and one creator override with distinct provenance.
5. Bookmark branch/revision/tick/digest; submit a running intervention; pause later; record the new view.
6. Undo at the later tick, record its counterfactual digest, Redo to the exact prior digest, create a sibling future, and compare at one labeled tick.
7. Save to confirmed durable generation, quit, reopen, and Continue to the exact authority/slot/branch/revision/tick/digest.
8. Export standalone pack and referencing archive; record hashes and platform completion state. Import into a different profile/slot, resolve Missing catalog with the exact pack, reject a mismatch, and reproduce the next 1,000 ticks.
9. Open WorkshopV1 and Classic through Legacy without modifying their source; return to Living; remove/restore the final civilization and keep stepping without game over.

Execute pointer-only and keyboard-only. Screen-reader coverage includes creation, inspection, Chronicle, history, save/recovery, and transfer. Muted/reduced-motion/high-contrast runs retain equivalent information and authority results.

- [ ] **Step 3: Execute the declared layout and backend matrix**

Exercise 723x802 at 100%/115%/130%; 1280x720, 1440x900, and 1920x1080 at 100%/130%; Retina/DPI movement; and resize during editing, import, delete confirmation, and history. Prove complete drawers/forms/outliner/history, minimum 44x44 targets, readable wrapping/ellipsis/details, and no scene occlusion.

Record native macOS Metal and Safari/Chrome natural plus forced WebGL2 on the qualification Mac. Record Windows 11 DX12 plus Edge/Chrome, and Ubuntu 24.04 Vulkan/forced GL plus Firefox, when those hosts are actually available. Missing hosts remain `skip`; local compile results do not fill them.

- [ ] **Step 4: Measure performance, endurance, fallback, and offline use**

Use representative and declared-capacity fixtures. Record raw samples and p50/p95 for authority step (target <=5 ms), native/WebGPU frame at 1920x1080 (<=16.7 ms), WebGL2 Low (<=33.3 ms), and input-to-visible response outside bounded jobs (<=100 ms). Report requested versus effective simulation speed without dropped ticks.

Run 36,000 mixed ticks and 100,000 endurance ticks with invariant, digest, and bounded Chronicle/history memory checks. Force native device/surface recreation, WebGPU acquisition failure, and WebGL2 fallback. With networking disabled, perform new/open/play/edit/branch/save/export/import and verify fonts/assets/AI/narrative remain local.

- [ ] **Step 5: Update documentation from executed behavior only**

Document controls, responsive layouts, creator/free-vs-autonomous costs, time modes, Chronicle provenance, history semantics, save labels, global Continue, archive versus permanent delete, pack blockers, import/export completion meanings, Legacy boundaries, accessibility, offline operation, recovery, and known limitations. Remove statements contradicted by actual acceptance. Do not call unqualified rows complete.

- [ ] **Step 6: Run documentation/evidence validation**

```bash
./tools/check-living-galaxy.sh
cargo test --test player_guide
cargo test --test living_release_contract
npm --prefix tools/browser exec playwright test
git diff --check
```

Inspect every changed documentation file for unfinished markers before committing. Expected: source gates pass; every LG ID and executed matrix row has current evidence; the unfinished-marker inspection is empty. Required but unavailable runtime rows remain explicit `skip` entries and prevent the `Release-qualified` label.

- [ ] **Step 7: Commit qualification evidence and current manuals**

```bash
git add docs/qualification/living-galaxy/README.md docs/qualification/living-galaxy/results.json docs/qualification/living-galaxy/manual-checklist.md docs/qualification/living-galaxy/performance.md docs/qualification/living-galaxy/limitations.md docs/PLAYER-MANUAL.md README.md
git commit -m "docs(living): record release qualification evidence"
```

## Cross-Plan Execution Order

```text
Authority Tasks 1-4
        |
        +--> Civilization phase implementation and fixtures
        |
        v
Authority Tasks 5-11 + Foundation Tasks 1-7
        |
        v
Experience Tasks 1-4 (session, Continue, lifecycle, content)
        |
        v
Experience Tasks 5-10 (inspection, creator, Chronicle, history, diorama, app)
        |
        v
Experience Tasks 11-12 (transfer, presentation isolation)
        |
        v
Experience Tasks 13-14 (automated and live qualification)
```

Shared-file edits are serialized in that order. Authority owns canonical Living core/store/Library protocols; civilization owns deterministic phase behavior and strategic witnesses; foundation owns layout, text editing primitives, diorama renderer, `DioramaFrame`, `RenderScene`, and SDF construction; this plan owns client sessions, product view models, workflows, platform capability controllers, integration, and qualification.

Implementation reaches **Implemented** only after appropriate source gates pass; **Artifact-qualified** only after exact built artifacts are recorded; **Runtime-qualified** only after live interaction/backend rows pass; and **Release-qualified** only when all required platform, accessibility, performance, recovery, offline, compatibility, and documentation evidence is present. Any missing host, inaccessible action, unimplemented simulation behavior, failed row, or unverified persistence handoff remains a named gap.
