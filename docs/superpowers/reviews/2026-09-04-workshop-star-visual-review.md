# Workshop V1 Star-Visual Slice Review — 2026-09-04

## Verdict

**APPROVE FOR SOURCE COMMIT.** No open P0, P1, or P2 finding remains in the reviewed star-visual slice or its Metal primitive-compatibility repair. The two prior P2 proof gaps remain closed. The additional repair routes every filled disc through the already-defined maximum-width kind-2 ring endpoint; source algebra, packing, shared-shader routing, focused tests, and native/wasm checks establish behavior equivalence without changing vertex ABI, authority, Classic, or Living ownership.

## Scope and evidence boundary

- Re-reviewed `src/presentation/workshop.rs`, `src/ui/platform.rs`, `tests/workshop_presentation.rs`, and `tests/workshop_ui.rs` against baseline Finding 6, the committed Living Galaxy Foundation constraints, and the two findings from the first review. The Metal compatibility addendum additionally reviewed `assets/shaders/primitives.wgsl`, `src/engine/primitives.rs`, and `tests/shaders.rs`, plus the updated Workshop witness assertions. No product file was edited, staged, or committed by this review.
- Source inspection confirms that star instances are `#[repr(C)]`, `Pod`, and `Zeroable`; use a 64-byte, four-lane instance contract; retain the complete 128-bit `EntityId` in `metadata`; place selection in `flags[0]`; and are extracted in stable entity-ID order. The same loop emits corresponding semantic entries, preserving visual/semantic order, center, identity, and selected state.
- Draw inspection confirms the intended order: lanes, system rings, all star halos, all star cores, star selection rings, worlds, shipments, and labels. Selection adds a separate outer ring without changing the star's core or halo color. Both presentation modes use finite, nonzero, luminous opaque cores; high contrast adds larger halo/core/selection radii and thicker system/selection rings.
- `WorkshopMarkerWitness` now records `ring_thickness`, the exact `packed_shape` read from emitted geometry, and the exact vertex span. Tests require all six emitted vertices to match that packed value, decode its upper-width bits, and compare the decoded width to the declared thickness within one quantization unit.
- The slice remains one-way presentation code. Focused tests preserve canonical Workshop bytes and `StateDigest` across extraction and contrast preferences. No Classic seven-world scene ABI, Workshop authority contract, `DioramaFrame`, `RenderScene`, or future Living renderer ownership moved into this repair.
- Fresh star gates passed: the exact star identity/pairing test (1/1), the exact star layer/contrast/witness test (1/1), focused Clippy with warnings denied, formatting, and diff checks.
- Fresh compatibility gates passed: `cargo test --lib engine::primitives` (7/7), `cargo test --test shaders` (9/9), the exact Workshop star layer/contrast/witness test (1/1), `cargo check --target wasm32-unknown-unknown --lib --no-default-features --features webgpu-backend`, the equivalent `webgl-backend` check, `cargo clippy --test shaders --test workshop_ui -- -D warnings`, `cargo fmt --all --check`, and `git diff --check`.
- This approval covers current source and CPU-generated primitive evidence only. No native window, WebGPU artifact, WebGL2 artifact, shader draw, pixel readback, screenshot comparison, browser interaction, or live visual acceptance was performed. Pixel visibility and backend parity remain separate acceptance gates after current artifacts are rebuilt.

## Findings

No open P0-P2 findings.

### 1. [P2] Exact entity-identity proof used symmetric fixture IDs

- **Severity:** P2
- **File:line:** `tests/workshop_presentation.rs:18-30`, `tests/workshop_presentation.rs:211-257`, `src/presentation/workshop.rs:476-483`
- **Description:** The initial test used `EntityId([value; 16])`, making every 32-bit word identical and allowing duplicated or reordered metadata words to pass. The repair introduces two asymmetric IDs with distinct bytes and words, asserts the exact four little-endian metadata words, reconstructs both original IDs, and independently asserts complete selected and unselected flag lanes.
- **Suggestion:** No further source change required. Retain the asymmetric fixtures and exact word/flag assertions as ABI regression coverage.
- **Status:** Resolved — the test now fails for duplicated chunks, reordered words, incorrect byte order, metadata/flag overlap, or incorrect selected-state encoding.

### 2. [P2] High-contrast geometry and packed ring width were not discriminatingly witnessed

- **Severity:** P2
- **File:line:** `src/ui/platform.rs:1157-1167`, `src/ui/platform.rs:1217-1235`, `src/ui/platform.rs:1248-1302`, `tests/workshop_ui.rs:1289-1434`, `tests/workshop_ui.rs:1450-1497`
- **Description:** The initial test exercised both modes independently but never compared them, and its witness masked packed ring shapes down to the low-byte kind. The repair retains normal and high-contrast witness sets, pairs them by layer and source index, requires larger high-contrast halo/core/selection radii, requires thicker high-contrast system/selection rings, and requires distinct packed ring shapes. Each witness is tied to all six emitted vertices; packed width is decoded and matched to the declared thickness within the encoder's quantization tolerance. Opaque core alpha and core luminance are checked separately from the decorative translucent halo.
- **Suggestion:** No further source change required. Preserve the cross-mode comparisons and packed-shape decoding when primitive encoding or contrast policy changes.
- **Status:** Resolved — removing the non-color contrast geometry, changing its direction, breaking witness/vertex coupling, or corrupting packed thickness now fails the focused test.

## Metal compatibility addendum

### Full-width kind-2 rings are source-equivalent to the former kind-1 filled discs

- **Severity:** None
- **File:line:** `src/engine/primitives.rs:3-6`, `src/engine/primitives.rs:52-67`, `src/engine/primitives.rs:182-190`, `assets/shaders/primitives.wgsl:39-78`, `tests/shaders.rs:12-16`, `tests/shaders.rs:65-101`, `tests/workshop_ui.rs:1473-1508`
- **Description:** For every positive finite radius, `disc` still emits the same six vertices, positions, local coordinates, color, and radius-sized quad. The only changed host value is `shape`: `packed_ring_shape(radius, radius)` computes a width ratio of exactly 1, rounds it to the 24-bit maximum `0x00ff_ffff`, shifts it into bits 8-31, and retains kind 2 in bits 0-7, producing `0xffff_ff02`. WGSL decodes those lanes as kind 2 and maximum width, then assigns `outer_coverage`, exactly the same value assigned by the former kind-1 branch. Alpha, discard behavior, unpremultiplied RGB, and straight-alpha pipeline blending are unchanged. A zero or negative finite radius remains a zero-area quad; nonfinite radii remain rejected. The shared WGSL is the primitive shader used by native Metal and the WebGPU/WebGL2 feature builds, so no backend-specific host ABI or shader variant diverges.
- **Suggestion:** No further source change required. Retain the exact packed-word test, WGSL endpoint/derivative validation, updated all-kind-2 Workshop witness assertion, and separate live backend acceptance boundary. If the dedicated kind-1 branch is removed later, treat that as its own compatibility cleanup because public `Vertex` construction can still encode it today.
- **Status:** Approved for source commit — mathematical equivalence and host/shader packing are established; runtime backend translation and pixels remain acceptance work.

### Compatibility tests are focused and non-vacuous for the changed contract

- **Severity:** None
- **File:line:** `src/engine/primitives.rs:326-366`, `tests/shaders.rs:65-101`, `tests/workshop_ui.rs:1473-1496`
- **Description:** The primitive unit test requires all six disc vertices to carry the exact maximum-width kind-2 word. The shader test validates WGSL with Naga, pins width-mask decoding and both endpoint branches after derivative evaluation, and couples a real `PrimitiveBatch::disc` result to the full-width endpoint literal. Workshop witness coverage now requires system rings, star halos, star cores, selection rings, and worlds all to use kind 2; it binds each witness to the exact packed shape of all six emitted vertices and decodes the upper width bits back to declared thickness within one quantization unit. Reverting `disc` to kind 1, mispacking the endpoint, changing the Workshop witness expectation, or losing the shader endpoint makes a focused gate fail.
- **Suggestion:** No further source change required for this repair. Backend shader creation and rendered-pixel tests would strengthen qualification but are correctly treated as acceptance evidence rather than implied by string/source validation.
- **Status:** Approved for source commit.

## Verified properties

- Stable star ordering is deterministic: authority storage is ordered, extraction explicitly sorts by `EntityId`, and star visuals plus semantics are emitted together.
- Every star visual has one semantic counterpart with matching identity, center, and selected state.
- Full 128-bit identity and selection occupy independent lanes; reserved flag words remain zero.
- `WorkshopStarInstance` retains explicit size, alignment, field-offset, vertex-format, and stride contracts for its 64-byte host layout.
- Witness order and vertex spans prove system-ring → star-halo → star-core → star-selection-ring → world layering.
- Normal and high-contrast star cores are opaque and luminous; high contrast also changes geometry, not color alone.
- Selecting a star adds exactly one six-vertex outer ring without recoloring its core or halo.
- Filled discs preserve their former analytic coverage while using one maximum-width kind-2 path across the shared primitive shader.
- The `Vertex` `#[repr(C)]` layout, 36-byte stride, attribute offsets/formats, and `u32` shape lane are unchanged; only disc shape values change from kind 1 to the reviewed `0xffff_ff02` endpoint.
- Presentation preferences do not enter canonical Workshop bytes, tick, or state digest.
- Classic ABI and future Living renderer ownership remain untouched.

## Acceptance recommendation

The star-visual source slice and full-width-ring Metal compatibility repair are suitable for inclusion in the reviewed Workshop baseline. Rebuild and exercise the native/Metal, browser WebGPU, and browser WebGL2 artifacts before claiming live pixel visibility, actual backend parity, or release acceptance.
