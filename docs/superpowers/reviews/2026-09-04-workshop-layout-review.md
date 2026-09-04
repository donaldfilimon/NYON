# Workshop Layout / Blank-Canvas Repair Re-review — 2026-09-04

## Verdict

**APPROVE the scoped Foundation Task 1 source repair for commit.** All four findings from the first review are closed. No open P0, P1, or P2 finding remains in the reviewed blank-canvas/layout slice.

The repaired source establishes a full-width compact canvas with mutually exclusive drawers, exact 900/1200 effective-width boundaries, a distinct Medium rail/overlay mode, complete logical focus and reveal behavior for the current Workshop model, shared visible bounds for drawing/hit testing/native and browser semantics, and a populated-world marker coverage regression in normal and high-contrast modes. Classic `SceneFrame`/`RenderFrame` ownership remains unchanged, and no Living module or `RenderScene` ownership was introduced.

This approval is source/test evidence only. It does not claim that a post-repair native or browser visual run has occurred, does not promote existing web artifacts into runtime evidence, and does not close the separate missing-star-rendering baseline finding.

## Scope and evidence boundary

- Re-reviewed the current Foundation layout implementation in `src/ui/workshop_layout.rs`, `src/ui/virtual_list.rs`, `src/ui/workshop_view.rs`, `src/ui/accessibility.rs`, `src/ui/platform.rs`, `src/ui/platform_native.rs`, `src/ui/platform_web.rs`, `src/app.rs`, and `src/app/input_router.rs`, plus `tests/workshop_ui.rs`, `tests/workshop_accessibility.rs`, `tests/workshop_web.rs`, and `tests/renderer_contracts.rs`.
- Compared the implementation with the four findings previously recorded in this file, baseline Finding 1 in `docs/superpowers/reviews/2026-09-04-workshop-v1-baseline-review.md:17-23`, Foundation Task 1 in `docs/superpowers/plans/2026-09-04-nyon-living-galaxy-foundation.md:28-133`, and the accepted responsive/accessibility requirements in `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-experience.md:95-115`.
- Independently reran `cargo test --test workshop_ui` (21 passed), `cargo test --test workshop_accessibility` (8 passed), `cargo test --test workshop_web` (8 passed), `cargo test --test renderer_contracts` (8 passed), and a scoped `git diff --check`; all exited 0.
- The controller separately reports host and wasm gates green. Those reported gates were not independently rerun in this re-review. Existing `dist/webgpu/nyon.js` and `dist/webgl/nyon.js` timestamps are artifact metadata only, not proof of startup, rendering, fallback, or visual correctness.
- No post-repair native or browser visual acceptance run was performed by this reviewer. The previous live observation of a populated but blank Workshop remains historical defect evidence; it is not current acceptance evidence.

## Prior findings

### 1. [P1] Compact layout eliminated the hierarchy and undersized creator actions

- **Severity:** P1
- **File:line:** `src/ui/workshop_layout.rs:117-139`, `src/ui/platform.rs:529-555`, `src/ui/platform.rs:663-841`, `src/ui/workshop_view.rs:62-235`, `tests/workshop_ui.rs:1022-1124`
- **Description:** Compact mode now preserves the entire working region as a full-width scene and has no permanent left/right panels. `WorkshopViewState` allows only one `Creator` or `Navigator` drawer, and the builder materializes drawer rows in bounded `VisibleWindow`s using 44-unit controls. At both 723x802/115% and /130%, the tests independently reveal every one of the fourteen creator tools and every populated outliner entry, verify that the requested logical action is materialized, and verify persistent open/close/switch controls meet the minimum size. Opening Navigator replaces Creator rather than stacking a second sheet.
- **Suggestion:** None for this finding. Keep the exact compact-size and action-size regression coverage.
- **Status:** Closed.

### 2. [P2] Responsive thresholds and Medium behavior did not match the accepted contract

- **Severity:** P2
- **File:line:** `src/ui/workshop_layout.rs:73-115`, `src/ui/workshop_layout.rs:123-139`, `src/ui/platform.rs:529-555`, `tests/workshop_ui.rs:935-991`
- **Description:** The implementation now classifies Compact below effective width 900, Medium from 900 through 1199, and Wide from 1200. Medium reserves a scaled 56-unit navigator rail, keeps the drawer closed by default, exposes a persistent 44x44 navigator action within that rail, and opens a measured overlay sheet over the scene in one action. Boundary tests exercise 899, 900, 1199, and 1200 directly, plus 1280x720 at 100% and 130%.
- **Suggestion:** None for this finding. Preserve exact transition-boundary tests when later resizable-panel work lands.
- **Status:** Closed.

### 3. [P2] Measured geometry did not drive semantics or a complete focus/reveal model

- **Severity:** P2
- **File:line:** `src/ui/platform.rs:105-147`, `src/ui/platform.rs:988-1014`, `src/ui/platform.rs:1038-1077`, `src/ui/accessibility.rs:69-89`, `src/ui/platform_native.rs:164-200`, `src/ui/platform_web.rs:197-260`, `src/app.rs:826-837`, `src/app.rs:864-885`, `src/app/input_router.rs:166-191`, `src/app/input_router.rs:460-501`, `tests/workshop_ui.rs:1126-1175`, `tests/workshop_accessibility.rs:272-308`, `tests/workshop_web.rs:54-61`
- **Description:** `PlatformControl.bounds` now feeds drawing and pointer hit testing, is copied into each visible action's `SemanticRect`, and drives native AccessKit bounds plus browser `data-nyon-bounds`. Unmaterialized semantic actions are marked invisible and non-interactive by both adapters. `PlatformUiFrame` retains a complete deduplicated logical focus order from the Workshop model; keyboard focus movement calls `reveal_action`, which opens the required drawer/section and adjusts its bounded visible window before the next frame. Pointer wheel and Page Up/Page Down use the same persistent view state. Focused tests compare hit-test, logical-focus, and semantic geometry for every visible control and prove every current Workshop model focus action can be revealed and materialized.
- **Suggestion:** None for this finding. Later hierarchy collapse/search and richer inspector virtualization remain separate Foundation Task 2 work and must preserve this parity.
- **Status:** Closed.

### 4. [P2] Tests did not prove a populated marker survived Workshop chrome composition

- **Severity:** P2
- **File:line:** `src/ui/platform.rs:1107-1191`, `tests/workshop_ui.rs:1177-1255`, `src/app.rs:810-837`, `src/app.rs:1273-1290`, `tests/renderer_contracts.rs:90-122`
- **Description:** `draw_workshop_scene` now returns the projected centers of actual world markers emitted into the Workshop underlay. The populated regression builds a real Workshop presentation frame, verifies that it contains worlds and primitive vertices, draws the closed platform overlay, and proves no opaque overlay quad covers any projected world-marker center. It runs in normal and high-contrast modes at 1280x720/100%, 723x802/115%, and 723x802/130%. The app continues to submit the Workshop scene as primitive underlay and chrome as primitive overlay, while the renderer encodes underlay before overlay. This is the requested structural compositor evidence for worlds; it does not assert that stars are rendered.
- **Suggestion:** None for this finding. Keep the later live native/browser visual recheck as a distinct acceptance gate. Track star visuals under the separate baseline finding rather than broadening this repair's claim.
- **Status:** Closed.

## Requirements coverage

| Requirement | Evidence | Result |
| --- | --- | --- |
| Full-width compact scene with mutually exclusive drawers | `src/ui/workshop_layout.rs:117-139`, `src/ui/workshop_view.rs:11-15`, `:62-84` | Covered |
| All 14 tools and populated hierarchy logically reachable at 723x802/115-130% | `src/ui/workshop_view.rs:116-147`, `tests/workshop_ui.rs:1022-1124` | Covered |
| Visible compact actions are at least 44x44 | `src/ui/platform.rs:529-555`, `:667-840`; focused tests green | Covered |
| Exact 900/1200 boundaries and distinct Medium rail/overlay | `src/ui/workshop_layout.rs:73-139`, `tests/workshop_ui.rs:935-991` | Covered |
| Shared drawing, hit-test, focus, and semantic geometry | `src/ui/platform.rs:105-147`, `:1038-1077`, `tests/workshop_ui.rs:1126-1175` | Covered for visible controls |
| Native and browser adapters consume visibility and bounds | `src/ui/platform_native.rs:164-200`, `src/ui/platform_web.rs:202-260` | Covered in source; native adapter test and host browser-contract test green |
| Complete current-model logical reveal path | `src/ui/workshop_view.rs:116-203`, `src/app/input_router.rs:166-191`, `:460-501` | Covered |
| Populated world-marker center not obscured in normal/high contrast | `src/ui/platform.rs:1107-1191`, `tests/workshop_ui.rs:1177-1219` | Covered structurally |
| Classic scene ABI unchanged | `src/engine/render_frame.rs:9-25`, `tests/renderer_contracts.rs:117-122` | Covered |
| Living rendering ownership untouched | `src/living` remains absent; no `RenderScene` contract was introduced | Covered |
| Post-repair live native/browser visual result | Not performed in this review | Outstanding acceptance evidence, not a source finding |
| Star rendering | Outside this repair; no star marker is claimed | Separate baseline finding remains open |

## Commit recommendation

Approve a narrowly scoped commit containing the reviewed Foundation layout repair and its tests. Do not describe that commit as live native/browser visual acceptance, do not treat existing web bundles as proof that the current source ran, and do not close the separate star-rendering finding. After commit, the appropriate next evidence is a fresh populated Workshop run in native and browser builds at the required compact scales and in normal/high-contrast modes.
