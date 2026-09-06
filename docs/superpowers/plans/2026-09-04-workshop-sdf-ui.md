# Workshop V1 SDF Typography and Icon Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development task-by-task, with independent review and exact evidence.

**Status:** Independently approved for implementation on 2026-09-04; source work waits for inspector approval

**Goal:** Replace Workshop's bitmap-text/primitive-chrome presentation with the existing offline SDF text and icon renderer while preserving the accepted responsive layout, semantic identities, input geometry, authority separation, and Metal/WebGPU/WebGL2 shader ABI.

**Architecture:** The accepted Workshop model and inspector produce one immutable platform frame keyed by stable `SemanticNodeId`s. A new pure platform-to-SDF bridge converts that frame into panels, glyphs, and typed icon witnesses. Native AccessKit and browser DOM continue to consume the same semantic frame. The final primitive overlay is reduced to bounded keyboard focus, caret/selection, and emergency diagnostic decoration.

**Tech Stack:** Pinned Rust 2024, existing wgpu SDF renderer, bundled Inter/Lucide atlas, AccessKit and browser semantic mirror.

**Spec:** `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-experience.md`, typography and shared-layout requirements, applied to the existing Workshop V1 compatibility surface.

**Prerequisite:** Begin source work only after the sighted-inspector slice is independently approved. That slice owns the final inspector records, header/body/footer geometry, clipping, virtualization, and reveal behavior consumed here.

## Frozen boundaries

- Use the checked-in Inter 4.1 Regular/SemiBold fonts and pinned Lucide subset under `assets/ui/`.
- No network, runtime filesystem font loading, icon package, new renderer backend, or backend-specific glyph path.
- Preserve `UiGlyphInstance` at 48 bytes, `UiPanelInstance` at 32 bytes, existing vertex locations, `MAX_UI_GLYPHS = 8_192`, and `MAX_UI_PANELS = 512`.
- Preserve `assets/shaders/ui_sdf.wgsl`, `src/engine/render.rs`, `src/engine/ui_renderer.rs`, and `src/engine/render_frame.rs` unless a focused RED test proves the current ABI cannot implement the required frame.
- Preserve render order: scene → primitive underlay → SDF panels/glyphs → bounded primitive overlay.
- Preserve Classic presentation and RulesV1 behavior. Workshop does not claim Living Diorama ownership.
- Convert all `PlatformUiFrame` shell and Workshop chrome using the shared installer. Classic remains on its existing path. The Player Guide's separate paginated primitive body is explicitly excluded; its platform chrome is included.
- Keep printable ASCII as the V1 atlas contract. Unsupported input yields a bounded presentation fallback without partially publishing a frame.
- Current Workshop scale remains the persisted 85/100/115/130 percent contract. The Living plan's 200-percent requirement remains a separately authorized Foundation task.

## Task 1: Add exact atlas measurement and atomic clipping

**Files:**

- Modify `src/ui.rs`
- Modify `tests/ui_assets.rs`

### Step 1: Write RED measurement tests

Add exact tests proving:

- `WWW` and `iii` do not measure to the same width;
- Regular and SemiBold resolve their correct atlas entries;
- measured advance equals the advance returned by actual emission;
- sentence-case printable ASCII is preserved without uppercase transformation;
- unsupported input and nonfinite geometry reject before mutating the destination batch.

### Step 2: Add one glyph-walk measurement path

Introduce a `TextMetrics` value and `AtlasMetrics::measure_text`. One internal glyph walk must drive both measurement and emission so advances cannot diverge. Resolve validated atlas entries without allocating a formatted key or linearly scanning all entries per glyph.

### Step 3: Write RED geometry/UV clipping tests

Cover left, right, top, and bottom partial clipping. Every partial crop must adjust both destination rectangle and UV rectangle proportionally. Fully clipped cells emit nothing but still contribute measured advance. Icon behavior must match glyph behavior.

### Step 4: Implement atomic clipped emission

Add `UiBatch::push_text_clipped` and `push_icon_clipped`. Build each request in temporary bounded storage, validate complete capacity and geometry, then append atomically. Keep existing Classic helpers behavior-compatible by delegating to the shared internal path with an unbounded clip.

Represent no clipping with `None`, never infinite geometry; `Some(rect)` must be finite and non-inverted. Zero-area clips emit nothing. Distinguish logical text advance from visible glyph-cell ink: spaces retain advance but emit no ink, and the padded atlas cell may exceed its advance. Crop emitted destination and UV rectangles together. Cover empty text, spaces, all four edges, exact-edge contact, multiline layout, zero-area clips, invalid geometry, missing glyphs, and prepopulated destinations. Any error leaves the destination byte-for-byte unchanged.

### Step 5: Freeze ABI and capacity

Retain exact instance sizes/offsets and add negative tests showing capacity failure leaves an existing batch byte-for-byte unchanged.

Test glyph and panel counts at cap-1, cap, and cap+1, including nonempty destination batches. Fully clipped glyphs and spaces must not consume emitted-instance capacity.

### Gate

```bash
cargo test --test ui_assets
cargo clippy --test ui_assets -- -D warnings
cargo fmt --all -- --check
```

## Task 2: Define typed SDF presentation records

**Files:**

- Create `src/ui/platform_sdf.rs`
- Modify `src/ui.rs`
- Modify `src/ui/platform.rs`
- Modify `tests/workshop_ui.rs`

### Step 1: Add typed roles and witnesses

Define:

- `PlatformVisibleNodeRecord`: source-assigned semantic ID, optional action ID, exact display text, role, overflow policy, bounds, optional clip, and state;
- `PlatformTextRole`: Control, Body, Metadata, SectionTitle, Status, Alert;
- `PlatformTextOverflow`: SingleLineEllipsis or Wrap;
- `PlatformTextStyle`: weight, scaled font size, line height, role, overflow;
- `PlatformTextRun`: stable semantic ID, exact text, bounds, clip, style;
- `PlatformIconRun`: stable semantic ID, typed `UiIcon`, bounds, clip;
- `PlatformPanelRole`: FullSurface, PersistentChrome, ControlFill, InspectorRowFill, Drawer, Modal, Scrim, ContrastBorder;
- `PlatformPanelWitness`: role, bounds, optional owning node, and emitted panel range;
- `PlatformSdfOutput`: `UiBatch`, text/icon witnesses, and typed panel witnesses.

These records are presentation witnesses only. They do not enter canonical Workshop bytes, state digest, history, store data, or semantic action routing.

Task 2 review reconciliation: `FullSurface` is restricted to the shell/Guide's existing `PlatformBackground::Full` surface; it is never Workshop persistent chrome. `InspectorRowFill` carries the materialized row's stable owner identity and bounds. Neither role weakens the canvas exclusion for `PersistentChrome`.

Create visible-node records at typed frame/control construction, not by matching final strings or inventing IDs in the renderer. Include heading, status, save state, validation, removal explanation, empty state, diagnostics, controls, and inspector facts. Informative roles are heading, text/status/alert, controls with names/values, and labeled groups; structural containers without informative content need no glyph witness. Derive/validate semantic content and SDF runs from these records; migrate bare title/status strings and control labels into this common path.

### Step 2: Add typed control icon mapping

Add a typed optional icon to `PlatformControl`, assigned by its typed constructor. Do not parse action-ID strings or display text.

Use only the current frozen atlas subset:

- Play and Pause for simulation state;
- Speed for Step/1x/4x/20x;
- Save for save;
- Load for Continue/Library entry;
- Check for Apply/Confirm;
- Close for Cancel/close/return;
- Reset only for reset framing, not undo;
- Settings and Help for their named controls;
- Crosshair or Zoom for create/select/focus only where the meaning is unambiguous.

Icons supplement labels. Error, save, destructive, and simulation status never become icon-only. The 44-unit Medium rail may use a typed icon with the full semantic name and persistent focus/help label.

Undo, Redo, Previous, Next, and Menu remain label-only where no semantically matching bundled icon exists. Reserve actual bounded space for the Medium rail's persistent focus/help label and test its sighted/semantic geometry; a semantic name alone is not a visible label.

### Step 3: Build platform panels and runs atomically

Implement `build_platform_ui_batch(frame, metrics)` in a fresh local batch:

1. background and chrome panels;
2. control and inspector-row fills;
3. icons;
4. Inter text.

Publish only after complete success. Intersect every run's accepted record clip with the viewport. Persistent chrome may not cover the canvas. Exactly one active drawer may cover only its accepted drawer sheet; inactive drawers emit no panel. Modal and scrim overlap is permitted only through their typed roles and accepted modal geometry. Control fills remain within their owning control. Omit offscreen virtualized rows entirely.

Keep the existing renderer's panel pass followed by glyph pass. While a modal is active, suppress covered nonmodal control fills, icons, and text rather than allowing them to repaint above its scrim/modal. Preserve underlying client/model state and make modal content the sole interaction scope. Do not introduce a new renderer pass or change the instance ABI for this repair.

### Step 4: Implement measured wrap and ellipsis

Use exact atlas advances. Single-line controls use measured `...`; details use measured wrapping and can break long unspaced tokens at glyph boundaries. Titles and primary actions remain fixed while accepted panel bodies scroll.

Typography before UI scale:

- Body: 15, Regular;
- Metadata: 13, Regular;
- Section title: 18, SemiBold;
- Control: 13–15, normally SemiBold;
- Line height: approximately 1.35 times the resolved font size.

### Step 5: Write parity and bounds tests

Prove:

- every visible informative semantic node has sighted text with the same `SemanticNodeId`;
- every visible action has an exact label or explicitly recognized typed icon witness;
- emitted text/icon ink after cropping is contained by clip and semantic geometry; logical advance remains independently correct;
- long names ellipsize by measured width;
- complete inspector details wrap without losing decisive content;
- no offscreen row emits glyphs or accepts input;
- reveal materializes the same identity in SDF and semantics;
- normal/high-contrast and reduced-motion modes preserve exact text/identity.

### Gate

```bash
cargo test --test ui_assets --test workshop_ui --test workshop_accessibility
cargo clippy --test ui_assets --test workshop_ui --test workshop_accessibility -- -D warnings
cargo fmt --all -- --check
```

## Task 3: Integrate the SDF batch and shrink primitive overlay

**Files:**

- Modify `src/app.rs`
- Modify `src/ui/platform.rs`
- Modify `tests/renderer_contracts.rs`
- Modify `tests/workshop_web.rs`

### Step 1: Write RED integration tests

Require a nonempty Workshop `UiBatch` and typed proof of layer order:

```text
scene < primitive underlay < SDF < primitive overlay
```

Also require the final Workshop primitive overlay to contain only permitted focus/caret/fallback decoration classes, not ordinary panels, controls, or text.

Introduce `PlatformDecorationKind` and `PlatformDecorationWitness` with bounds, owning ID where applicable, and exact vertex spans for the existing non-indexed batch. `PlatformPrimitiveOverlay` contains the batch and witnesses. Every emitted primitive span must belong to one permitted witness; reject unowned, overlapping, out-of-range, or forbidden decoration spans. High-contrast panel/control borders belong to SDF `ContrastBorder` panels; keyboard focus remains a bounded primitive decoration.

### Step 2: Split platform drawing responsibility

Replace the all-primitive `PlatformUiFrame::draw` responsibility with:

- SDF panel/text/icon composition through `build_platform_ui_batch`;
- `append_primitive_overlay` for static rectangular focus, caret/selection, and bounded fallback only.

Rectangular controls receive rectangular focus geometry built from four lines, not circular rings.

### Step 3: Install atomically in App

`install_platform_frame` builds the SDF output and validated typed primitive overlay against the current embedded metrics in temporary storage. On success it installs both together, synchronizes semantics, and retains the immutable frame. Never publish one successful layer with a stale/failed other layer.

On failure it:

- clears the unfinished SDF batch;
- leaves authoritative Workshop/session/store state unchanged;
- leaves controls semantically accessible;
- logs detailed internal context without imported data or raw untrusted strings;
- draws only a bounded known-ASCII code such as `UI TEXT UNAVAILABLE: CAPACITY` through the primitive fallback.

It must never restore the old full-frame primitive chrome as a fallback.

### Step 4: Preserve shell and guide boundaries

The shared installer converts all platform shell and Workshop chrome; add main menu, Settings, Credits, recovery, and Guide-chrome regressions. The paginated Player Guide body remains a separate primitive presentation slice and is excluded from the Workshop-only primitive restriction. Test that its body is still readable and not covered by converted chrome. Classic stays unchanged.

### Step 5: Preserve browser semantics

Prove DOM semantics keep stable node/action IDs, canvas association, visibility, and `tabindex`; no runtime font/icon URL or network request is introduced; loader backend selection is unchanged.

### Gate

```bash
cargo test --test renderer_contracts --test workshop_web --test workshop_ui --test workshop_accessibility --test ui_assets --test shaders
cargo clippy --test renderer_contracts --test workshop_web --test workshop_ui --test workshop_accessibility --test ui_assets -- -D warnings
cargo fmt --all -- --check
git diff --check
```

## Task 4: Qualify responsive, contrast, and capacity behavior

**Files:**

- Modify `tests/workshop_ui.rs`
- Modify `tests/workshop_accessibility.rs`
- Modify `tests/renderer_contracts.rs`

### Step 1: Freeze the viewport matrix

Exercise:

- 723×802 at 100, 115, and 130 percent;
- 1280×480 at 100 and 130 percent;
- 1280×720 at 100 and 130 percent;
- 1440×900 at 100 and 130 percent;
- 1920×1080 at 100 and 130 percent.

At each size, no emitted ink or panel leaves its clip/viewport. Persistent chrome never covers the accepted canvas; only the typed active drawer/modal/scrim exceptions may overlap it. Every fixed title/primary action remains visible at short heights.

### Step 2: Freeze state coverage

Cover shell, empty/populated Workshop, capacity fixture, status, save state, backend fallback, creator validation, removal blockers, branch state, timeline overflow, long names/errors, complete inspector sections, drawers, and active modal.

At 1280×480 with 100% and 130% scale, cross both normal/high contrast with Navigator, Creator, modal, removal and validation states. Use a validated maximum 512-byte description; verify first and last body content, persistent header, footer preferences/remove/scroll actions, cross-frame stable IDs, and zero text/control intersections. Reduced-motion variants retain these bounds and identities.

### Step 3: Freeze accessibility and non-color cues

- every SDF witness maps to the node delivered to AccessKit/browser DOM;
- hidden virtual rows are absent from SDF and noninteractive in adapters;
- focused/revealed rows appear in both paths;
- modal content is the sole interactive scope;
- high contrast adds static outline/check/border cues rather than hue alone;
- reduced motion changes no static text, icon, selection, or focus geometry.

### Step 4: Freeze capacity

The maximum visible Workshop frame must remain below 8,192 glyphs and 512 panels. Any overflow produces the atomic bounded fallback without semantic or authority loss.

## Task 5: Independent review and complete gates

Use a fresh independent reviewer. Resolve every P0–P2 before acceptance.

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
cargo check --target wasm32-unknown-unknown --lib
cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings
cargo fmt --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all-targets -- -D warnings
cargo run --release --manifest-path tools/ui-atlas/Cargo.toml --locked -- --check
git diff --check
```

## Task 6: Rebuild and perform live acceptance

### Native Metal

- build a fresh release binary;
- package/launch only that exact binary;
- exercise Wide, Medium, Compact, 1280×480, 723×802 at 115/130 percent, normal/high contrast, reduced motion, long text, inspector scroll/reveal, creator dialog, and focus;
- verify Inter glyphs and typed icons are visible and primitive bitmap text is absent from ordinary Workshop chrome.

### Browser WebGPU and forced WebGL2

- rebuild both artifacts;
- run `./tools/check-workshop.sh`;
- exercise the same viewport/state matrix on natural WebGPU and forced `?backend=webgl2`;
- inspect backend label, first-frame result, console warnings/errors, semantic DOM, hidden rows, focus, and lack of font/icon network requests;
- if optional WebGPU High falls back, qualify only the observed Low path.

### Evidence boundary

Record source/tests, artifacts, native Metal, browser WebGPU, browser WebGL2, and real assistive-technology/manual evidence separately. Pixel presence does not prove semantic parity; semantic tests do not prove pixels; local in-app browser evidence does not substitute for the named release browser/platform matrix.

Capacity/error fallback is mandatory automated evidence. Do not claim live fallback unless it occurs naturally or a separately scoped, reversible test-only injection exercises the actual installer; do not force corrupted user data or add a production failure switch solely for a screenshot.
