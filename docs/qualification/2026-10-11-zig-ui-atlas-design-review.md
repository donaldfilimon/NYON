## Design Document Review: NYON owned Zig UI atlas design

### Summary

**Needs revision — two major (Important) issues block an implementation-ready verdict.** The owned Zig V2 direction, bounded source profile, exact advance preservation and qualification/retirement boundaries are sound; durable publication ordering and the binding between generator source identity and the running executable need explicit contracts. Two minor clarifications can be resolved in the same revision. No critical issue was found.

Reviewed specification: `docs/superpowers/specs/2026-10-11-nyon-zig-ui-atlas-design.md`, SHA-256 `7ba06e776f509ced71c416b6ffec3208fd40b4380dfa4389330faee1180c40fa`. Read in full. This review approves neither implementation nor asset promotion.

### Issue 1: Make the publication durability barriers and commit outcome explicit

- **Severity**: major
- **Section**: Transactional publication and check mode, lines 140–144; cancellation at line 90
- **Description**: The protocol says “sync each file and its directory … then rename” the staging directory, followed by “writing and syncing a sibling temporary selection file and atomically replacing the selection file.” It does not explicitly require syncing the candidate directory's parent **after** its rename and **before** publishing the selection, or syncing the selection's parent **after** replacement. Flushing the staging directory before rename does not durably publish its new name. Consequently the claim “after selection it references a complete validated directory” is not established for a system crash: the selection can survive while the candidate's new directory entry does not. Linux's documented file-sync semantics require a separate containing-directory sync for directory-entry durability ([Linux fsync(2)](https://man7.org/linux/man-pages/man2/fsync.2.html)). The document also promises cancellation between publication steps leaves selection unchanged, but cancellation or an IO failure after the selection rename has already crossed the visible commit point cannot guarantee that outcome.
- **Suggestion**: Specify the ordered barriers: sync every artifact and release receipt; sync the staging directory; rename to the immutable candidate directory; durably sync the affected parent directory entries; only then write/sync and atomically replace selection; finally sync the selection parent before reporting durable success. Cover newly created ancestors and existing candidate directories whose prior publication may have been interrupted. Name the commit point and distinguish pre-commit failure/cancellation from post-replace durability-unknown failure; stop honoring cancellation as an unchanged-selection outcome once commit begins. Require platform-specific equivalent semantics or explicit unsupported release promotion, and fault tests for each barrier and the post-replace failure outcome. This is a protocol requirement; macOS/Windows/Linux filesystem behavior remains untested here.
- **Status**: open

### Issue 2: Bind the running generator to its claimed source and compiler identity

- **Severity**: major
- **Section**: Packing, metrics and identity, lines 134–136; Proposed exact atlas gate, lines 176–183
- **Description**: The manifest includes the “sorted generator-input digest,” while receipts carry “generator executable identity … separately.” Freshness recomputes on-disk tool-source identity, but no stated invariant connects those bytes to the executable doing the generation. A generator built at source A can be run directly after the checkout is edited to B; it can hash B, generate using A, and emit a manifest claiming B. Running `--check` with that same old executable can reproduce the same misattribution. Separately recording its executable hash makes the run identifiable but does not prove it implements B. The proposed gate's preceding build reduces the normal-path risk, but it does not define the direct CLI admission rule or prevent a source change between compilation and invocation. This undermines the asserted invalidation of old candidates/receipts after generator-source changes.
- **Suggestion**: Embed a canonical build-time digest of the exact compiled generator source closure, profile/configuration and compiler identity in the executable, and require it to equal a freshly validated source snapshot before generation/check/admission. Alternatively require an independently validated immutable build receipt binding that digest to the actual executable hash. Keep host-specific binary hashes outside deterministic candidate bytes. Define handling of edits during compilation/generation and add regressions: build A, edit a generator module/profile/build input to B, invoke A directly, and require typed stale-tool failure without candidate publication or selection changes. A clean rebuild of B must produce the appropriate new identity even if its pixels happen to be unchanged.
- **Status**: open

### Issue 3: Freeze fixed-point evaluation order and primitive normalization before fixtures

- **Severity**: minor
- **Section**: SVG parser and geometry subset, lines 114–116; Deterministic rasterization and SDF, lines 120–124
- **Description**: Rounding and tolerances are specified, but expression evaluation order is not completely pinned. For the bundled `wind.svg` operand `12.8`, converting the operand to Q32.32 then rounding `x*5/3` gives raw `91625968982`; multiplying it by a separately rounded Q32.32 `5/3` gives `91625968986`. Both are plausible readings of “scale 5/3” with the stated rounding. Full-circle initial points/direction and the exact scaling/order for arc-center division and square root are likewise left implicit. This is not evidence of a current pixel defect; it makes independent fixtures harder to author before the implementation and leaves unnecessary profile choices to its author.
- **Suggestion**: Add a compact numeric annex with raw-integer formulas and rounding points for coordinate scaling, circle initialization, corrected arc radii/centers and midpoint normalization. Specify zero-length/near-zero denominators and range checks. Freeze those intermediate values in independently derived fixtures, including bundled decimal operands; do not use the first implementation's outputs as the oracle.
- **Status**: open

### Issue 4: State output-record path semantics to preserve the acyclic identity graph

- **Severity**: minor
- **Section**: Packing, metrics and identity, line 134; Transactional publication and check mode, line 140
- **Description**: V2 `hashes.json` retains output file records, but their `path` values are not specified. The existing builder records repository-relative output paths (`tools/build-ui-atlas.rs:414`). Substituting V2's complete publication path would place `<candidate-id>` inside `hashes.json`, which is hashed by the manifest that defines the candidate ID. That recreates a cycle despite the intended directed graph.
- **Suggestion**: State that generated payload paths are fixed artifact-relative names such as `atlas.r8` and `atlas-metrics.json`, with source records separately repository-relative. Forbid candidate IDs, publication roots and receipt paths in generated output descriptors. Reuse the platform plan's explicit artifact-relative path convention and add a relocation-stability fixture.
- **Status**: open

### Strengths

- The design explicitly selects fully owned Zig parsing/rasterization. It does not inherit the earlier platform-raster alternative, introduce a new production dependency, or treat the no-Rust mandate as permission to retire unqualified behavior.
- Current architecture claims match inspected source: the separate Cargo workspace and versions, sequential legacy writes, fixed 205-entry packing, threshold/SDF formula, retained metadata-only V1 loader and missing pixel-hash admission. Evidence: `tools/ui-atlas/Cargo.toml`, `tools/ui-atlas/src/main.rs`, `tools/build-ui-atlas.rs:135`, `tools/build-ui-atlas.rs:177`, `tools/build-ui-atlas.rs:322`, `zig/src/retained_atlas.zig:48`, `zig/src/retained_atlas.zig:70`, and `zig/src/retained_atlas.zig:198`.
- Exact layout preservation is credible: an independent read-only byte audit revalidated all 21 manifest file hashes and decoded both fonts' Windows format-4 ASCII mappings and `hmtx` values. All 190 calculated advances match the committed JSON when interpreted as f32. Both fonts have 2,048 units/em, 2,937 glyphs/full horizontal metrics and the seven listed composite ASCII characters. This verifies input/metric claims, not generated coverage.
- All fifteen SVG files were inspected; the stated element/command/circular-arc/round-stroke profile covers their current contents. The specification includes malformed input, component cycles, allocation failures, bounded flattening and checked wide arithmetic instead of relying solely on trusted-input hashes.
- V2 coverage changes are honestly versioned, while cell/origin/advance contracts remain frozen. The independent primitive fixtures, all-entry pixel reports, typography/layout cases and actual backend qualification prevent whole-atlas goldens from becoming their own correctness oracle. Existing clipping, atomic emission and capacity contracts are present in `tests/ui_assets.rs`; baseline/cell placement is visible in `src/ui.rs:531`.
- The selected-pointer/receipt graph separates candidate generation from all-target promotion, and the required visual matrix includes native and browser fallback paths. Missing target witnesses remain unverified. Original fonts, Inter OFL and Lucide ISC/Feather MIT notices stay intact.

### Verification scope

**Current:** source inspection and the independent read-only Python hash/font/advance audit completed successfully (audit exit 0). A separate exact-rational arithmetic probe produced the two scaling values above (exit 0). These probes are review evidence only and add no Python dependency to the proposed delivered graph. Reviewed the relevant October 9 platform/atlas contracts and the current Zig pin/build/check script. The full runtime, cross-host generator outputs, performance budget, filesystems and actual glyph pixels remain **unverified**. Remote font/icon archives were not downloaded or revalidated; their records are provenance metadata only.

No generator, Cargo gate, Zig game gate, renderer or target qualification was run for this documentation-only review. No source, assets, dependencies, AGENTS files, ledger, commits or other design documents were changed. Only this new review file was authored. The specification hash was rechecked at completion and remained the reviewed hash above.
