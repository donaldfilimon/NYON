# Workshop V1 Star-Visual Slice Review — 2026-09-04

## Verdict

**APPROVE FOR SOURCE COMMIT.** No open P0, P1, or P2 finding remains in the reviewed star-visual slice. The two prior P2 proof gaps are closed: asymmetric fixtures now pin the complete metadata word order and independent selection flags, while marker witnesses now couple declared ring thickness to the exact packed shape emitted by the primitive batch and compare normal/high-contrast geometry directly.

## Scope and evidence boundary

- Re-reviewed only `src/presentation/workshop.rs`, `src/ui/platform.rs`, `tests/workshop_presentation.rs`, and `tests/workshop_ui.rs` against baseline Finding 6, the committed Living Galaxy Foundation constraints, and the two findings from the first review. No product file was edited, staged, or committed by this review.
- Source inspection confirms that star instances are `#[repr(C)]`, `Pod`, and `Zeroable`; use a 64-byte, four-lane instance contract; retain the complete 128-bit `EntityId` in `metadata`; place selection in `flags[0]`; and are extracted in stable entity-ID order. The same loop emits corresponding semantic entries, preserving visual/semantic order, center, identity, and selected state.
- Draw inspection confirms the intended order: lanes, system rings, all star halos, all star cores, star selection rings, worlds, shipments, and labels. Selection adds a separate outer ring without changing the star's core or halo color. Both presentation modes use finite, nonzero, luminous opaque cores; high contrast adds larger halo/core/selection radii and thicker system/selection rings.
- `WorkshopMarkerWitness` now records `ring_thickness`, the exact `packed_shape` read from emitted geometry, and the exact vertex span. Tests require all six emitted vertices to match that packed value, decode its upper-width bits, and compare the decoded width to the declared thickness within one quantization unit.
- The slice remains one-way presentation code. Focused tests preserve canonical Workshop bytes and `StateDigest` across extraction and contrast preferences. No Classic seven-world scene ABI, Workshop authority contract, `DioramaFrame`, `RenderScene`, or future Living renderer ownership moved into this repair.
- Fresh focused gates passed: the exact star identity/pairing test (1/1), the exact star layer/contrast/witness test (1/1), `cargo clippy --test workshop_presentation --test workshop_ui -- -D warnings`, `cargo fmt --all --check`, and `git diff --check`.
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

## Verified properties

- Stable star ordering is deterministic: authority storage is ordered, extraction explicitly sorts by `EntityId`, and star visuals plus semantics are emitted together.
- Every star visual has one semantic counterpart with matching identity, center, and selected state.
- Full 128-bit identity and selection occupy independent lanes; reserved flag words remain zero.
- `WorkshopStarInstance` retains explicit size, alignment, field-offset, vertex-format, and stride contracts for its 64-byte host layout.
- Witness order and vertex spans prove system-ring → star-halo → star-core → star-selection-ring → world layering.
- Normal and high-contrast star cores are opaque and luminous; high contrast also changes geometry, not color alone.
- Selecting a star adds exactly one six-vertex outer ring without recoloring its core or halo.
- Presentation preferences do not enter canonical Workshop bytes, tick, or state digest.
- Classic ABI and future Living renderer ownership remain untouched.

## Acceptance recommendation

The star-visual source slice is suitable for inclusion in the reviewed Workshop baseline. Rebuild and exercise the native/WebGPU and WebGL2 artifacts before claiming live pixel visibility, backend parity, or release acceptance.
