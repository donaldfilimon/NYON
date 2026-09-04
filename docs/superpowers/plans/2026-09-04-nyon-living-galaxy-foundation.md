# NYON Living Galaxy Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the readable, responsive, accessible cosmic-diorama foundation that makes the existing Workshop galaxy, logistics, creation, inspection, and history visible before Living Galaxy V2 simulation is integrated.

**Architecture:** A pure measured `WorkshopLayout` becomes the single source for canvas, panels, controls, hit testing, focus, and semantics. WorkshopV1 state remains immutable authority; its presentation extractor feeds a renderer-neutral `DioramaFrame`, selected through an explicit render-scene enum beside the frozen Classic scene. SDF text/panels and the diorama use separate render stages, so ordinary and high-contrast chrome cannot cover the galaxy.

**Tech Stack:** Rust nightly-2026-09-01, winit 0.30.12, wgpu 30.0.1, glam, bytemuck, naga validation, embedded Inter SDF atlas, AccessKit, browser semantic mirror, WebGPU, and WebGL2.

**Spec:** `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-design.md`, with UI/visual authority in `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-experience.md` and acceptance in `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-qualification.md`

## Global Constraints

- Preserve Classic RulesV1 and WorkshopV1 authority, canonical bytes, digests, phase order, store identity, and fixture behavior.
- No presentation, layout, camera, animation, GPU result, accessibility state, or wall-clock value may enter an authority digest.
- One measured layout drives drawing, clipping, pointer hit testing, focus order, native semantics, and browser semantics.
- Qualify 723x802, 1280x720, 1440x900, and 1920x1080 at 100% and 130%; also qualify 723x802 at 115%.
- Compact layout uses drawers and scrolling; it must not truncate hierarchy, inspector, branches, timeline, or forms.
- High and Low quality retain text, selection, ownership, travel direction, and hazards. Reduced motion removes nonessential motion only.
- Essential rendering works without compute shaders on WebGL2 and without a network connection.
- Unsupported edit characters reject atomically before reaching the ASCII-only SDF renderer.
- Before Task 1, use the git-worktree skill to create or verify an isolated workspace seeded from an explicitly accepted baseline. Do not stage, overwrite, or relocate preexisting dirty Workshop work to manufacture that baseline.
- Each task stages and commits only its listed paths and runs `git diff --check` before commit.

---

### Task 1: Establish measured layout and repair scene occlusion

**Files:**
- Create: `src/ui/workshop_layout.rs`
- Modify: `src/ui.rs`
- Modify: `src/ui/platform.rs`
- Modify: `src/app.rs`
- Test: `tests/workshop_ui.rs`
- Test: `tests/renderer_contracts.rs`

**Interfaces:**
- Consumes: logical viewport, persisted UI scale, and platform-frame content.
- Produces: `WorkshopLayout`, `WorkshopLayoutMode`, `PlatformBackground`, and region-only Workshop chrome.

- [ ] **Step 1: Add failing layout and compositor tests**

```rust
use nyon::ui::workshop_layout::{WorkshopLayout, WorkshopLayoutMode};

#[test]
fn workshop_layout_preserves_a_real_canvas_at_supported_sizes() {
    for (viewport, scale, expected) in [
        (Vec2::new(723.0, 802.0), 1.15, WorkshopLayoutMode::Compact),
        (Vec2::new(1280.0, 720.0), 1.30, WorkshopLayoutMode::Medium),
        (Vec2::new(1440.0, 900.0), 1.00, WorkshopLayoutMode::Wide),
    ] {
        let layout = WorkshopLayout::resolve(viewport, scale).unwrap();
        assert_eq!(layout.mode, expected);
        assert!(layout.canvas.width() >= 320.0);
        assert!(layout.canvas.height() >= 300.0);
        assert!(layout.chrome.iter().all(|rect| !rect.overlaps(layout.canvas)));
    }
}

#[test]
fn workshop_frame_never_draws_a_full_viewport_overlay() {
    let frame = workshop_platform_fixture(Vec2::new(1280.0, 720.0), 1.0);
    assert_eq!(frame.background, PlatformBackground::ChromeOnly);
    assert!(frame.layout.chrome.iter().all(|rect| *rect != frame.layout.viewport));
}
```

Extend the renderer-order assertion so the diorama is before SDF chrome and the primitive overlay contains focus/diagnostics only.

- [ ] **Step 2: Run tests and observe the expected failures**

```bash
cargo test --test workshop_ui workshop_layout_preserves_a_real_canvas_at_supported_sizes -- --exact
cargo test --test workshop_ui workshop_frame_never_draws_a_full_viewport_overlay -- --exact
cargo test --test renderer_contracts sdf_ui_is_composed_between_legacy_chrome_and_the_frozen_overlay -- --exact
```

Expected: the first two fail to compile because the layout/background contracts do not exist; the existing order test characterizes the current occlusion-prone path.

- [ ] **Step 3: Implement the pure layout contract**

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkshopLayoutMode { Compact, Medium, Wide }

#[derive(Clone, Debug, PartialEq)]
pub struct WorkshopLayout {
    pub viewport: PlatformRect,
    pub top_bar: PlatformRect,
    pub canvas: PlatformRect,
    pub left_panel: Option<PlatformRect>,
    pub right_panel: Option<PlatformRect>,
    pub bottom_bar: PlatformRect,
    pub chrome: Vec<PlatformRect>,
    pub mode: WorkshopLayoutMode,
}

impl WorkshopLayout {
    pub fn resolve(viewport: Vec2, ui_scale: f32) -> Result<Self, LayoutError>;
}
```

Use these unscaled targets: top 56, bottom 58, wide left 280, wide right 304, medium left 244, medium right 264, compact side panels as overlays opened one at a time. Clamp to supported scales; reject nonfinite/nonpositive viewports. Add `width`, `height`, `intersection`, `contains_rect`, and `overlaps` to `PlatformRect`.

- [ ] **Step 4: Draw Workshop chrome only outside the canvas**

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformBackground { Full, ChromeOnly }
```

Add `layout` and `background` to `PlatformUiFrame`. Shell/settings use `Full`; Workshop uses `ChromeOnly`. `PlatformUiFrame::draw` emits Workshop quads only for `layout.chrome`, never the viewport or canvas. Pass real UI scale into `build_workshop_platform_frame`. Keep focus/diagnostics in `overlay_batch`.

- [ ] **Step 5: Run focused and regression tests**

```bash
cargo test --test workshop_ui workshop_layout -- --nocapture
cargo test --test renderer_contracts
cargo test --test workshop_presentation
cargo fmt --all --check
git diff --check
```

Expected: all pass and prove nonempty canvas plus no full-window Workshop overlay in both contrast modes.

- [ ] **Step 6: Commit the compositor foundation**

```bash
git add src/ui/workshop_layout.rs src/ui.rs src/ui/platform.rs src/app.rs tests/workshop_ui.rs tests/renderer_contracts.rs
git commit -m "fix(ui): keep the Workshop galaxy visible"
```

### Task 2: Replace collection truncation with virtualization

**Files:**
- Create: `src/ui/virtual_list.rs`
- Create: `src/ui/workshop_view.rs`
- Modify: `src/ui.rs`
- Modify: `src/ui/platform.rs`
- Modify: `src/ui/workshop.rs`
- Modify: `src/app.rs`
- Modify: `src/app/input_router.rs`
- Test: `tests/workshop_ui.rs`
- Test: `tests/workshop_accessibility.rs`

**Interfaces:**
- Consumes: complete ordered model collections and measured layout regions.
- Produces: `VisibleWindow`, `WorkshopViewState`, search/collapse state, scroll offsets, drawers, and focus reveal.

- [ ] **Step 1: Write failing virtualization tests**

```rust
#[test]
fn visible_window_reveals_first_last_and_focused_rows() {
    let mut window = VisibleWindow::new(100, 10, 0);
    assert_eq!(window.range(), 0..10);
    window.reveal(99);
    assert_eq!(window.range(), 90..100);
    window.reveal(45);
    assert!(window.range().contains(&45));
}

#[test]
fn focused_virtual_control_is_materialized_before_focus_moves() {
    let model = model_with_outliner_rows(40);
    let mut view = WorkshopViewState::default();
    view.reveal_action(&model, &SemanticActionId::new("outliner.entity-39"));
    let frame = build_workshop_platform_frame(WorkshopPlatformInput::new(
        &model, Vec2::new(723.0, 802.0), 1.15, &view, None,
    ));
    assert!(frame.controls.iter().any(|c| c.action_id.as_str() == "outliner.entity-39"));
}
```

Add cases for 12 branches, every inspector row, filtered ancestors, empty search, compact drawer exclusivity, and all timeline actions at 723x802/130%.

- [ ] **Step 2: Run tests and observe missing types**

```bash
cargo test --test workshop_ui visible_window -- --nocapture
cargo test --test workshop_ui focused_virtual_control_is_materialized_before_focus_moves -- --exact
```

Expected: compile failures for `VisibleWindow`, `WorkshopViewState`, and `WorkshopPlatformInput`.

- [ ] **Step 3: Implement bounded visible windows**

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VisibleWindow { start: usize, length: usize, total: usize }

impl VisibleWindow {
    pub fn new(total: usize, length: usize, start: usize) -> Self;
    pub fn range(self) -> std::ops::Range<usize>;
    pub fn scroll_rows(&mut self, delta: isize);
    pub fn reveal(&mut self, index: usize);
}
```

Use saturating bounds. Zero-length regions return `0..0`; no arithmetic may wrap on imported maximum-size collections.

- [ ] **Step 4: Implement persistent view state and search**

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkshopDrawer { Hierarchy, Inspector, History }

#[derive(Clone, Debug, Default)]
pub struct WorkshopViewState {
    pub query: String,
    pub collapsed: BTreeSet<EntityId>,
    pub outliner_start: usize,
    pub inspector_start: usize,
    pub branch_start: usize,
    pub open_drawer: Option<WorkshopDrawer>,
}
```

Filter case-insensitively over printable ASCII labels/kinds and retain ancestors of matching descendants. `reveal_action` expands ancestors, opens the compact drawer, scrolls, rebuilds, then focuses. All logical entries remain semantic; visual controls materialize only in the window.

- [ ] **Step 5: Remove fixed `take` and width-break behavior**

Replace `.take(14)`, `.take(4)`, status `.take(3)`, and timeline `break` with region-derived windows or `toolbar.more`. Add `hierarchy.scroll.previous/next`, `inspector.scroll.previous/next`, and `history.scroll.previous/next`. Pointer wheel scrolls the pointed region. PageUp/PageDown/Home/End and tree arrows update the focused region.

- [ ] **Step 6: Run UI/accessibility coverage**

```bash
cargo test --test workshop_ui
cargo test --test workshop_accessibility
cargo test --test workshop_web
cargo fmt --all --check
git diff --check
```

- [ ] **Step 7: Commit virtualization**

```bash
git add src/ui/virtual_list.rs src/ui/workshop_view.rs src/ui.rs src/ui/platform.rs src/ui/workshop.rs src/app.rs src/app/input_router.rs tests/workshop_ui.rs tests/workshop_accessibility.rs
git commit -m "feat(ui): make Workshop collections fully reachable"
```

### Task 3: Replace step-only string editing with conventional text fields

**Files:**
- Create: `src/ui/text_edit.rs`
- Modify: `src/ui.rs`
- Modify: `src/ui/platform.rs`
- Modify: `src/app/input_router.rs`
- Modify: `src/platform/native.rs`
- Modify: `src/platform/web.rs`
- Test: `tests/workshop_ui.rs`
- Test: `tests/workshop_accessibility.rs`
- Test: `tests/workshop_web.rs`

**Interfaces:**
- Consumes: committed field text, native key/text/IME events, browser input/composition/clipboard events, and shared measured control rectangles.
- Produces: `TextEditState`, atomic edit outcomes, selection/caret geometry, cancel/commit actions, and semantic text-field values.

- [ ] **Step 1: Write failing editing-contract tests**

```rust
#[test]
fn printable_ascii_insert_replace_delete_and_cancel_are_conventional() {
    let mut edit = TextEditState::begin("Nacre");
    edit.select(0..5).unwrap();
    assert_eq!(edit.insert("Aster Vale"), Ok(TextEditChange::Changed));
    assert_eq!(edit.text(), "Aster Vale");
    edit.backspace();
    assert_eq!(edit.text(), "Aster Val");
    assert_eq!(edit.cancel(), "Nacre");
}

#[test]
fn rejected_paste_is_atomic_and_preserves_selection() {
    let mut edit = TextEditState::begin("Nacre");
    edit.select(1..4).unwrap();
    let before = edit.clone();
    assert_eq!(edit.insert("🌌"), Err(TextEditError::UnsupportedText));
    assert_eq!(edit, before);
}
```

Add coverage for arrows with Shift, Home/End, Command/Ctrl-A, clipboard paste, pointer drag selection, double-click word selection, IME commit rejection, 64-byte limits, leading/trailing/doubled-space validation at commit, Escape restore, Enter commit, Tab commit-and-focus-next, and focus restoration after validation errors.

- [ ] **Step 2: Verify the tests fail for the missing editor**

```bash
cargo test --test workshop_ui text_edit -- --nocapture
cargo test --test workshop_web browser_text_input -- --nocapture
```

Expected: unresolved `TextEditState`; existing plus/minus controls cannot satisfy the editing journeys.

- [ ] **Step 3: Implement a platform-independent edit state**

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextEditState {
    original: String,
    text: String,
    anchor: usize,
    caret: usize,
}

impl TextEditState {
    pub fn begin(committed: &str) -> Self;
    pub fn text(&self) -> &str;
    pub fn selection(&self) -> std::ops::Range<usize>;
    pub fn insert(&mut self, text: &str) -> Result<TextEditChange, TextEditError>;
    pub fn backspace(&mut self) -> TextEditChange;
    pub fn delete_forward(&mut self) -> TextEditChange;
    pub fn move_caret(&mut self, motion: TextMotion, extend: bool);
    pub fn select(&mut self, range: std::ops::Range<usize>) -> Result<(), TextEditError>;
    pub fn commit(&self) -> Result<String, TextEditError>;
    pub fn cancel(self) -> String;
}
```

Byte offsets must always be UTF-8 boundaries even though accepted committed names are printable ASCII. Validate the complete proposed result before mutation so unsupported paste/composition never partially edits the field.

- [ ] **Step 4: Route native and browser input into the same editor**

Native maps winit keyboard, received-character, IME commit, and pointer events. Browser semantic mirrors use real text inputs but dispatch the same typed edit actions; JavaScript must not mutate authority fields directly. Render selection, caret, validation text, and focus ring from the same measured field rectangle used for hit testing and accessibility.

- [ ] **Step 5: Run editing and compatibility gates**

```bash
cargo test --test workshop_ui
cargo test --test workshop_accessibility
cargo test --test workshop_web
cargo test --test workshop_client
cargo fmt --all --check
git diff --check
```

- [ ] **Step 6: Commit conventional editing**

```bash
git add src/ui/text_edit.rs src/ui.rs src/ui/platform.rs src/app/input_router.rs src/platform/native.rs src/platform/web.rs tests/workshop_ui.rs tests/workshop_accessibility.rs tests/workshop_web.rs
git commit -m "feat(ui): add conventional Workshop text editing"
```

### Task 4: Establish deterministic diorama projection, navigation, and picking

**Files:**
- Create: `src/presentation/diorama_camera.rs`
- Modify: `src/presentation/mod.rs`
- Modify: `src/presentation/workshop.rs`
- Modify: `src/app.rs`
- Modify: `src/app/input_router.rs`
- Test: `tests/diorama_camera.rs`
- Test: `tests/workshop_presentation.rs`

**Interfaces:**
- Consumes: authoritative integer positions, measured canvas, selection, wheel/pinch/drag/key navigation, and optional reduced-motion preference.
- Produces: stable world-to-canvas projection, inverse ray/plane projection, semantic zoom bands, camera targets, and depth-aware picking candidates.

- [ ] **Step 1: Write failing camera and picking tests**

```rust
#[test]
fn projection_round_trips_the_galaxy_plane_within_one_pixel() {
    let camera = DioramaCamera::fit(Rect::new(280.0, 56.0, 916.0, 786.0), fixture_bounds());
    for point in fixture_points() {
        let canvas = camera.project(point).unwrap();
        let recovered = camera.unproject_to_plane(canvas, point.z).unwrap();
        assert!((recovered - point).length() <= camera.world_units_per_pixel());
    }
}

#[test]
fn pick_order_is_depth_then_distance_then_stable_id() {
    let frame = overlapping_diorama_fixture();
    assert_eq!(frame.pick(Vec2::new(500.0, 400.0)).unwrap().entity_id, EntityId::new(7));
}
```

Add tests for empty and one-object fits, nonfinite input rejection, resize preservation, pan bounds, zoom clamps, focus-selected behavior, keyboard navigation, 723x802 compact layout, and reduced-motion immediate camera settling.

- [ ] **Step 2: Verify the tests fail for missing camera contracts**

```bash
cargo test --test diorama_camera
cargo test --test workshop_presentation camera -- --nocapture
```

- [ ] **Step 3: Implement finite, clamped camera mathematics**

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticZoom { Galaxy, System, World }

#[derive(Clone, Copy, Debug)]
pub struct DioramaCamera {
    pub target: DVec3,
    pub yaw_radians: f64,
    pub pitch_radians: f64,
    pub distance: f64,
    pub canvas: Rect,
}

impl DioramaCamera {
    pub fn fit(canvas: Rect, bounds: DioramaBounds) -> Self;
    pub fn project(&self, world: DVec3) -> Option<Vec2>;
    pub fn unproject_to_plane(&self, canvas: Vec2, z: f64) -> Option<DVec3>;
    pub fn semantic_zoom(&self) -> SemanticZoom;
    pub fn apply(&mut self, intent: CameraIntent, reduced_motion: bool);
}
```

Keep authority coordinates unchanged; convert only in presentation. Reject nonfinite data, clamp pitch away from singularity, clamp distance from zero, and fit degenerate bounds with a documented minimum radius.

- [ ] **Step 4: Route navigation and picking through typed intents**

Wheel/pinch zooms around pointer/focus, primary drag pans, secondary drag or dedicated keys orbit, `F` focuses selection, and Escape exits focused drill-down. Clicking asks the already-built `DioramaFrame` for the ordered pick result and dispatches a stable entity selection action. Hidden semantic layers cannot win picks.

- [ ] **Step 5: Run projection and input regressions**

```bash
cargo test --test diorama_camera
cargo test --test workshop_presentation
cargo test --test workshop_ui
cargo fmt --all --check
git diff --check
```

- [ ] **Step 6: Commit camera foundation**

```bash
git add src/presentation/diorama_camera.rs src/presentation/mod.rs src/presentation/workshop.rs src/app.rs src/app/input_router.rs tests/diorama_camera.rs tests/workshop_presentation.rs
git commit -m "feat(graphics): add stable diorama navigation"
```

### Task 5: Render a variable-capacity layered cosmic diorama

**Files:**
- Create: `src/presentation/diorama.rs`
- Create: `src/engine/render_frame.rs`
- Create: `src/engine/diorama_renderer.rs`
- Create: `assets/shaders/diorama_links.wgsl`
- Create: `assets/shaders/diorama_worlds.wgsl`
- Modify: `src/presentation/mod.rs`
- Modify: `src/engine/mod.rs`
- Modify: `src/engine/render.rs`
- Modify: `src/engine/gpu.rs`
- Modify: `src/app.rs`
- Test: `tests/diorama_frame.rs`
- Test: `tests/renderer_contracts.rs`
- Test: `tests/workshop_presentation.rs`

**Interfaces:**
- Consumes: immutable presentation snapshots, `DioramaCamera`, measured canvas, selection/focus, theme, contrast, reduced-motion, and quality tier.
- Produces: variable-length GPU instance/link batches, stable semantic entities, and a renderer scene variant shared by Workshop and Living Galaxy.

- [ ] **Step 1: Write failing frame and renderer-capacity tests**

```rust
#[test]
fn diorama_frame_preserves_more_than_classic_seven_worlds() {
    let frame = DioramaFrame::from_fixture(capacity_fixture(64, 512));
    assert_eq!(frame.systems.len(), 64);
    assert_eq!(frame.worlds.len(), 512);
    assert_eq!(frame.semantic_entities.len(), 576);
}

#[test]
fn empty_and_capacity_frames_produce_bounded_renderer_uploads() {
    for frame in [DioramaFrame::empty(), DioramaFrame::from_fixture(capacity_fixture(64, 512))] {
        let upload = DioramaUpload::prepare(&frame).unwrap();
        assert_eq!(upload.world_instances.len(), frame.worlds.len());
        assert!(upload.byte_len() <= DioramaUpload::MAX_BYTES);
    }
}
```

Add tests for one system/one world, 2048 routes, 256 fleets, stable ordering independent of hash-map insertion, invalid reference diagnostics, high-contrast non-color encodings, reduced-motion frame equality at equal ticks, and WebGL2-aligned buffer layouts.

- [ ] **Step 2: Verify missing renderer types**

```bash
cargo test --test diorama_frame
cargo test --test renderer_contracts diorama -- --nocapture
```

- [ ] **Step 3: Define the immutable shared frame**

```rust
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DioramaFrame {
    pub camera: DioramaCamera,
    pub systems: Vec<SystemVisual>,
    pub worlds: Vec<WorldVisual>,
    pub links: Vec<LinkVisual>,
    pub traffic: Vec<TrafficVisual>,
    pub effects: Vec<EffectVisual>,
    pub semantic_entities: Vec<DioramaSemanticEntity>,
}

pub enum RenderScene<'a> {
    Classic(&'a SceneFrame),
    Diorama(&'a DioramaFrame),
}
```

Sort each collection by explicit layer then stable entity ID. Keep Classic `SceneFrame` and its exact seven-world ABI untouched. `RenderFrame` owns `Option<RenderScene<'a>>`; no later plan may add another renderer scene enum.

- [ ] **Step 4: Implement bounded dynamic GPU buffers and shader variants**

`DioramaRenderer` grows buffers geometrically up to specified capacity, reuses allocations between frames, skips zero-length draws, and reports overflow as a recoverable visible renderer diagnostic. Shaders render layered star glow, world discs/rings, route/freight lines, fleet marks, selection halos, and patterned conflict/hazard regions. WebGPU and WebGL2 paths share identical host layouts and separate compatible shader entry points where required.

Graphics quality changes sampling and decorative density only; it cannot hide identities, ownership, shortages, routes, hazards, conflicts, selection, or errors. Reduced motion freezes decorative twinkle and transitions but keeps authoritative fleet/route positions current.

- [ ] **Step 5: Switch Workshop to the shared diorama without changing its model**

Implement `From<&WorkshopSceneFrame> for DioramaFrame` in `src/presentation/workshop.rs`. Remove the primitive-only Workshop disc/line submission after image/golden coverage demonstrates equivalent semantics. Keep a visible fallback diagnostic when the diorama renderer cannot initialize.

- [ ] **Step 6: Run renderer, wasm, and Workshop regressions**

```bash
cargo test --test diorama_frame
cargo test --test renderer_contracts
cargo test --test workshop_presentation
cargo check --target wasm32-unknown-unknown --lib
cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings
cargo fmt --all --check
git diff --check
```

- [ ] **Step 7: Commit the shared diorama renderer**

```bash
git add src/presentation/diorama.rs src/presentation/workshop.rs src/presentation/mod.rs src/engine/render_frame.rs src/engine/diorama_renderer.rs src/engine/mod.rs src/engine/render.rs src/engine/gpu.rs src/app.rs assets/shaders/diorama_links.wgsl assets/shaders/diorama_worlds.wgsl tests/diorama_frame.rs tests/renderer_contracts.rs tests/workshop_presentation.rs
git commit -m "feat(graphics): render scalable cosmic dioramas"
```

### Task 6: Make all Workshop chrome and inspector content visibly SDF-rendered

**Files:**
- Create: `src/ui/platform_sdf.rs`
- Modify: `src/ui.rs`
- Modify: `src/ui/platform.rs`
- Modify: `src/presentation/ui.rs`
- Modify: `src/app.rs`
- Modify: `src/engine/render.rs`
- Test: `tests/workshop_ui.rs`
- Test: `tests/renderer_contracts.rs`
- Test: `tests/workshop_accessibility.rs`

**Interfaces:**
- Consumes: `PlatformUiFrame`, measured layout, virtualized controls, Inter atlas/metrics, theme, contrast, focus, and validation state.
- Produces: `UiBatch` glyph/quads, bounded primitive decoration batch, and one semantic node per actionable or informative visible item.

- [ ] **Step 1: Write failing visible-content parity tests**

```rust
#[test]
fn every_visible_semantic_label_has_an_sdf_text_run() {
    let frame = populated_workshop_platform_frame();
    let batch = build_platform_ui_batch(&frame, fixture_atlas()).unwrap();
    for node in frame.semantic_nodes.iter().filter(|node| node.visible) {
        if let Some(label) = &node.visible_label {
            assert!(batch.text_runs.iter().any(|run| run.text == *label));
        }
    }
}

#[test]
fn long_labels_clip_or_wrap_inside_their_measured_regions() {
    let frame = narrow_workshop_platform_frame(723.0, 802.0, 1.30);
    let batch = build_platform_ui_batch(&frame, fixture_atlas()).unwrap();
    assert!(batch.text_runs.iter().all(|run| frame.layout.viewport.contains_rect(run.clip)));
    assert!(batch.text_runs.iter().all(|run| run.bounds.intersection(run.clip).is_some()));
}
```

Add parity checks for inspector values, validation errors, status, timeline overflow, hierarchy disclosure, search, save state, fallback warnings, high contrast, 200% scale, and focus/hover/pressed states.

- [ ] **Step 2: Verify the current primitive frame fails parity**

```bash
cargo test --test workshop_ui visible_semantic_label -- --nocapture
cargo test --test renderer_contracts platform_ui -- --nocapture
```

- [ ] **Step 3: Define visible platform primitives**

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct PlatformTextRun {
    pub text: String,
    pub origin: Vec2,
    pub clip: PlatformRect,
    pub style: PlatformTextStyle,
    pub semantic_id: SemanticActionId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlatformPanelVisual {
    pub rect: PlatformRect,
    pub role: PlatformPanelRole,
    pub state: PlatformVisualState,
}

pub fn build_platform_ui_batch(
    frame: &PlatformUiFrame,
    atlas: &SdfAtlas,
) -> Result<UiBatch, PlatformUiBuildError>;
```

Use Inter Regular/SemiBold from the checked-in SDF atlas. Body/metadata/section title sizes are 15/13/18 logical units with approximately 1.35 line height. Geometry comes from Task 1 layout and Task 2 virtualization; no builder may invent parallel rectangles.

- [ ] **Step 4: Install SDF chrome in the renderer**

App builds one `UiBatch` from the platform frame and supplies it to the existing `UiRenderer`. Primitive overlay remains for simple backgrounds, borders, focus rings, and caret/selection geometry. Draw order is diorama, primitive underlay, SDF UI, then bounded primitive overlay. The Workshop canvas remains unoccluded.

- [ ] **Step 5: Run visual-contract and accessibility gates**

```bash
cargo test --test workshop_ui
cargo test --test renderer_contracts
cargo test --test workshop_accessibility
cargo test --test workshop_web
cargo fmt --all --check
git diff --check
```

- [ ] **Step 6: Commit complete visible chrome**

```bash
git add src/ui/platform_sdf.rs src/ui.rs src/ui/platform.rs src/presentation/ui.rs src/app.rs src/engine/render.rs tests/workshop_ui.rs tests/renderer_contracts.rs tests/workshop_accessibility.rs
git commit -m "feat(ui): render complete Workshop chrome"
```

### Task 7: Qualify the foundation across layout, graphics, input, and accessibility

**Files:**
- Create: `tests/living_foundation_acceptance.rs`
- Create: `tools/check-living-foundation.sh`
- Modify: `tools/check-workshop.sh`
- Modify: `docs/PLAYER-MANUAL.md`
- Modify: `README.md`

**Interfaces:**
- Consumes: Tasks 1-6 and existing Workshop fixtures.
- Produces: one reproducible automated foundation gate, browser/native manual procedure, captured viewport matrix, and documentation that distinguishes source evidence from live acceptance.

- [ ] **Step 1: Write the failing acceptance matrix**

```rust
#[test]
fn required_viewport_scale_matrix_keeps_canvas_and_controls_reachable() {
    for (width, height, scale) in [
        (723.0, 802.0, 1.00),
        (723.0, 802.0, 1.15),
        (723.0, 802.0, 1.30),
        (1280.0, 720.0, 1.00),
        (1440.0, 900.0, 1.00),
        (1920.0, 1080.0, 2.00),
    ] {
        let acceptance = FoundationAcceptance::render(populated_fixture(), width, height, scale);
        assert!(acceptance.canvas_is_visible());
        assert!(acceptance.all_required_actions_reachable());
        assert!(acceptance.semantic_geometry_matches_visual_geometry());
        assert!(acceptance.all_targets_at_least(44.0));
    }
}

#[test]
fn classic_vertex_and_rules_goldens_remain_unchanged() {
    assert_eq!(std::mem::size_of::<Vertex>(), 36);
    assert_eq!(default_rules_v1_digest(), 0x67D9_6E98_3D6C_9330);
}
```

Add automated journeys for create system/star/world, edit each field, search first/last of 100 rows, traverse twelve branches, choose an occluded overlapping object, pan/zoom/focus, save/reopen, keyboard-only operation, screen-reader action routing, reduced motion, high contrast, empty fixture, one-world fixture, and capacity fixture.

- [ ] **Step 2: Implement the exact foundation gate**

`tools/check-living-foundation.sh` runs:

```bash
#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo check --target wasm32-unknown-unknown --lib
cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings
./tools/build-web-webgpu.sh
./tools/build-web-webgl.sh
git diff --check
```

It prints a final statement that these commands establish source/build coverage only, not native launch, browser runtime, assistive-technology, performance, or manual visual acceptance.

- [ ] **Step 3: Run the automated gate**

```bash
chmod +x tools/check-living-foundation.sh
./tools/check-living-foundation.sh
```

Expected: every named command passes. Record actual test counts and artifact paths from this run; do not reuse historical totals.

- [ ] **Step 4: Perform live native and browser acceptance**

Launch the exact native artifact and both browser backends. At each required viewport/scale, capture:

1. visible galaxy canvas with Aster Vale, Aster, and Nacre;
2. no opaque full-window UI overlay;
3. unclipped creator labels and values;
4. first and last hierarchy row plus more than four branches;
5. keyboard edit, selection, paste rejection, cancel, and commit;
6. screen-reader names, roles, states, actions, and bounds;
7. high-contrast and reduced-motion behavior;
8. visible WebGPU/WebGL2 fallback status.

Expected: the previously observed blank-canvas and clipped-field defects are absent. Any unavailable browser, platform, or assistive technology remains pending rather than inferred from compilation.

- [ ] **Step 5: Update documentation from observed behavior**

Document actual navigation, editing, search, drawers, graphics quality, contrast, reduced motion, keyboard shortcuts, and backend status in `docs/PLAYER-MANUAL.md` and `README.md`. Do not describe Living civilization features that are not yet implemented.

- [ ] **Step 6: Re-run gates and review only this slice**

```bash
./tools/check-living-foundation.sh
git diff --check
git diff -- tests/living_foundation_acceptance.rs tools/check-living-foundation.sh tools/check-workshop.sh docs/PLAYER-MANUAL.md README.md
```

- [ ] **Step 7: Commit qualification and documentation**

```bash
git add tests/living_foundation_acceptance.rs tools/check-living-foundation.sh tools/check-workshop.sh docs/PLAYER-MANUAL.md README.md
git commit -m "test(ui): qualify the cosmic Workshop foundation"
```

## Foundation Completion Boundary

The foundation is complete only when the measured-layout, virtualization, editing, camera, scalable diorama, SDF parity, and compatibility tests pass and the live native/browser matrix has explicit evidence. It does not by itself establish Living V2 authority correctness, civilization autonomy, durable Living saves, or whole-product completion.
