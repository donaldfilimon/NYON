# Workshop V1 Sighted Inspector Review — 2026-09-04

Historical pre-repair review. Its three source findings were closed by [the subsequent repair acceptance](2026-09-04-workshop-inspector-repair-acceptance.md). The original findings and gate results below remain as the evidence trail, not the current source verdict.

## Verdict

**REQUEST CHANGES.** No P0 or P1 finding was found, but three P2 findings remain in the completed sighted-inspector slice. The catalog-backed facts, ten entity-kind branches, overview, Foundry recipe, active ion-storm arithmetic, current-frame semantic/sighted parity, ordinary responsive layouts, and authority-preservation checks are sound. The implementation does not yet satisfy the accepted short-height layout contract, does not keep informative semantic identities stable across ordinary state evolution, and materially enlarges two already oversized shared UI modules instead of using the inspector boundary the feature itself establishes.

This verdict is source and host-test evidence only. It is not a native/browser visual acceptance result and does not claim that any currently built artifact contains this source.

## Scope and evidence boundary

- Reviewed the current source in `crates/nyon-workshop-core/src/pack.rs`, `crates/nyon-workshop-core/tests/pack.rs`, `src/ui/workshop.rs`, `src/ui/workshop_view.rs`, `src/ui/platform.rs`, `src/ui/guide.rs`, `src/app.rs`, `src/app/input_router.rs`, and `tests/workshop_ui.rs`. Supporting read-only inspection covered `crates/nyon-workshop-core/src/simulation.rs`, `src/ui/accessibility.rs`, `src/ui/platform_native.rs`, `src/ui/platform_web.rs`, `assets/workshop/core-pack-v1.json`, the current V1 design, and the committed Foundation responsive requirements.
- `crates/nyon-workshop-core/src/pack.rs`, its test, and the reviewed `src/ui/*` and `tests/workshop_ui.rs` paths remain untracked in the current Foundation working tree. Therefore Git cannot produce a committed before/after patch for most of this slice. The review uses direct current-source inspection, the task-declared changed regions, the previous baseline review's recorded module size, and executable focused gates. It does not treat an empty scoped Git diff as proof of no scope creep.
- `src/app.rs` and `src/app/input_router.rs` have a much larger tracked Workshop baseline diff than this inspector slice. Within the task-declared follow-up, the inspected app wiring obtains the catalog from the active validated Workshop history and passes it read-only into `WorkshopUiContext`; the input-router addition only classifies wheel position as drawer, docked inspector, or neither. Other concurrent tracked and untracked changes were preserved and are not approved by this review.
- No product file was edited, staged, or committed. This document is the only file created by the reviewer.

## Findings

### 1. [P2] The docked inspector overlaps fixed controls at valid short heights and scrolls away its title

- **Severity:** P2
- **File:line:** `src/ui/platform.rs:666-756`, `src/ui/platform.rs:1130-1199`, `src/ui/workshop_view.rs:90-131`, `src/ui/workshop_view.rs:273-293`, `tests/workshop_ui.rs:717-943`
- **Description:** Docked inspector text always begins 212 units below the right panel's top and reserves 416 units from its height, but when less space exists the body is forced to a 14-unit minimum. The fixed scroll/preferences/remove controls are independently positioned upward from the panel bottom. At the valid 1280x480/130% Medium layout, the right panel spans y=72.8 through 404.6, the inspector starts at y=284.8, and its 14-unit body places the 13.05-unit title through y=297.85. The reduced-motion control spans y=256.6 through 300.6, so the two overlap. At 1280x480/100% Wide, the title at y=268 through 281.05 intersects both the inspector-scroll controls ending at y=270 and the reduced-motion control starting at y=274. Drawing text before controls merely obscures the text; it does not make the layout non-overlapping or the semantic bounds truthful. A multi-line record can extend still farther because the first record is admitted even when `y + height > region.max.y`, and the stored `clip` is not consumed by `PrimitiveBatch::text`. In addition, the title is record zero in `InspectorModel::text_records`; one Next action advances `inspector_start` to one, so every scrolled view drops the sighted title. This contradicts the accepted requirement that short-height panel bodies scroll while the title and primary action remain visible.
- **Suggestion:** Split the inspector into an always-visible measured header, a body viewport calculated from the actual top and bottom reserved control rectangles, and fixed footer actions. If the body has no positive space, use a drawer/overlay or a deliberate collapsed state rather than intersecting controls. Virtualize or clip at line granularity so one long record cannot escape the body, and keep the title outside `inspector_start`. Add 1280x480 at 100% and 130%, a maximum 512-byte validated description, first/last body rows, and title-persistence assertions for both Wide and Medium; require every sighted bound to be inside its body clip and disjoint from every control.
- **Status:** Open — source-commit blocker for the sighted-inspector slice.

### 2. [P2] Informative semantic IDs change when earlier-sorted rows are inserted

- **Severity:** P2
- **File:line:** `src/ui/workshop.rs:174-217`, `src/ui/workshop.rs:1741-1760`, `src/ui/workshop.rs:1953-1963`, `src/ui/platform.rs:1163-1193`, `src/ui/platform.rs:1217-1243`, `src/ui/platform_native.rs:210-220`, `tests/workshop_ui.rs:717-759`
- **Description:** The shared sighted/semantic model correctly uses `SemanticNodeId` as its current-frame join key, but `assign_inspector_semantic_ids` appends each row's current enumeration index. That ordinal is presentation position, not object identity. For example, a world inventory containing Energy and Ore produces IDs ending in `energy.0` and `ore.1`; when ordinary Foundry output inserts Alloy ahead of them in the `BTreeMap`, those existing facts become `energy.1` and `ore.2`. The same shift affects deposit, industry, route, shipment, and hazard rows when a lower-sorted entity appears. Native AccessKit deliberately maps `SemanticNodeId` to persistent `NodeId`, and the browser uses it as the DOM element ID, so this churn removes and recreates unchanged facts, losing accessibility continuity and accumulating stale native identity mappings. The test named `inspector_visible_records_and_semantic_rows_share_stable_ids_and_values` proves only that the two representations agree inside one frame; it never compares identity across a state transition.
- **Suggestion:** Derive row IDs from stable field identity plus the complete underlying `CatalogId` or `EntityId`, not the row's current index or display label. Keep ordinals only for genuinely repeated facts that have no stronger key, and make that ordinal stable within the owning entity. Add a two-frame test that produces Alloy, inserts/removes an earlier-sorted related entity, and verifies the IDs of unchanged Energy/Ore and related-object facts remain identical while sighted and semantic values update in place.
- **Status:** Open — accessibility/semantic correctness blocker.

### 3. [P2] The slice adds another large responsibility block to already oversized shared UI modules

- **Severity:** P2
- **File:line:** `src/ui/workshop.rs:1388-2156`, `src/ui/workshop.rs:2307-2500`, `src/ui/platform.rs:1130-1243`
- **Description:** The prior baseline review recorded `src/ui/workshop.rs` at 2,144 lines and already identified its mixed model/creator/outliner/inspector/timeline/semantics ownership as a P2 maintainability blocker. It is now 2,752 lines, with roughly 770 contiguous lines of inspector assembly/arithmetic plus inspector semantic construction in the same module. `src/ui/platform.rs` is now 1,678 lines and owns shell controls, responsive control geometry, inspector wrapping/virtualization/semantic geometry, and scene primitive drawing. The new `InspectorModel` and `PlatformSightedText` contracts provide clean extraction seams, but the implementation leaves them embedded in the two busiest UI files. This is not a small incidental increase: the short-height overlap in Finding 1 and the unstable identity in Finding 2 both sit at the boundary currently split across these large modules.
- **Suggestion:** Move catalog-backed inspector derivation, stable record identity, and readiness/hazard presentation math into a focused Workshop inspector module; move the measured inspector body/header/footer layout and sighted-record construction into a focused platform-inspector module. Keep narrow re-exports so callers and tests do not absorb churn. The refactor should reduce concepts in the parent files rather than merely move lines, with pure tests colocated around record identity and geometry.
- **Status:** Open — maintainability blocker under the repository's strict review standard.

## Verified behavior and requirements coverage

| Requirement | Evidence | Result |
| --- | --- | --- |
| Overview plus all ten V1 entity kinds | `src/ui/workshop.rs:1426-1733`; `tests/workshop_ui.rs:537-714` covers overview, faction, system, star, world, lane, deposit, industry, route, shipment, and hazard | Covered |
| Validated catalog metadata and truthful raw IDs | `crates/nyon-workshop-core/src/pack.rs:96-125`, `:163-187`, `:212-284`, `:356-468`; selected star/world/deposit/industry/route/shipment/hazard rows retain raw IDs | Covered |
| Catalog metadata cannot mutate canonical bytes/hash | Validated pack fields remain private; accessors return shared references; `crates/nyon-workshop-core/tests/pack.rs:25-62` freezes canonical bytes and hash around reads | Covered |
| Foundry recipe and readiness | `src/ui/workshop.rs:1570-1620`, `:2014-2095`; `tests/workshop_ui.rs:773-822` proves 2 Energy + 3 Ore to 1 Alloy and reports both missing inputs | Covered for V1 recipe/readiness |
| Active ion-storm status and effective route capacity | `src/ui/workshop.rs:1622-1661`, `:1684-1709`, `:2103-2155`, `:2723-2733` matches simulation's half-open interval and ordered floor scaling in `crates/nyon-workshop-core/src/simulation.rs:390-401`, `:464-487`; focused test proves 2 becomes 1 | Covered |
| Same semantic and sighted content model | `InspectorTextRecord` feeds both `build_semantic_tree` and `PlatformSightedText`; current-frame label/value/bounds parity test passes | Covered within one frame; stable cross-frame identity fails Finding 2 |
| Visible wrapping and clipping | ASCII content is losslessly chunked to fixed glyph advance and ordinary tested frames stay inside their clips | Partial; short-height/oversized-record clipping fails Finding 1 |
| Wide, Medium, Compact inspector | Wide and Medium use the docked right panel; Compact uses the mutually exclusive Navigator/Inspector drawer; 1440x900, 1280x720/130%, and 723x802/130% tests pass | Covered at tested heights; valid short heights fail Finding 1 |
| Virtual scrolling and semantic reveal | Record-based next/previous, wheel routing, and `reveal_semantic` materialize every tested compact row | Partial; title is scrolled away and oversized records are not line-virtualized |
| Pointer, keyboard, focus, and semantic routing | Shared view actions drive buttons and wheel; control hit/focus/semantic geometry suites remain green; offscreen informative nodes are hidden | Covered for current controls and ordinary geometry |
| Active validated catalog wiring | `src/app.rs:772-814` obtains `session.history().catalog()` and passes the same pack and hash read-only into the UI model | Covered |
| Right-panel wheel routing only | `src/app/input_router.rs:487-529` routes drawer first, then right panel, and ignores scene/no-panel positions; focused unit test passes | Covered |
| Workshop canonical bytes/digest | UI model test snapshots canonical bytes and digest before all overview/entity builds and proves they are unchanged | Covered |
| Catalog canonical bytes/hash | Frozen core hash, canonical round-trip, reordered-source invariance, and read-only metadata test pass | Covered |
| Classic ABI and star/layout slices | Renderer ABI contracts pass; full Workshop UI suite retains responsive layout, scene occlusion, star layer/selection/contrast, and shared-control geometry tests | Covered by source/tests; no live pixels claimed |
| Authority separation | Inspector functions accept immutable state/catalog references and return presentation-only strings/models; app wiring does not construct or enqueue authority commands | Covered |
| No Living/SDF ownership change in this slice | Task-identified changes remain in V1 pack/UI/platform/app input wiring. Concurrent SDF/renderer working-tree changes exist outside this slice and are not attributed or approved here | Covered within the reviewable task boundary |

## Independent gate results

- `cargo test -p nyon-workshop-core --test pack -- --nocapture`: 9 passed, 0 failed.
- `cargo test --test workshop_ui -- --nocapture`: 28 passed, 0 failed.
- `cargo test --test workshop_accessibility -- --nocapture`: 8 passed, 0 failed.
- `cargo test --test workshop_web -- --nocapture`: 9 passed, 0 failed.
- `cargo test --test renderer_contracts -- --nocapture`: 8 passed, 0 failed.
- `cargo test --lib workshop_wheel_targets_drawer_or_docked_inspector_but_not_scene_canvas -- --nocapture`: 1 passed, 0 failed.
- `cargo clippy -p nyon-workshop-core --test pack -- -D warnings`: exited 0.
- `cargo clippy -p nyon --test workshop_ui -- -D warnings`: exited 0.
- `cargo fmt --all --check`: exited 0.
- Scoped `git diff --check` exited 0, but the untracked-path limitation above applies; this is not a complete patch-level scope proof.

Green gates do not exercise the 1280x480 short-height collision or compare semantic row identities across a changing inventory/collection, so they do not close Findings 1 or 2.

## Commit recommendation

Do not commit the sighted-inspector slice as complete. First separate the header/body/footer geometry and cover valid short heights plus maximum wrapped records; derive stable informative node IDs from authoritative/catalog identity rather than mutable order; and extract the new inspector responsibilities from the two oversized modules. Then rerun the focused pack/UI/accessibility/web/renderer gates and perform fresh native and browser visual acceptance across Wide, Medium, and Compact. Keep that live evidence separate from the source/test verdict and preserve the already reviewed star/layout slices and Classic ABI.
