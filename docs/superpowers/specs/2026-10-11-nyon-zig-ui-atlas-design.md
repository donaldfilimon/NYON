# NYON owned Zig UI atlas design

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Historical; Zig continuation canceled 2026-10-19.** Preserve this source contract and evidence. Active recovery targets Rust stable1.99.1; see the 2026-10-19 Rust designs/plans. Historical review labels do not approve new Rust behavior. Tests and CI gates remain paused.
<!-- END 2026-10-19 rewrite decision -->


- Author: Abbey / Donald Filimon
- Date: 2026-10-11
- Status: Draft — proposed implementation contract; source inventory verified locally, generator and visual acceptance unimplemented.
- Compiler: exactly `0.18.0-dev.146+35accc06e`, as pinned by `zig/.zigversion`.
- Scope: owned offline atlas generation, validation and retained-UI asset admission only.

## Overview

Replace the isolated Rust atlas builder with project-owned Zig parsing, geometry, rasterization, signed-distance conversion, serialization and verification. Generation must run offline from the committed Inter and Lucide sources without Rust, platform font services, foreign raster libraries, Python, JavaScript or shell subprocesses. Small platform adapters elsewhere in the game remain confined to their approved platform responsibilities; atlas generation requires none.

Select a new asset identity, `nyon-ui-atlas-v2`, retaining the existing 205 semantic entries, source fonts/icons, placements, advances and R8 SDF interpretation. Exact fontdue/resvg pixel equivalence is not promised. V1 stays immutable as migration evidence while V2 obtains independent numeric/geometry fixtures, repeatable generation and actual target visual/layout qualification. This specification supersedes only the platform-raster recommendation and V1-only generator compatibility strategy in the October 9 platform specification and atlas plan. Their candidate → target witness → release promotion separation remains required. The approved October 11 no-Rust direction and delegated design authority determine the replacement; they do not turn missing implementation or visual evidence into a pass.

## Background and verified current state

`tools/ui-atlas/src/main.rs` includes `tools/build-ui-atlas.rs`. The separate Cargo workspace pins fontdue 0.9.3, resvg 0.45.1, serde 1.0.219, serde_json 1.0.143 and sha2 0.10.9. `build`, `blit_alpha`, `signed_distance`, `cell_bounds`, `hash_manifest` and `check_output` are the behavior references. Existing writes publish the three files sequentially; the new tool must not inherit that partial-publication behavior.

Current measured identities, from local SHA-256 and byte inspection on 2026-10-11:

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `assets/ui/atlas.r8` | 1,048,576 | `23d4d4f1e7429024957dc17c40c84f8ac89c81f0c5ed7254a88014fe7b102c80` |
| `assets/ui/atlas-metrics.json` | 53,288 | `e18f5b93a7e4e968afb028a3f7b27c92e1fe8667d2ae440c5fda41799e9416bb` |
| `assets/ui/hashes.json` | 4221 | `a6456a670ec2a81bd542647418b0978301852016d024e2a438e73bb2a461ecad` |
| `assets/ui/fonts/Inter-Regular.ttf` | 411,640 | `40d692fce188e4471e2b3cba937be967878f631ad3ebbbdcd587687c7ebe0c82` |
| `assets/ui/fonts/Inter-SemiBold.ttf` | 419,744 | `78a843fade9d4612a5567302fb595b56976eb5fcebf4fea5a5912d638bafcde3` |

A read-only Python inspection verified all 21 entries in the current hash manifest against their files, both fonts' table directories and ASCII mappings. This inspection is design evidence, not part of the delivered pipeline. Both fonts have 2,048 units/em, 2,937 glyphs, 2,937 horizontal metrics, long `loca`, Unicode `cmap` formats 4 and 12, and `glyf` outlines. All 95 printable ASCII scalars map to nonzero glyph IDs. A separate read-only calculation of `hmtx.advanceWidth * 40 / 2048` matched all 190 committed advances after parsing each old JSON number as f32 (zero bit mismatches); decimal spellings differ because V1 uses shortest round-trip f32 text. Seven ASCII characters are composite in both fonts: U+0022, U+002C, U+003A, U+003B, U+0060, U+0069 and U+006A. Their reachable components use translation only; observed component flags are `0x006`, `0x007`, `0x026`, `0x027`, `0x107`, `0x226`. Simple ASCII outlines have at most 81 points/5 contours in Regular and 76 points/5 contours in SemiBold; these are observations, not parser allocation limits.

`zig/src/retained_atlas.zig` currently validates V1 metadata through `loadFrozenMetadata` using the exact metrics hash above. It requires 205 entries, 64-KiB maximum JSON, a 1-MiB scratch buffer, strict JSON fields, valid finite advances, disjoint ownership and atomic `replaceFrozenMetadata`. It does not generate pixels or validate their hash: `PayloadDescriptor` contains length only. V2 requires an explicit new loader and payload-hash check, not an edit to `FROZEN_SHA256` disguised as compatibility. `tests/ui_assets.rs` supplies existing coverage, mapping, atomic text emission, clipping, finite geometry, metric and provenance contracts.

`zig/build.zig` currently registers no atlas tool. `zig/tools/check.sh` blocks cargo/rustc/rustup but still runs two Python checks. It is a partial migration gate; neither it nor the Rust gate establishes the final owned-tooling requirement. No generator build, atlas regeneration, Rust gate, game gate or target visual acceptance was run for this document.

## Goals and non-goals

Goals are a bounded independently tested Zig generator; byte-identical V2 generation across supported generation hosts and Debug/ReleaseSafe; preserved semantic coverage, metrics and deterministic retained-UI behavior; transactional output publication; a non-mutating freshness/check mode; and complete replacement of this Rust tooling dependency after qualification.

Non-goals are general font/SVG compatibility, font hinting, system font fallback, Unicode shaping, dynamic font atlases, runtime SVG parsing, new font/icon content, shader changes, platform UI implementation, gameplay, city systems and session persistence. OS text/IME/accessibility remains a separate service; rejecting unsupported text in the atlas must retain the existing all-or-nothing behavior. Adding character coverage or changing cells is a separately versioned asset contract.

## Alternatives and selected approach

| Approach | Benefit | Cost / decision |
| --- | --- | --- |
| Owned bounded TrueType/SVG rasterizer; V2 identity | No production dependency or host raster variation; independently testable pure pipeline | More geometry work; selected, with the explicit bounded profile below |
| Platform CoreText/CoreGraphics, later DirectWrite | Less font-engine implementation | OS-sensitive pixels; pure generation delegated to foreign APIs; rejected for the current ownership requirement |
| Port fontdue/resvg algorithms and seek exact V1 bytes | Potential original raster match | Large behavior/dependency/licensing surface with uncertain exact match; rejected as the primary path |
| Keep frozen atlas and only copy/hash it | Fast runtime migration bridge | Does not regenerate from inputs or retire the generator dependency; temporary reference only |

## Proposed architecture

```mermaid
flowchart LR
  I[Committed fonts icons licenses] --> V[Source manifest and bounded validation]
  V --> T[Owned TrueType outlines and metrics]
  V --> S[Owned SVG stroke geometry]
  T --> R[Fixed arithmetic coverage raster]
  S --> R
  R --> D[Threshold and signed distance]
  D --> P[Fixed cell placement]
  T --> J[Canonical metrics]
  P --> H[Hashes and immutable artifact set]
  J --> H
  H --> C[Candidate admission]
  C --> W[Actual target UI witnesses]
  W --> A[Release admission]
```

Proposed ownership under `zig/tools/ui_atlas/`: `profile.zig` constants/allowlist; `inputs.zig` bounded reads/hash verification; `ttf.zig` table/mapping/outline decoding; `svg.zig` XML/path parser; `geometry.zig` fixed arithmetic and curve flattening; `raster.zig` coverage; `sdf.zig` encoding; `metrics.zig` canonical JSON; `manifest.zig` identity/freshness; `publish.zig` staging/rename/check; and `tests.zig` independent fixtures and negative cases. `zig/tools/build_ui_atlas.zig` owns CLI/IO only. Parser and generation modules consume caller-owned immutable byte slices and return owned results with explicit `deinit`; no global allocator or background process. Wire module registration through `zig/build.zig` when implementation reaches that task.

Proposed signatures (design interfaces, not assertions about the current Zig stdlib API):

```zig
pub const AtlasId = enum { nyon_ui_atlas_v1, nyon_ui_atlas_v2 };
pub const Cell = [64 * 64]u8;
pub const Atlas = [1024 * 1024]u8;
pub const SourceSet = struct {
    regular: []const u8, semibold: []const u8,
    icons: [15][]const u8, licenses: [2][]const u8,
};
pub const Outputs = struct {
    atlas: []u8, metrics: []u8, hashes: []u8, manifest: []u8,
    pub fn deinit(self: *Outputs, allocator: std.mem.Allocator) void;
};
pub fn generate(a: std.mem.Allocator, sources: *const SourceSet) AtlasError!Outputs;
pub fn verifyOutputs(sources: *const SourceSet, outputs: *const Outputs) AtlasError!void;
pub fn check(io: std.Io, root: []const u8, selected: []const u8) AtlasError!void;
```

`AtlasError` has distinct input hash, unsupported profile, malformed bounds, unsupported scalar, missing glyph, numeric overflow, capacity, cell overflow, allocation, stale output, stale executable, publication conflict, durability-unknown and IO errors. Diagnostics contain relative file, byte offset/table or entry key, and stage; never dump source buffers. Cancellation before selection commit begins releases owned buffers and preserves the selection. Once commit begins, cancellation is deferred through the final durability barrier and outcome classification; post-replacement errors must never be reported as an unchanged-selection cancellation.

### Source profile and bounds

The immutable input manifest, proposed `assets/ui/atlas-inputs-v2.json`, is separately authored/reviewed and pins the two font hashes above, all fifteen icon hashes, both license hashes, provenance archive records, profile version and ordered semantic mapping. Do not use generated `hashes.json` as a writable source authority. Build paths are fixed repository-relative entries; reject traversal, duplicates, symlinks in input/output paths and files changing between stat/read/hash. Generation never fetches archive URLs. Read each source once into bounded storage and hash the exact bytes subsequently parsed.

Limits are normative: font ≤1 MiB each; SVG ≤16 KiB each; license ≤64 KiB each; source manifest ≤64 KiB; table count ≤32; glyph count ≤4,096; contours ≤128/glyph; decoded points ≤4,096/glyph after component expansion; component depth ≤8 with cycle detection and ≤64 total component references per glyph; instructions ≤65,535 bytes/glyph; SVG ≤128 elements, ≤128 subpaths and ≤4,096 numeric operands/icon; flattened segments ≤8,192/entry; subdivision depth ≤16; numeric token ≤32 bytes; total generation arena ≤64 MiB. Process one entry at a time. Checked arithmetic precedes allocation/slicing/conversion. Limit failure rejects the entire artifact set, never clips or drops geometry. Empty glyph space is valid; missing mappings and empty non-space glyphs are errors.

### Font parsing and scalar mapping

Support only sfnt TrueType version `0x00010000` with required unique `head`, `maxp`, `hhea`, `hmtx`, `cmap`, `loca`, `glyf` tables. Check directory ranges, padded non-overlap and table checksums (`head.checkSumAdjustment` zeroed for that table); validate the whole sfnt checksum adjustment. Additional pinned tables are bounded/checked and otherwise opaque: `GDEF`, `GPOS`, `GSUB`, `OS/2`, `cvt `, `fpgm`, `gasp`, `name`, `post`, `prep`. Reject unknown tables in this profile. No TTC, CFF, variable fonts, WOFF, collections or installed-font lookup.

Require `unitsPerEm=2048`, `indexToLocFormat=1`, monotone long offsets with `numGlyphs+1` entries within `glyf`, and valid `numberOfHMetrics`. Read `hmtx` using the format's repeated final advance for any trailing left bearings even though the pinned fonts use all full metrics. Select Windows platform 3/encoding 1 `cmap` format 4 explicitly; validate segment ordering, terminal segment, even range offsets, subtable bounds and glyph indices. Mapping applies delta modulo 65,536 after nonzero range-index result. Require every U+0020..U+007E mapping nonzero and in range. Other cmap subtables are range-checked but not used. No platform-dependent selection order or format-12 fallback. Non-ASCII scalars, surrogate values, malformed UTF-8 and controls fail before emission; runtime coverage remains printable ASCII only.

Decode simple contours using endpoint indices, instruction length, repeated point flags and signed short/long coordinate deltas, with cumulative coordinate overflow checks. Expand implied on-curve midpoints between consecutive off-curve points, including contour wrap. Close every contour; preserve winding and holes. Composite support is restricted to the observed flag vocabulary: byte/word signed XY arguments, translation, MORE_COMPONENTS, ROUND_XY_TO_GRID, WE_HAVE_INSTRUCTIONS and USE_MY_METRICS. Reject point attachment and scale/2×2 transforms rather than misreading their operands. Parse and bounds-check instructions, but never execute hinting or grid fitting; ROUND_XY_TO_GRID has no hinted-grid effect in this explicitly unhinted profile. Use the selected glyph's `hmtx` advance, matching the existing atlas's per-character advance contract; component metrics affect neither raster placement nor an undocumented kerning adjustment. Reject conflicting USE_MY_METRICS components. Add synthetic nested/cyclic composite fixtures even though current reachable composites are shallow.

Place outlines directly in the cell using `x = 8 + font_x * 40 / 2048`, `y = 48 - font_y * 40 / 2048`; do not round through a bitmap bearing first. A control-hull/bounds pass must establish that rendered coverage fits the cell; report overflow rather than clip. Advance is exactly `hmtx.advanceWidth * 40 / 2048`, represented as a dyadic rational and converted once to f32 for retained UI. This product is exactly representable for the pinned values. Require bitwise equality to all 190 independently frozen V1 advance values before candidate admission. Any failure blocks admission and needs a new metric-contract decision; V2's selected scope permits coverage changes only. No kerning or GSUB/GPOS shaping is introduced.

### SVG parser and geometry subset

Implement a bounded ASCII XML tokenizer accepting only the inspected `svg` root and self-closing `path`, `circle`, `line`, `rect` children. Require root dimensions/viewBox 24×24/`0 0 24 24`, namespace, `fill="none"`, `stroke="currentColor"`, width 2, round caps and joins. Whitespace and attribute order may vary; duplicates and unknown attributes reject. No entities, DTD, processing instructions, comments, groups, styles, transforms, references, scripts, images or external resources. Resolve currentColor to opaque alpha; color never affects coverage.

Number grammar is `[+-]?(digits[.digits?]?|.digits)` with at least one digit, at most six fractional places and magnitude ≤4,096. Exponents, NaN and infinity reject. Whitespace/comma/sign delimiters and implicit parameter repetition follow explicit parser states; `M/m`'s later pairs become `L/l`. Support only observed `M/m L/l H/h V/v C/c S/s A/a Z/z`. Smooth cubic reflection occurs only immediately after C/c/S/s; other commands reset it. Close restores the subpath start; a subsequent relative command starts from that restored point. Reject malformed operand counts and unsupported Q/T commands before rasterization.

The current arcs all have equal positive radii and zero rotation: support circular A/a only, requiring `rx==ry`, rotation 0, binary flags. Reject unequal radii/rotation rather than claim a general elliptical implementation. Coincident endpoints contribute no arc; zero radius becomes a line. If the chord is longer than the diameter, raise the effective radius to half the chord per the endpoint construction. Compute the two centers from chord midpoint ± perpendicular(chord) times `sqrt(r²/chord² − 1/4)`, selecting by large-arc and sweep flags in SVG's y-down coordinates. Use the deterministic fixed arithmetic below. Split the selected sweep into ≤180-degree pieces using the antipodal point when required, then recursively split by normalized radial-vector sums; the exactly antipodal midpoint uses the signed 90-degree rotation. Pin all four flag combinations, radius correction, branch crossing and sign selection in independent endpoint/center/midpoint fixtures.

`circle` is a closed centerline circle. `rect` supports the observed positive dimensions and `rx=1`, with omitted `ry` equal to rx; clamp radii to half extents, construct four straight edges and quarter-circle corners. Lines/path subpaths are open unless explicitly closed. Normalize the source into cell coordinates with scale 5/3 and inset (12,12), including stroke radius 5/3. Flatten quadratic/cubic curves and circle/arc centerlines; render their stroked region as the union of radius-sized capsules for each flattened segment. Capsule union provides round joins/caps and closed joins without alpha darkening; zero-length open segments produce a disk. Nonzero font fill and SVG stroke union are distinct raster modes.

### Deterministic rasterization and SDF

Use signed Q32.32 coordinates with checked i256 intermediates, no floating-point raster math, transcendental library calls, platform raster calls or fast-math. Parse decimal operands as exact integer ratios; fixed conversion, multiplication and division round to nearest, ties away from zero. Square root uses the exact rational midpoint comparison defined below; input scaling and overflow checks are part of the API. Serialize the arithmetic/profile revision in every artifact identity.

#### Normative fixed arithmetic operation order

Let `S=2^32`; every coordinate below is its signed raw Q32.32 integer. All additions/products use checked i256 before narrowing to i64. `R(n,d)` requires `d>0` and rounds the signed rational `n/d` to nearest integer, exact half away from zero: apply sign after `(abs(n)/d) + (2*(abs(n)%d)>=d)`. Never approximate a rational coefficient as a fixed-point constant before multiplication. `Qsqrt(N,D)` requires `N>=0,D>0`: find largest integer k with `k*k*D<=N`; return k+1 iff `4*N >= D*(2*k+1)^2`, otherwise k. This compares the actual half-integer boundary, not the midpoint of adjacent squares. All indicated expressions are evaluated as exact wide integers until the single named `R`/`Qsqrt` operation. Division by zero, overflow or a result outside the coordinate bounds fails before output; there is no epsilon substitute.

| Operation | Ordered raw-integer definition |
| --- | --- |
| Decimal token | Parse sign/mantissa m and decimal-place count d; raw source coordinate `R(m*S,10^d)`. Apply relative offsets by exact checked addition in source coordinates before scaling. |
| Source → cell | For a position q use `12*S + R(5*q,3)`; for a radius/length use `R(5*q,3)`. Stroke radius is `R(5*S,3)`. Do not multiply by `R(5*S,3)` and divide by S. `12.8` gives source raw `54975581389`, scaled-without-inset `91625968982`, final position `143165576534`. |
| Font → cell | With integer font coordinates f, X is `8*S+R(40*f*S,2048)` and Y is `48*S-R(40*f*S,2048)`. Composite translation is added in font units before this conversion. |
| Curve bisection | Each De Casteljau stage uses componentwise `R(a+b,2)`; calculate first-level adjacent pairs, then second-level pairs, then the final split point. Do not substitute a rearranged polynomial expression. |
| Arc chord / radius | Scale both endpoints and input radius first. Let `d=P1-P0`, `L2=dx*dx+dy*dy`. `L2==0` emits no arc; nonnegative zero input radius emits a line. Negative source radii reject. If `4*r*r<L2`, replace r by the least integer with `4*r*r>=L2` (ceiling exact square-root of `L2/4`); retain the original endpoints. |
| Arc center | `M=(R(X0+X1,2),R(Y0+Y1,2))`; `h=Qsqrt((4*r*r-L2)*S*S,4*L2)`. Set sign to −1 when large-arc equals sweep, otherwise +1. Center is `M + sign*(R(-dy*h,S),R(dx*h,S))`. Compute both components from the same d and h. |
| Vector normalization | For nonzero raw vector v, `l=Qsqrt(vx*vx+vy*vy,1)`; reject if l==0. Radius-r projection is `(R(vx*r,l),R(vy*r,l))`. Even a one-raw-unit nonzero vector follows this rule; no near-zero threshold. |
| Arc midpoint | Store sweep direction and whether the piece is a half-turn explicitly. For a less-than-half-turn piece, normalize `(P0-C)+(P1-C)` to r then add C. For an exact half-turn, rotate `(P0-C)` by +90° `(-y,x)` for sweep=1 or −90° `(y,-x)` for sweep=0, normalize to r, then add C. A zero sum in a non-half-turn piece rejects NumericDegeneracy. Major sweeps first split at the radius-r antipode of P0, retaining sweep=1 clockwise in y-down coordinates. Subdivision retains both exact endpoints; it never rebuilds them from the center. |
| Full circle / rectangle | Circle begins at `(cx+r,cy)` and uses four exact quadrant endpoints in order right,bottom,left,top,right (clockwise), flattening one quarter at a time. Rounded rectangle begins at `(x+rx,y)`, proceeds clockwise top/right/bottom/left with exact tangent endpoints; corners use their centers and +90° sweeps. Clamp raw radii before corner construction using `min(r,floor(extent/2))` for positive extents; no nearest rounding is used for this bound. |

Arc radius correction and all path geometry operate after the one source-to-cell conversion. C/c and S/s reflection/relative operations occur in source coordinates before conversion, preserving the SVG command state; flattening occurs in cell coordinates. For circle/rect, scale source x/y/center and lengths independently once, then form the cell-space endpoints above. Tests freeze those operations, not a differently rounded algebraic equivalent. Row/cell clipping is not a way to recover a failed numeric construction.

Independent arithmetic fixtures include all three raw `12.8` values above, positive/negative half rounding, `Qsqrt(2,1)=1`, `Qsqrt(9,4)=2`, a one-raw-unit vector, corrected semicircle endpoints `(0,0),(4*S,0)` with input radius S producing r=`2*S`, center `(2*S,0)`, sweep=1 midpoint `(2*S,-2*S)`, and an uncorrected clockwise quarter from `(S,0)` to `(0,S)` with r=S producing center `(0,0)` and midpoint components `3037000500`. These are exact arithmetic fixtures; they claim no actual generated glyph or visual acceptance.

De Casteljau subdivision bisects at t=1/2 in fixed arithmetic. Stop only when control points lie within 1/256 cell pixel of the endpoint segment (including projection outside its endpoints), or for circular arcs when the midpoint-to-chord distance is ≤1/256. Left subsegment precedes right. Depth/segment caps reject unresolved curves. Fixtures pin endpoints, extrema, control points, flattening order and the conservative deviation check. Approximation tolerance is part of V2 identity; changing it creates a new profile, never a permissive test adjustment.

Each output pixel has an 8×8 centered sample grid at `(x+(2i+1)/16, y+(2j+1)/16)`, i,j=0..7. Font inclusion uses nonzero winding on flattened contours: ignore horizontal crossings, use half-open Y intervals and signed cross products; a point exactly on an edge counts inside. SVG coverage is boolean union of closed capsule tests, using squared distances and dot products with checked wide arithmetic. Bound geometry before these products so every operation fits i256; use a checked helper or return NumericOverflow, never wrapping arithmetic. Coverage byte is `(covered_samples*255+32)/64` integer division. No gamma, color management, LCD subpixels or hinting.

Preserve the old threshold and SDF interpretation: alpha ≥96 is inside; an entirely empty cell becomes all zero. Search offsets −8..8, initial minimum squared distance 64, outside-cell is outside, replace only for strictly smaller opposite-class distance. Use a frozen independently calculated lookup table for squared distances 0..64 instead of host sqrt: `n=min(sqrt(d²)+0.5,8)/8`, inside round(128+127n), outside round(128−128n), positive half-away rounding. Freeze the 65 inside/outside byte pairs from high-precision independent arithmetic before implementing the LUT and verify every pair against the V1 formula. No LUT generated from the implementation under test. It preserves SDF encoding while the alpha raster is explicitly new.

### Packing, metrics and identity

Packing is never heuristic: 1024×1024 row-major R8, 64×64 cells, 16 cells per row. Cell i has origin `(64*(i%16), 64*(i/16))`. Entries 0..94 are Regular U+0020..U+007E, 95..189 SemiBold, then `play,pause,speed,reset,settings,help,crosshair,save,load,check,close,zoom,field-atmosphere,field-hydrosphere,field-topology`. Icon advance remains 40. Entries 205..255 and all other unused pixels remain zero. Full-cell UVs and the existing font origin/baseline remain unchanged.

V2 metrics retain V1 field names and ordered entry shape but use `format_version: 2`. Keep the existing descriptor values and ordering string; no implicit fallback to version 1. Canonical JSON uses UTF-8, two-space indentation, prescribed V1 field ordering, LF and exactly one trailing newline; omit inapplicable entry fields. Write advances as exact terminating decimal dyadic values with unnecessary fractional zeroes removed, retain `.0` for integral f32 fields. Serialize no nonfinite number, timestamp, absolute path or host name. Keep metadata ≤65,536 bytes.

Use a V2 `hashes.json` with schema version 2, existing archive audit records, lexically sorted source records followed by metrics and atlas output records. Source records carry `path_scope: "repository"` and repository-relative allowlisted paths. Generated records carry `path_scope: "artifact"` and fixed bare filenames: `atlas-metrics.json` and `atlas.r8`; `manifest.json` uses artifact-relative `atlas.r8`, `atlas-metrics.json`, `hashes.json` only. Path grammar is nonempty ASCII allowlist segments joined by `/`; reject `.`, `..`, empty segments, leading/trailing slash, backslash, colon, NUL, percent encoding and Unicode variants. Resolve repository paths under the explicit root and artifact names under the validated artifact directory without following symlinks. Candidate IDs, publication roots, absolute paths, selection/receipt paths and generator executable paths are forbidden in generated payload descriptors. Relocating identical artifacts between qualification/release roots does not change any generated byte or candidate ID. A separate canonical `manifest.json` identifies `nyon-ui-atlas-v2`, generation profile, source-manifest SHA-256, output lengths/hashes, exact Zig pin and sorted generator-input digest. Generator inputs include the exact owned atlas Zig module closure, build adapter/profile files and pinned compiler version under the executable-binding contract below. Unknown/missing/extra manifest paths reject. Hashes excludes itself and manifest; manifest hashes atlas/metrics/hashes; candidate ID hashes canonical manifest bytes. This directed hash graph has no self-reference. Qualification receipts carry environment, generator executable identity and timings separately, so compiler/source identity is audited without embedding host-specific binary bytes into deterministic payloads.

Freshness is content-based. Every required build recomputes input/configuration/tool-source identity and compares it to the selected manifest; mtimes and cache presence cannot satisfy freshness. `--check` additionally runs the full generator in memory and compares all four outputs byte for byte. A changed license, source, ordering, numeric profile, Zig pin or generator source invalidates the candidate and applicable receipts even if raster bytes happen to match. Editing expected hashes to accept a failure is forbidden.

### Bind the running executable to its source closure

The atlas build adapter constructs a bounded immutable source snapshot before invoking compilation. It copies the explicit closed list of generator sources, profile/configuration, `zig/build.zig`, the atlas build adapter and `zig/.zigversion` into an isolated build directory; parses no imports outside that closure except the pinned Zig standard library. Reject dynamically discovered imports, undeclared generated inputs, symlinks and source changes while snapshotting. File records use the repository-relative grammar above and sort by path. Define the embedded closure digest as SHA-256 of domain `NYON-ATLAS-BUILD-V1\0`, followed by length-prefixed exact compiler version and each record's length-prefixed path, byte length and SHA-256. Length prefixes and byte lengths are unsigned little-endian u64. Include the compiler distribution identity from the approved per-host toolchain inventory in the build receipt; its platform-specific binary digest stays out of deterministic atlas bytes. The compiler's built-in version must equal the embedded version and `.zigversion`.

Compile only this snapshot using explicit include/module paths and no checkout-relative imports. The adapter writes a generated identity module containing the digest/version, imports it into the generator and copies neither that generated module nor executable into the digest's input closure. The adapter source and fixed identity-module template are included inputs, so this exclusion creates no editable unbound behavior. Any future generated code must be a deterministic function of already included bytes with its generator included, or the build fails closure validation. Compare the checkout's complete input closure to the snapshot before and after compilation; on drift discard the build as `ConcurrentModification`. Host-specific build receipts bind snapshot digest, actual compiler distribution and invocation/options/target, and actual output executable SHA-256. These receipts are required for admission and are independently rehashed; they contain no candidate ID, preventing a build/candidate hash cycle. Debug/ReleaseSafe settings belong in these receipts; both modes embed the same semantic source/profile/compiler-version digest so payload reproducibility remains testable.

The build adapter emits `zig-out/bin/nyon-ui-atlas.build.json` as the trusted-build receipt; every CLI operation requires `--build-receipt <path>`, verifies its executable hash against the actual running binary, and fails if it is absent. Every direct `generate`, `check`, `check-candidate`, admit or promote invocation first reads the running generator's embedded digest/version, snapshots the exact current closure, and requires equality. A stale executable returns `StaleExecutable` before generation or publication; it must never attest to the newly hashed disk closure as if it were its own code. Generation uses its authenticated input buffers. Rehash the closure and asset inputs before candidate publication or returning successful check/admission; drift returns `ConcurrentModification` without publication. The exclusive publication lock protects writers, not source edits; perform the final identity comparison while holding that lock and immediately before the first publication operation. The identity always names the compiled snapshot even if the checkout changes after that comparison; it never relabels old code as new. A later build/check will reject the now-stale selection. No arbitrary executable can gain admission merely by emitting a claimed digest: admission requires the independent trusted-build receipt's executable hash and the embedded digest to match the actual invoked binary. This is a repository build provenance contract, not a defense against a compromised approved compiler or operating system.

Mandatory regression: build A; independently modify each of a module, profile, adapter/template and compiler-pin input to B; invoke A directly for generation and check; expect `StaleExecutable`, no candidate/selection writes. Cleanly rebuild B, verify the embedded/receipt/executable binding, and require a changed manifest/candidate ID even for a behavior-neutral edit with identical raster bytes. Race tests edit a file during snapshot, compilation and generation; none may produce a successful B-labelled result from A. Test forged embedded identity without the matching approved build receipt, and deliberately omitted import dependencies. Runtime source verification is necessary for every direct CLI path, not just the gate's preceding build command.

### Transactional publication and check mode

Candidate output lives under ignored `zig/zig-out/qualification/ui-atlas/<candidate-id>/`. Release V2 assets ultimately live in new `assets/ui/generated/nyon-ui-atlas-v2/<candidate-id>/`, with selection held by a small versioned `assets/ui/selected-atlas.json`. These are proposed paths, created only by the implemented generator/promoter. Existing V1 files remain untouched during qualification. The selection schema is `{ "format_version": 1, "asset_id": "nyon-ui-atlas-v2", "candidate_id": "<64 lowercase SHA-256 hex>", "release_receipt_sha256": "<64 lowercase SHA-256 hex>" }`, serialized canonically with no user-supplied path. Resolve the immutable directory from the validated fixed asset ID and candidate hash under the fixed root. The release receipt is a separate immutable document, `release-receipt.json`, containing manifest hash, specification/profile scope and the complete required witness closure. It is excluded from the generated candidate manifest hash because it is produced later. The selection authenticates that receipt by hash; loaders validate its closure instead of trusting its label. Qualification binaries receive an explicit candidate directory/expected hash and never synthesize a release selection.

Generate and validate all outputs and, for promotion, the authenticated release receipt before touching publication. Acquire an exclusive create-new lock for the target publication root; fail on contention and never delete another writer's lock. Validate that the filesystem supports the required same-filesystem atomic name operations and directory durability barriers before entering commit. The source-binding checks above run under this lock before publication.

The ordered durability protocol is normative; file sync is not directory-entry sync:

1. Create any missing destination/staging ancestors, and selection ancestors only for promotion, one at a time under the fixed root. After each creation sync the new directory and the existing parent that acquired its name, proceeding top-down; a failure is pre-commit. Require source/destination directory renames on one filesystem. Use a unique invocation-owned staging directory, never reuse a crash-left staging name.
2. Write all four generated payloads and, for a release publication, `release-receipt.json`. Sync every file's contents/metadata, re-read and hash every file, then sync the staging directory containing their names. For candidate-only generation no receipt exists and selection is never touched.
3. Rename staging to the absent immutable candidate-ID directory. Sync both affected parent directories after the rename (once if the same parent), and sync the final directory. These barriers must complete before preparing selection. For an already existing directory, verify exact payloads and, for release publication only, the receipt, re-sync each file and final directory, and sync its parent; prior crash-interrupted naming is not assumed durable. Candidate and release roots are distinct; promotion stages a new complete release directory including receipt rather than mutating an immutable qualification candidate.
Candidate-only `generate` stops after step 3 and returns the candidate publication result below; it never runs steps 4–5, creates selection ancestors, or reads/writes a selection or release receipt. Steps 4–5 apply only to promotion.

4. Re-read the current selection under the lock and verify its expected previous identity. Check cancellation now. Write the exact new selection into a unique sibling temporary file, sync that file, then sync its parent to persist the temporary name. At this point candidate/receipt names and bytes have completed their durability barriers while selection still names the old artifact.
5. Enter `Committing`: defer cancellation, atomically replace the selection name with the temporary file, and record whether the replacement definitely happened. The successful replacement is the **visible commit point**. Immediately sync the selection parent after replacement; only completion of this final barrier yields `CommittedDurable`. Never truncate/write the selected file and never automatically roll back after replacement.

Expose separate results: `publishCandidate(...) -> CandidatePublicationOutcome` executes steps 1–3 in the qualification root; `promoteSelection(...) -> SelectionCommitOutcome` executes the full protocol in the release root. `check` and `check-candidate` return a read-only `VerificationOutcome` (`Verified` or `Rejected`) and never claim either kind of publication. Every command diagnostic includes `operation` and its corresponding outcome type; these outcomes are not interchangeable.

| CandidatePublicationOutcome | Meaning and recovery; selection always untouched |
| --- | --- |
| `CandidateNotPublished` | Pre-publication validation/IO/conflict/cancellation failure, or a rename failure definitively guaranteed not to install a new final name. This invocation installed no final name; a preexisting candidate may still exist. No durability/admission success is claimed. Clean only invocation-owned staging files. |
| `CandidateDurable` | All step-3 barriers completed for the exact four candidate payloads. Both fresh publication and byte-identical existing-candidate verification plus file/directory/parent re-sync return this success, with `created` versus `existing` recorded. Return the candidate ID/path, not a release-selection identity. No selection or release receipt was accessed or created. |
| `CandidateDurabilityIndeterminate` | Candidate directory rename had ambiguous outcome, a fresh rename succeeded but any step-3 barrier failed, or an existing verified candidate's required re-sync failed. The directory may be present and correct, but durable publication is unproven. Record candidate ID, created/existing path and failing barrier; retain final directories, never delete a preexisting one or imply that selection changed. A later invocation under the publication lock verifies exact bytes and completes all step-3 barriers before it can return CandidateDurable. |

Before a fresh candidate rename or existing-candidate re-sync begins, honor cancellation as `CandidateNotPublished`. From that point defer cancellation through step 3: completed barriers yield `CandidateDurable` with a deferred-cancellation informational flag; failed/ambiguous barriers yield `CandidateDurabilityIndeterminate`. A crash after rename but before its barriers has the same unproven-durability recovery rule. Cleanup errors after `CandidateDurable` are separate diagnostics and cannot relabel the candidate as absent or undurable. Unsupported candidate-directory durability primitives reject before publication as `CandidateNotPublished`; pure in-memory generation and read-only checking still work.

The following table belongs exclusively to `SelectionCommitOutcome`; promotion still returns `NotCommitted` on any artifact-stage failure before selection replacement, with the artifact-stage diagnostic retained. Candidate-directory uncertainty must never be promoted to selection-commit uncertainty when no selection replacement was attempted:


| Outcome | Meaning and recovery |
| --- | --- |
| `NotCommitted` | Failure/cancellation before replacement, or a failed replacement definitively guaranteed not to modify selection by the qualified filesystem primitive. Selection remains the prior bytes; an unselected complete candidate may exist. Clean only this invocation's staging/temp files. |
| `CommittedDurable` | Replacement and final selection-parent durability barrier completed. New selection identifies the complete durable artifact set. A deferred cancellation is reported as an informational flag, never as an unchanged-selection failure. |
| `CommitIndeterminate` | Replacement outcome is ambiguous, or replacement succeeded but the final parent barrier failed/was interrupted. Selection may visibly be new and crash recovery may observe old or new selection; report previous/new IDs and failing barrier, retain all artifacts, and never claim failure preserved the old selection. A later recovery invocation takes the lock, reads and validates the observed selection and referenced set, then performs the required sync barriers; it reports the observed durable selection without silently reverting or retrying a competing promotion. |

For any observed selection, every referenced candidate/receipt has already crossed its own durability barriers before visible commit. A crash before commit preserves the prior durable selection; a crash in `Committing` has indeterminate old/new visibility but never intentionally publishes a pointer to an unsynced candidate. After a crash with a stale lock, explicit ownership/recovery checks precede lock removal. A cleanup failure after `CommittedDurable` does not downgrade committed data to `NotCommitted`; record cleanup separately.

Platform implementations must demonstrate their native file-content, directory-entry, replacement and flush semantics on the tested filesystem. On Linux the adapter must include a successful directory `fsync` equivalent after each relevant rename; on macOS it must qualify the actual file/directory flush and replacement sequence, including any stronger flush needed for the claimed power-loss guarantee. On Windows a successful file `FlushFileBuffers` or replace call alone is not assumed to establish parent-directory durability. If the available Zig/system filesystem primitives cannot establish the ordered barriers on a platform/filesystem, return `UnsupportedDurablePublication` before `Committing`; release promotion is unavailable there until an equivalent protocol is independently qualified. Candidate publication can proceed independently of selection promotion whenever its own steps 1–3 are supported; lack of a selection-replacement primitive alone does not block it. If candidate-directory durability is also unsupported, only pure in-memory generation/check and use of an already qualified packaged atlas remain available there; do not return CandidateDurable without its barriers. No foreign raster code or new dependency is introduced; any native durability operation stays in the owned Zig IO boundary using already approved system interfaces. Network/unknown filesystem semantics are unsupported for promotion. Unit fault tests do not establish actual OS/power-loss behavior.

`--check` is read-only with respect to project files and selected assets: authenticated executable/source snapshots, in-memory generation, no lock, no publication, no selection edit. Build-system disposable compiler cache writes are allowed separately. Hash/read the selected immutable manifest once; verify source snapshots and selected pointer unchanged at end, otherwise report `ConcurrentModification`. CLI exits: 0 for `generate` only with `CandidateDurable`, for promotion only with `CommittedDurable`, and for `check`/`check-candidate` only with `Verified`; 1 invalid inputs/different output; 2 IO/toolchain prerequisite/unsupported durability before the applicable publication boundary; 3 publication conflict; 4 `StaleExecutable` or source drift; 5 only selection `CommitIndeterminate`; 6 only candidate `CandidateDurabilityIndeterminate`; 7 cancellation honored before the operation's publication boundary. JSON diagnostics always include the operation, typed outcome and failing phase. Promotion failing at candidate-directory sync returns `NotCommitted` plus exit 2/artifact-stage diagnostic, not exit 5: selection is known untouched. A candidate-only unknown durability result uses exit 6, never exit 5. Read-only `Verified` checks exact content/identity/freshness; it does not assert a directory sync or durable publication occurred.

Normative focused outcome fixtures, in addition to every existing promotion fault test:

| Command and observed events | Outcome / exit | Required selection/receipt effect |
| --- | --- | --- |
| generate: fresh candidate, all step-3 barriers complete | CandidateDurable(created) / 0 | No reads or writes of selection/release receipt |
| generate: exact existing candidate, all re-sync barriers complete | CandidateDurable(existing) / 0 | Same; no payload mutation |
| generate: validation, pre-rename IO or supported-primitives check fails | CandidateNotPublished / 1 or 2 as diagnostic specifies | Same; only owned staging cleanup |
| generate: cancellation before rename/re-sync | CandidateNotPublished / 7 | Same |
| generate: rename succeeded, candidate-parent sync fails | CandidateDurabilityIndeterminate / 6 | Same; retain candidate |
| generate: cancellation arrives after rename; remaining barriers succeed | CandidateDurable(created), cancellation_deferred / 0 | Same |
| generate: existing candidate re-sync or ambiguous rename fails | CandidateDurabilityIndeterminate / 6 | Same; never delete existing candidate |
| check-candidate: current authenticated executable and exact valid candidate bytes | Verified / 0 | No publication, lock, selection or receipt creation; no durability claim |
| promote: candidate/release directory sync fails before replacement | NotCommitted / 2 | Prior selection unchanged; retain artifact diagnostic |
| promote: selection replacement succeeds, final parent sync fails | CommitIndeterminate / 5 | Selection may be new; no rollback claim |
| promote: all barriers complete | CommittedDurable / 0 | New durable selection |

The candidate fixtures use a filesystem operation recorder that fails any selection-path or release-receipt access. Inject faults/cancellation on both sides of the rename and every candidate sync, including the existing-directory branch, and assert exact recorded barriers and outcomes. These model tests establish intended protocol behavior, not actual filesystem durability. No exit is a full-game or visual qualification claim.

## Retained UI interface and data changes

Keep `loadFrozenMetadata` and its V1 digest untouched while it is needed as the historical witness. Add a separate `loadVersionedMetadata` that receives the expected versioned asset manifest, metrics bytes, actual atlas bytes and bounded scratch; hash both payloads before strict schema validation and resident replacement. It accepts only an explicitly selected V2 release identity in ordinary builds, or an explicitly supplied V2 candidate identity in qualification builds. The fixture-only checksum bypass must remain unreachable from production entrypoints. Extend payload descriptors with expected identity/hash rather than inferring integrity from length.

V2 retains existing allocator/aliasing budgets, nonfinite geometry rejection, clipping/UV correspondence, weighted advances, atomic text emission, caret boundary mapping and unsupported-character failure. No schema change to gameplay or user saves occurs. Switching asset versions is a build/package selection; no user data migration, runtime download or font-cache mutation. Unrecognized versions fail explicitly without silent V1 fallback.

## Independent validation and acceptance

| Layer | Required evidence |
| --- | --- |
| Inputs/parser | Frozen source hashes; table/mapping/advance fixtures extracted and reviewed independently; truncated header/table/glyph/cmap/loca/instruction cases, arithmetic overflow, overlaps, duplicate fields, repeat overruns, cycles, missing glyphs, unsupported transforms/XML/scalars, allocator failure at each allocation |
| Geometry | Hand-authored simple/composite TrueType binary fixtures, quadratic implied points/winding holes; cubic smooth reflection; circular arcs all flags/radius correction; round caps/joins, rounded rectangles, close/reopen and degenerate segments; exact fixed-arithmetic ties/sqrt boundaries |
| Raster/SDF | Hand-computed square/triangle/disk/capsule samples including all 0..64 coverage counts; overlapping stroke union; edge ties; all threshold 95/96/97, empty/full/single/border/corner masks; exact independent 65-pair SDF LUT; never derive expected values with generator helpers |
| Atlas/contracts | All 205 identities/cells, zero unused cells, no overflow, complete V1/V2 per-entry pixel diff; all 190 advances bit-identical to frozen V1 fixtures; no change to retained text widths/caret/clip behavior; strict V2 loader and hash mutation rejection |
| Publication | Independent fault injection before/after every ancestor/file/directory barrier and both renames; candidate-parent barrier failure forbids selection replacement; post-selection-parent failure returns CommitIndeterminate, never NotCommitted; ambiguous replace and deferred cancellation fixtures; crash-left existing candidate gets re-synced; actual native filesystem interruption/power-loss proof remains separate; check mode writes nothing |
| Source binding | Build A/edit B/invoke A rejects on every CLI; snapshot/compile/generate races, omitted imports, forged receipt/binary, rebuild B with unchanged pixels changes identity; approved compiler/binary/source snapshot binding verified |
| Reproducibility | Artifact relocation across candidate/release roots preserves all generated bytes and ID; two fresh generations with empty isolated caches; Debug and ReleaseSafe; actual macOS ARM64, Windows x64 and Linux x64 generation, identical payload/manifest bytes; input traversal order and timezone/locale must not affect results |
| Target runtime | Pinned candidate plus applicable shader/host IDs; all glyphs/icons and representative retained screens, actual image/layout/input/accessibility evidence for every required backend and scale row |

Freeze handwritten numeric fixtures before implementation. Freeze V1 metrics/hash evidence before generating V2. New V2 whole-atlas goldens are admitted only after these independent tests and visual review; generated bytes are not themselves correctness evidence. The comparison report lists each cell's changed pixel count, max byte delta, coverage bounds, thresholded area and old/new advance. No global similarity threshold hides a missing dot, hole, descender or icon stroke. There is no claim that visual V1/V2 images must be identical. Acceptance requires readable complete glyphs, preserved holes/strokes, no bleed/cropping, unchanged baseline/advance/caret placement and no retained control layout regression. Any failing glyph/icon blocks promotion; do not loosen thresholds after inspection.

Use the platform specification's viewport matrix: 723×802 at 100/115/130%, and 1280×720, 1440×900, 1920×1080 at 100/130%; cover native high-DPI changes and browser device-pixel-ratio 1/2. Include font sizes 12/16/20/40, both weights, icons 16/24/40, partially clipped cells, disabled and high-contrast states. Validate native Metal, DX12, Vulkan, applicable approved GL fallback and browser WebGPU/WebGL2. Target observers record legibility, punctuation, whitespace, caps/joins, caret and clipping results explicitly; screenshots alone are not a signed acceptance result. Actual host/browser/GPU identities and artifact hashes accompany captures. Missing targets remain unverified. The atlas task consumes platform journeys; it does not implement hosts or shaders.

### Proposed exact atlas gate

From `zig/`, with the exact pinned compiler, proposed build steps are:

```sh
zig fmt --check tools/build_ui_atlas.zig tools/ui_atlas src/retained_atlas.zig
zig build test-atlas
zig build test-atlas -Doptimize=ReleaseSafe
zig build atlas-tool -Doptimize=ReleaseSafe
./zig-out/bin/nyon-ui-atlas check --build-receipt zig-out/bin/nyon-ui-atlas.build.json --root .. --selection ../assets/ui/selected-atlas.json
zig build check-atlas-retained -Doptimize=ReleaseSafe
```

Before selection exists, replace the `check` invocation with `generate --build-receipt zig-out/bin/nyon-ui-atlas.build.json --root .. --out zig-out/qualification/ui-atlas` and `check-candidate --build-receipt zig-out/bin/nyon-ui-atlas.build.json --root .. --candidate <exact-produced-manifest>`; no fallback to V1 or copied outputs. All commands above are proposed and currently unimplemented. Detailed plan tasks must install them before calling the gate.

Final required atlas generation/check graph contains only pinned Zig compiler/std, project Zig source and input assets. Qualify in a fresh environment with Rust/Python/Node/font tools absent, clean build caches and network disabled; trace executable invocation and inspect build/link/package inputs to detect absolute-path tools, precompiled Rust libraries and hidden generator calls. PATH stubs alone are insufficient. Runtime packages embed selected R8/metrics and contain no generator/font parser service. Final project gate must call the owned atlas checks through its Zig driver; the current shell/Python migration gate is not the final gate. Broader tool replacement belongs to the parent roadmap.

## Rollout and retirement

1. Freeze current source/metric/hash evidence and independent fixtures; implement pure parser/geometry/raster modules with synthetic failures and leak checks.
2. Generate V2 only into isolated immutable candidates; run all structural/numeric/metric/repeatability checks and retain complete V1/V2 diffs. This closes candidate admission only.
3. Add explicit V2 retained loader and qualification selection. Available macOS hosts may gather their target receipts while other targets remain pending. Normal packaging rejects candidates and incomplete release receipts.
4. Complete every required target/runtime and cross-host generation row; promote the exact candidate without rebuilding it. Emit a release receipt and atomically select it. A changed input/tool/profile/output requires a new candidate and affected qualification.
5. Under the user's existing no-Rust retirement authorization, remove the obsolete atlas Cargo workspace, Rust generator and required Cargo atlas checks only after V2 regeneration, retained UI, runtime and delivered-artifact proof passes. Preserve original source fonts/icons/licenses, frozen V1 evidence and any unported contracts. Update provenance/build/docs through their owners. Do not delete another agent's files or unqualified behavior.

Rollback before retirement selects the previous qualified immutable set atomically; a binary accepts only versions it explicitly implements. After Rust retirement, rollback is to the previous qualified Zig asset/package identity, never a newly required Rust path. V1 evidence is data and may remain in a bounded migration fixture archive. No automatic cleanup of user data or working trees is part of atlas rollback.

## Security, privacy and licensing

Parsers are offline build-time boundaries over allowlisted bundled inputs. Hash checking narrows accepted release inputs; bounded malformed-input tests remain necessary because parser fixtures and future source maintenance bypass release hash admission. Cap all bytes/work, reject unknown grammar early, check arithmetic, use explicit ownership and never execute font bytecode/XML content. Nothing reads user fonts, network URLs, user profiles or system font configuration. Reports contain only project-relative input identities, errors and artifacts.

Keep `assets/ui/licenses/Inter-OFL.txt` and `Lucide-ISC.txt` verbatim, including the Feather-derived MIT notice inside the latter. Preserve the original font binaries and attribution/provenance records; the generator does not modify fonts or claim authorship of Inter/Lucide. Ship notices with release packages as the current provenance contract requires. There is no new production dependency and no code vendored from fontdue/resvg. If implementation later copies third-party code, stop that change for a separate dependency/license review; this design assumes owned implementation from format contracts and independent fixtures.

## Observability, budgets and risks

Each CLI result records stage, exact source/profile/tool identities, entry count, output hashes, duration, peak accounted allocation and typed failure. Emit per-entry differences only on request; default summary lists failing keys and report path. No background telemetry or alerting is needed; CI fails on any stale/malformed/non-reproducible result. Timing and host identifiers live in evidence receipts, never deterministic output manifests.

The output atlas is exactly 1 MiB plus ≤64 KiB metadata and bounded manifests; two fonts total 831,384 bytes. The naïve SDF bound is 205×4096×289 ≈243 million neighbor checks, with early squared-distance pruning. Coverage has 53,739,520 sample positions before edge tests; row/shape bounding boxes may prune work without altering inclusion rules. Target complete generation ≤30 seconds and ≤64 MiB owned arena on the measured macOS host; these are proposed engineering budgets, unmeasured today. Record full generation and raster/SDF phase timings; exceeding a budget requires a reviewed optimization preserving exact vectors, not a fabricated performance pass.

| Risk | Severity | Mitigation |
| --- | --- | --- |
| Owned curves/strokes subtly lose punctuation or holes | High | Independent primitive/composite geometry and raster fixtures, per-entry reports, all-entry actual visual sheets |
| Numeric divergence between compilers/hosts | High | Integer fixed arithmetic, checked intermediates, pinned compiler/profile, independent rounding fixtures, cross-host byte checks |
| Compound-font handling accidentally omitted | High | Observed seven composites explicitly required; child expansion, cycle/depth/flag fixtures before full-font acceptance |
| Pixel change silently alters retained layout | High | Preserve all advance bits and baseline/cell contract; versioned loader and caret/clip/control regression fixtures |
| Partially published asset set breaks build/runtime | High | Immutable staged directories, selected pointer last, crash/concurrency tests, payload hashes before loading |
| Broad format support expands into a font/browser engine | Medium | Exact allowlisted table/command profile; unsupported feature failure instead of partial rendering |
| Slow simple raster implementation | Medium | Bounded shape pruning and measured phase times; defer optimization until exact outputs are fixed |
| Missing backend evidence stalls final retirement | High | Admit candidates independently, qualify available hosts first, retain pending rows and old behavior until replacement proof |

## Open evidence and implementation handoff

No product decision is delegated to a future implementer: V2, source profile, coverage math, SDF encoding, ordering, metric preservation and publication protocol are selected here. Outstanding items are evidence, not permissive defaults: reviewed independent fixture bytes; an implemented generator and V2 loader; actual cross-host determinism/timing; actual filesystem crash semantics; target visual/layout/accessibility receipts; and the final Rust-free artifact graph. None currently passes by virtue of this document.

Implementation should proceed as separately reviewed slices: input/profile/independent fixtures; TTF mapping/outlines; SVG geometry; shared coverage/SDF/packing; serialization/transactional CLI; retained V2 admission; then target qualification and retirement. Each slice gets named failure tests and actual Debug/ReleaseSafe exit evidence before its dependents run. The detailed implementation plan is a later artifact; this design does not create source files, modify generated outputs or overwrite existing plans.

## References

- [Approved expanded Zig roadmap](../plans/2026-10-11-expanded-zig-rewrite.md)
- [Finish NYON in Zig](../plans/2026-10-11-finish-nyon-in-zig.md)
- [Platform bindings design and candidate/target/release protocol](2026-10-09-nyon-zig-platform-bindings-design.md)
- [Historical atlas plan; platform raster portions superseded here](../plans/2026-10-09-nyon-zig-atlas-tooling.md)
- [Current atlas generator](../../../tools/build-ui-atlas.rs)
- [Isolated current dependency manifest](../../../tools/ui-atlas/Cargo.toml)
- [Current asset tests](../../../tests/ui_assets.rs)
- [Current retained metadata validator](../../../zig/src/retained_atlas.zig)
- [Current source provenance](../../../assets/ui/PROVENANCE.md)
- [Current hash manifest](../../../assets/ui/hashes.json)

## Revision summary — 2026-10-11 atlas review response

The original review file remains unchanged as requested. This specification addresses its four open issues:

- Issue 1 — Status: addressed. Response: publication now orders file/staging/rename-parent/final-selection-parent barriers, including new ancestors and existing directories; names the visible commit point; defines NotCommitted, CommittedDurable and CommitIndeterminate; defers commit cancellation; and fails explicitly for unsupported platform durability. Independent fault/crash cases distinguish these outcomes.
- Issue 2 — Status: addressed. Response: immutable compile snapshots and embedded canonical closure/compiler-version digests bind the running executable to its source. Trusted-build receipts bind actual compiler/executable identities; direct CLI snapshot comparison rejects stale executables, with explicit cycles/exclusions, race and behavior-neutral-edit tests.
- Issue 3 — Status: addressed. Response: the normative arithmetic annex fixes ratio scaling, sqrt midpoint comparison, arc correction/center/normalization order, circle/rectangle orientation and degenerate handling, with exact independently calculated raw-value fixtures.
- Issue 4 — Status: addressed. Response: generated records use fixed artifact-relative basenames, source records are separately repository-relative, forbidden path forms are enumerated, and relocation stability is required. Candidate/publication/receipt paths cannot enter the candidate hash graph.

This revision changes only this design document. Generator/source/runtime/filesystem and visual acceptance remain unimplemented or unverified; successful document and arithmetic checks are not runtime qualification.

## Revision summary — 2026-10-11 atlas review fix 2

Fix-1 review Issue 1 — Status: addressed. Response: split candidate publication from selection commit and read-only verification. Candidate-only generation now succeeds as CandidateDurable after its own rename/directory barriers (including idempotent re-sync); candidate durability uncertainty has its own outcome/exit and cannot imply a selected pointer changed. Explicit cancellation boundaries, command/outcome/exit fixtures and selection/receipt access rejection retain all prior promotion durability guarantees. Both raw review files remain unchanged. No arithmetic/raster contract, product source, asset or runtime evidence changed.
