# NYON Zig Atlas Tooling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Regenerate and qualify the complete fixed UI atlas with owned Zig tooling and explicit compatibility/version gates.

**Architecture:** Owned Zig code authenticates committed sources, parses the exact SVG subset, owns cell placement/SDF/serialization and calls a narrow installed platform raster boundary. Isolated qualification candidates precede target UI witnesses and all-target release promotion; none replaces committed assets.

**Tech Stack:** Pinned Zig/std; approved CoreText/CoreGraphics calls on the first generation host; separately authorized alternatives only.

**Spec:** [Approved NYON Zig platform binding design](../specs/2026-10-09-nyon-zig-platform-bindings-design.md). Read it with this plan. Exact user-approved bytes are `.superpowers/sdd/2026-10-09-nyon-zig-migration/task-6-user-approved-platform-spec.md`, SHA-256 `1d3257c31e7383118e46a78e6550e83752b4f6d2cf2fb74fc6d79b1af0c04d04`; durable spec approval metadata does not change its architecture.

**Status:** Proposed execution plan; user plan review/approval pending. Donald approved the written platform spec on 2026-10-09 at 06:52 EDT; that approval authorized detailed planning only. Existing SDD execution method is preserved, not newly dispatched by this file.

## Global Constraints

- Exact compiler: `0.18.0-dev.120+9fe22a29b`; no pin changes, new branches/work trees, commits, pushes, installs, downloads, new dependencies, source retirement, real-profile use or asset replacement without their separately required authorization.
- All retained project-owned generation/build/test-driver/corpus/validation/package logic is Zig. Temporary Rust game oracles and Python/shell migration checks remain until independently qualified replacements; no final Rust/Python/shell implementation exception.
- Foreign C/Objective-C/COM/JS code only performs approved platform calls. OS/SDK compilers, drivers and raster APIs are declared external boundaries, not permission to add a foreign project tool.
- 6A1 collects read-only existing versions/headers/licenses/protocol evidence and missing-prerequisite proposals. Only separately authorized 6A2 acquires/links/probes. A receipt records authorization; it never grants it.
- Candidate admission → authorized available-target qualification → target witnesses → all-target release promotion. Ordinary release loading/packaging rejects qualification candidates. Current assets remain intact; changed asset versions require separate approval.
- Every task below is future work. No listed command has been run by this planning dispatch. Do not report an unavailable compiler/device or skipped target as a pass.
- Run all future commands below from `/Users/donaldfilimon/dev/active/NYON/zig` unless an absolute repository-root command is shown. The build steps/options below are introduced by main task H01; they are not claimed available today.
- Each code task starts with type-correct inert scaffolding and tests. Compile-only must exit 0 before the behavioral RED run. A compiler/import error is never the intended RED. End with a frozen diff/receipt and fresh review checkpoint instead of a commit.

## Review Focus

1. Platform raster drift must not silently rewrite assets; U01's follow-on measurement/U06 `all_205_compatibility`, `mismatch_needs_version_approval` preserve current bytes.
2. A glyph at a negative bearing or cell edge must fail before clipping; U04 `bearing_overflow`, `alpha_95_96_97` pin placement/SDF.
3. Malformed SVG or a local provenance URL must not trigger network/resource loading; U02/U03 `reject_external`, `nonfinite_arc`, `unsupported_command` own fail-closed behavior.
4. Candidate loading must not require its future UI witness or admit a release; U06/U07/U08 `candidate_before_host`, `release_rejects_candidate`, `missing_target_pending` own the acyclic protocol.
5. A valid old witness must not approve changed font/tool/raster bytes; U06/U08 `changed_identity_invalidates`, `cross_version_receipt_rejected` bind independent evidence.

---

## File map and interfaces

Paths below are proposed, absent until their task is authorized. Modify `zig/build.zig` only through main H01's registration seam. The shared contract is [main H01](2026-10-09-nyon-zig-platform-bindings.md#task-h01-shared-artifact-identities-and-test-harness); this plan consumes its exact `Allocator, Digest, Candidate, OwnedCandidate, Approval, Target, Witness, Requirement, TargetAuthorization, ArtifactError, ReleaseManifest` definitions.

| Files | Responsibility and owner |
| --- | --- |
| `zig/tools/ui_atlas/{contract,input}.zig`, `zig/tests/fixtures/atlas/{sources,independent-vectors}.json` | U00 exact source/ordering/format and reviewed independent expectations |
| `zig/tools/ui_atlas/raster.zig`, `zig/host/macos/nyon_raster.m` | U01 typed alpha-mask/metrics boundary; platform calls only |
| `zig/tools/ui_atlas/{svg_lex,svg_parse,svg_geometry}.zig` | U02/U03 bounded XML/path grammar and geometry |
| `zig/tools/ui_atlas/{placement,sdf}.zig` | U04 cell placement and exact arithmetic |
| `zig/tools/ui_atlas/{metrics,hashes}.zig` | U05 finite metrics, exact serialization/provenance |
| `zig/tools/ui_atlas/{candidate,witness,promote}.zig` | U06/U07/U08 structural admission, target witness and release join |
| `zig/tools/build_ui_atlas.zig`, `zig/tools/ui_atlas/tests.zig` | U09 Zig CLI; task-labelled unit tests throughout |
| `docs/qualification/platform/atlas/` | Future frozen source, generator and target receipts; never status by inference |

U00 defines: `SourceSet{fonts:[2]Input,icons:[15]Input,licenses:[]const Input,baseline:[3]Input}`, `Input{path:[]const u8,bytes:[]const u8,hash:Digest}` borrowed from an owned input arena; `CellId=u16`; `Mask{alpha:[]u8,width:u32,height:u32,xmin:i32,ymin:i32,advance:f32,allocator:Allocator}` owning, `deinit(*Mask) void`; `RasterSettings{font_px:u32=40,icon_px:u32=40,api:Digest,os:Digest,options:Digest}`; `Raster{context:*anyopaque,font_fn:*const fn(*anyopaque,Allocator,Input,u21,RasterSettings) AtlasError!Mask,path_fn:*const fn(*anyopaque,Allocator,*const Geometry,RasterSettings) AtlasError!Mask}`. Raster context has one stable owner and no ambient font/network fallback. Font/path callbacks return owned bytes; fixed atlas operations never call runtime international-text fallback.

`AtlasError=Allocator.Error||error{BadSource,UnsupportedXml,UnsupportedPath,Limit,Nonfinite,MissingGlyph,CellOverflow,BadMask,MetricMismatch,PixelMismatch,UnapprovedVersion,IoFailure,Unavailable}`.
U02 owns `Svg` parsed arena and `deinit(*Svg) void`; U00 declares opaque `Geometry` for the Raster callback signature; U03 owns its normalized path arena behind `*Geometry` and `destroyGeometry(a:Allocator,shape:*Geometry) void`. Both immutable after validation. U04 `Cell=[4096]u8`, `Atlas=[1048576]u8`. U05 `Entry{kind:enum{regular,semibold,icon},key:[]const u8,cell:CellId,advance:f32}` plus explicit format-v1 fields frozen from current metrics in U00; `Outputs{atlas:[]u8,metrics:[]u8,hashes:[]u8,allocator:Allocator}` owns all payloads with `deinit(*Outputs) void`. Exact descriptor/entry JSON names/order and optional omissions are reviewed U00 inputs, never inferred from std JSON defaults.
U06 `AssetMode=union(enum){exact_compatible:void,changed_version:Digest}`; `AssetTargetReceipt{candidate_id:Digest,target:Target,witnesses:[]const Witness}`. U08 consumes main `ReleaseManifest` with one allocator owner.

## Frozen behavior and execution order

The source oracle is `tools/build-ui-atlas.rs` (included by `tools/ui-atlas/src/main.rs`), not a nonexistent font-atlas.rs. It is read-only behavior evidence; no Rust execution/library enters the selected tooling graph. U00 records actual source paths from `assets/ui/PROVENANCE.md` and `assets/ui/hashes.json`, the two committed Inter Regular/SemiBold TTFs, fifteen Lucide SVGs and licenses. Freeze every exact path/hash and the current atlas/metrics/hash bytes. Provenance archive URLs/hashes are records, never fetch instructions.

205 entries: U+0020..U+007E regular 95, then semibold 95, then 15 icons in current ICONS/provenance order. 1024² R8; 64² cells; 16 cells/row; unused cells zero. Font 40 px, x=8, baseline 48; placement `x=8+xmin, y=48-ymin-height`. Icons map 24² to 40² at inset 12. All integer extent/offset operations checked before allocation/copy. All advances finite. No cell overflow is silently clipped.

SVG profile: bounded local XML/path parser for `svg,path,circle,line,rect`, viewBox 0 0 24 24, fill none, currentColor stroke 2, round cap/join, rect rx 1; exact M/m L/l H/h V/v C/c S/s A/a Z/z grammar with repeated/relative signed fractional coordinates, arc flags and smooth reflection. Proposed parser bounds to freeze U00:64KiB/icon, 8192 tokens/nodes, depth 32, 4MiB arena, 4096 normalized segments/icon. Bounds must first cover all exact 15 sources and independently reviewed expansion vectors; a source outside them blocks profile approval rather than being truncated. Reject DTD/entities/CSS/scripts/external references/unsupported transforms/nonfinite geometry. Arc subdivision uses a reviewed finite geometric error bound in U03; it never claims pixel equality with resvg.

`6U0=U00;6U1=U01–U04;6U2=U05;6U3a=U06;6U3b-target=U07;6U3c=U08`; U09 drives these stages. U01 real platform measurements require applicable main H06 authorization. U02–U05 pure tests can proceed with synthetic masks. U06 needs generated outputs but no UI host; U07 consumes authorized host witnesses from main H23. U08 is an all-target join; U09 CLI implementation consumes module interfaces, not already passed real target receipts. Real CLI commands in earlier tasks are executed after U09 exists, and do not create a driver cycle.

### Task U00: Freeze source and independent atlas expectations

**Files:** Create `zig/tools/ui_atlas/contract.zig`, `zig/tools/ui_atlas/input.zig`, `zig/tools/ui_atlas/tests.zig`, `zig/tests/fixtures/atlas/sources.json`, `zig/tests/fixtures/atlas/independent-vectors.json`.

**Interfaces:** Consumes approved spec, H01 Digest/source receipt; current committed inputs read-only. Produces `validateSources(sources:*const SourceSet) AtlasError!void`; `cellFor(index:u16) AtlasError!CellId`; U00 public types above.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** `U00 source_fence` asserts `expectEqual(205, entry_count)`, `expectEqual(95,regular_count)`, `expectEqual(95,semibold_count)`, `expectEqual(15,icon_count)` and exact semantic order; `expectError(error.BadSource,validateSources(&tampered))`. Independently freeze threshold/distance/JSON/hash vectors, including absent optional fields and whitespace advance. No new generator generates its own expected file.
- [ ] **Step 2 — establish compilation.** Run `zig build test-atlas -Dcase=U00 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-atlas -Dcase=U00`. Expected: nonzero with test `U00` failing the inert source valid at or accepts a changed font hash; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Read local inputs into an owned arena; authenticate each before platform calls. Freeze exact current metrics descriptor/entry schema and original generator operation order in a reviewed receipt. Define the bounded profile, reject duplicates/path traversal/unknown sources and source drift.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-atlas -Dcase=U00`, then `zig build test-atlas -Dcase=U00 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; source identities and all independent expected-vector authors/review hashes recorded; no archive URL opened.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task U01: Owned masks from pinned platform fonts

**Files:** Create `zig/tools/ui_atlas/raster.zig`, `zig/host/macos/nyon_raster.m`; extend atlas tests.

**Interfaces:** Consumes U00 SourceSet/Mask/RasterSettings and main H06a approved installed-framework ABI receipt. Produces `openRaster(a:Allocator,authorization:*const TargetAuthorization,settings:RasterSettings) AtlasError!RasterOwner`; `RasterOwner{raster:Raster,allocator:Allocator}` with `deinit(*RasterOwner) void`; `validateMask(mask:*const Mask) AtlasError!void`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** `U01 mask_validity` rejects inconsistent alpha length, dimension arithmetic overflow and NaN/Inf advances with validateMask. It does not call U03 geometry or U04 place/SDF. Font-byte selection and returned ownership are tested with independent synthetic masks; no generated-cell expectation is needed for this unit cycle. `font_bytes_only` asserts missing U+0041 yields MissingGlyph rather than system fallback. Allocation/callback failures release masks/context once.
- [ ] **Step 2 — establish compilation.** Run `zig build test-atlas -Dcase=U01 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-atlas -Dcase=U01`. Expected: nonzero with test `U01` failing a missing glyph or invalid mask is accepted by the raster boundary; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Open the exact TTF byte streams and explicit grayscale transform/hinting settings via CoreText/CoreGraphics. Return alpha 8/bearings/advance without atlas/SDF/manifest policy in foreign code. U01 implements font mapping/raster ownership only; its typed path callback returns Unavailable until U03 supplies the path implementation, and U01 tests do not invoke it. Complete 205-entry generation/compatibility measurement is the explicitly named follow-on checkpoint after U03–U05/U09 and before U06, not a prerequisite of U01's compiling RED/GREEN cycle.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-atlas -Dcase=U01`, then `zig build test-atlas -Dcase=U01 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; font-selection/mask-validity/ownership tests pass without importing U03/U04 implementation. Actual full-generation measurements belong to the follow-on checkpoint; no complete-atlas compatibility claim is made here.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task U02: Fail-closed XML and path parsing

**Files:** Create `zig/tools/ui_atlas/svg_lex.zig`, `zig/tools/ui_atlas/svg_parse.zig`; extend tests.

**Interfaces:** Consumes U00 profile/limits and Input. Produces `parseSvg(a:Allocator,input:Input) AtlasError!Svg`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** `U02 reject_external` uses `expectError(error.UnsupportedXml,parseSvg(...))` for DTD/entity/CSS/href/script/unknown attributes; `unsupported_command` rejects Q/q or any command not frozen above. Positive vectors cover all 15 actual sources, implicit coordinates, negative decimals, commas, arc flags 0/1; incomplete pairs/bad flags/depth 33/token 8193 fail before any output. Repeated prefixes must consume all input.
- [ ] **Step 2 — establish compilation.** Run `zig build test-atlas -Dcase=U02 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-atlas -Dcase=U02`. Expected: nonzero with test `U02` failing unsupported XML is accepted by the inert parser; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Implement bounded tokenizer and typed AST, retaining source spans and explicit attribute whitelist; never regex-extract path data or delegate to a network-capable parser. Freeze no permissive unknown-field behavior.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-atlas -Dcase=U02`, then `zig build test-atlas -Dcase=U02 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; every source completely consumed, negative vectors reject and allocator-failure sweep has no leaks.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task U03: Typed SVG geometry and raster handoff

**Files:** Create `zig/tools/ui_atlas/svg_geometry.zig`; extend tests and U01 raster path call.

**Interfaces:** Consumes U02 Svg, U00 Raster, spec transforms. Produces `geometry(a:Allocator,svg:*const Svg) AtlasError!*Geometry`; `rasterIcon(a:Allocator,raster:Raster,shape:*const Geometry,settings:RasterSettings) AtlasError!Mask`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** `U03 nonfinite_arc` rejects NaN/Inf/negative radii/invalidflag before raster; `smooth_reflection` checks exact reflected control points; independent quarter-circle, degenerate arc, rounded rect rx 1, relative close/reopen, round cap/join vectors pin geometry and maximum deviation before raster. Assert `expectEqual(12,transform.offset_x)` and `expectEqual(@as(f32,40.0/24.0),transform.scale)`; segment 4097 rejects. Raster mask bounds/AA measurements do not replace geometry vectors.
- [ ] **Step 2 — establish compilation.** Run `zig build test-atlas -Dcase=U03 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-atlas -Dcase=U03`. Expected: nonzero with test `U03` failing S/s reflection or arc endpoint is wrong; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Normalize to bounded typed segments, preserve sub path closure and stroke semantics. Freeze reviewed arc geometric tolerance and deterministic subdivision in U00 receipt before implementation; reject failure to achieve it under 4096 segments. Implement no undocumented transform. Platform handles stroke raster only.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-atlas -Dcase=U03`, then `zig build test-atlas -Dcase=U03 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; independent geometry expectations pass; measured raster differences remain explicit and cannot be erased by changing these fixtures.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task U04: Cell placement and exact signed distance bytes

**Files:** Create `zig/tools/ui_atlas/placement.zig`, `zig/tools/ui_atlas/sdf.zig`; extend tests.

**Interfaces:** Consumes U00 Mask/Cell/Atlas and reviewed independent masks. Produces `place(mask:*const Mask,cell:*Cell) AtlasError!void`; `distanceField(mask:*const Cell,out:*Cell) void`; `writeCell(atlas:*Atlas,id:CellId,cell:*const Cell) AtlasError!void`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** `U04 alpha_95_96_97` asserts inside false/true/true; `empty_zero` uses `expectEqualSlices(u8,&([_]u8{0}**4096),&out)`. Independently fixed full/border/corner/single-pixel vectors test outside-cell=false and half-away rounding. `bearing_overflow` includes x=-1 after origin adjustment, width65 and height overflow; expectError(error.CellOverflow,place(&mask,&cell)) before touching sentinel destination; id 205..255 untouched zero. Checked length and arithmetic boundaries must fail without partial copy.
- [ ] **Step 2 — establish compilation.** Run `zig build test-atlas -Dcase=U04 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-atlas -Dcase=U04`. Expected: nonzero with test `U04` failing an alpha 96 pixel or border distance differs from the independent expected byte; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Implement the exact source search: min_squared 64, dx/dy−8..8, opposite status only, replace strictly smaller; f32 sqrt/min 8; n=min(distance+0.5, 8)/8; inside 128+n*127, outside 128−n*128; half-away round with checked u8. No faster approximate distance transform or reassociation. Place only after full bounds validation.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-atlas -Dcase=U04`, then `zig build test-atlas -Dcase=U04 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; all 4096 bytes per vector equal, entire generated atlas length 1048576 and unused cells zero; failure paths do not alter approved inputs.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task U05: Exact metrics, source provenance and payload hashes

**Files:** Create `zig/tools/ui_atlas/metrics.zig`, `zig/tools/ui_atlas/hashes.zig`; extend tests.

**Interfaces:** Consumes U00 frozen schema, U04 Atlas, 205 finite Entry values, source licenses/provenance. Produces `serializeMetrics(a:Allocator,entries:[]const Entry) AtlasError![]u8`; `assembleOutputs(a:Allocator,sources:*const SourceSet,atlas:*const Atlas,entries:[]const Entry) AtlasError!Outputs`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** `U05 json_bytes` compares complete reviewed pretty JSON bytes, optional omission, finite f32 decimal text and exactly one trailing newline using `expectEqualSlices`; NaN/Inf/205→204/duplicate key reject. `lexical_hash_order` checks shuffled input traversal yields identical manifest; `provenance_no_fetch` retains exact two archive records and zero network calls. Perturb one cell/advance/license and assert distinct hash.
- [ ] **Step 2 — establish compilation.** Run `zig build test-atlas -Dcase=U05 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-atlas -Dcase=U05`. Expected: nonzero with test `U05` failing serializer uses a different field order or emits a nonfinite advance; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Implement explicit format_version 1 field order/spacing/optional omissions from U00; source paths lexical, then metrics/atlas hashing in the existing contract. Own all returned slices. Baseline hashes are comparison inputs, never overwritten to make output pass.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-atlas -Dcase=U05`, then `zig build test-atlas -Dcase=U05 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; exact serializer/hashing vectors pass, including independent known SHA256 vectors and current output comparison report.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### U01 follow-on measurement checkpoint: complete 205-entry outputs

This is separate from U01's unit GREEN. After reviewed U03 geometry, U04 placement/SDF, U05 serialization and U09 CLI interfaces exist, the authorized macOS raster measurement invokes the complete pipeline twice and records all 205 entries' mask validity, advances/bearings, generated-cell differences and complete atlas/metrics/hash payload comparisons. The existing raw fontdue/resvg masks are not supplied; do not invent their bytes or treat platform mask similarity as old-generator proof. Complete output comparison and independent mask/geometry/SDF expectations remain the evidence.

- [ ] Run `zig build atlas-tool -- measure --full205 --approval ../docs/qualification/platform/atlas-dispatch.json --out zig-out/qualification/atlas-measure` after U09 driver registration. Expected: exit 0 only for a complete measurement report, exit 2 for an unavailable authorized raster prerequisite and exit 1 for a measurement/structural failure; exit 0 is not a compatibility claim.
- [ ] Independently review the 205-entry report, exact source/raster/tool identities and repeated output hashes before U06 attempts admission. Any missing entry blocks admission. Mismatches still fail exact-compatible U06; only a separately approved changed-version proposal may enter the unchanged candidate → target witness → all-target release path.

### Task U06: 6U3a isolated structural and reproducibility candidate admission

**Files:** Create `zig/tools/ui_atlas/candidate.zig`; extend tests.

**Interfaces:** Consumes U05 Outputs, H01 Approval/Candidate, U00 baseline and source identities; U01 raster identity. Produces `admit(a:Allocator,outputs:*const Outputs,repeated:*const Outputs,sources:*const SourceSet,mode:AssetMode,approval:*const Approval,raster:RasterSettings) ArtifactError!OwnedCandidate`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** `U06 candidate_before_host` asserts `expectEqual(.qualification_candidate,candidate.kind)` with target UI checks pending and no host requirement. `mismatch_needs_version_approval` rejects any changed baseline byte in exact_compatible mode; changed_version requires matching approved proposal digest/scope and complete old/new metrics/pixel diff. `changed_identity_invalidates` changes tool/raster/source with same output and requires new candidate ID. Repeat generation mismatch rejects; copying baseline instead of invoking generator is rejected by generator-source/input receipt provenance. `release_rejects_candidate` exercises H01 loader.
- [ ] **Step 2 — establish compilation.** Run `zig build test-atlas -Dcase=U06 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-atlas -Dcase=U06`. Expected: nonzero with test `U06` failing exact-compatible admission accepts a changed pixel or requires a future UI witness; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Authenticate all sources and independently validate shape/metrics/SDF/hash expectations; compare two actual independent invocations under pinned settings. Create immutable manifest under candidate-ID qualification directory with all payload lengths/hashes/tool/profile/raster/mode/approval references and pending target checks. Preserve committed assets. Report full differences on incompatibility; a changed-version proposal is an external authorization prerequisite, never an automatic switch.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-atlas -Dcase=U06`, then `zig build test-atlas -Dcase=U06 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; compatibility rejection and approved-version test fixtures pass; real exact-compatible exit 0 only if all three generated payloads match frozen baseline; otherwise explicit gap remains.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task U07: 6U3b target witness admission

**Files:** Create `zig/tools/ui_atlas/witness.zig`; extend tests.

**Interfaces:** Consumes U06 Candidate, main H23 real UI/image/layout/scale/accessibility Witness values bound to exact host/shader/asset and environment. Produces `qualifyTarget(a:Allocator,candidate:*const Candidate,target:Target,required:[]const Requirement,witnesses:[]const Witness) ArtifactError!AssetTargetReceipt`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** `U07 target_rows_complete` removes one viewport/scale/IME/accessibility row and expects AdmissionFailed; mismatched shader/host/fixture identity expects StaleEvidence. `candidate_before_host` confirms admission predates UI evidence; `missing_target_pending` asserts complete macOS witnesses produce only macOS-qualified result while Windows/Linux absent remain pending. `borrowed_witness_lifetime` makes explicit caller-owned slices remain live through receipt serialization.
- [ ] **Step 2 — establish compilation.** Run `zig build test-atlas -Dcase=U07 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-atlas -Dcase=U07`. Expected: nonzero with test `U07` failing one missing required UI row is treated as passed; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Validate every applicable target/browser/backend row and exact asset/shader pairing without rebuilding payloads. All 205 glyph/icon sheets check bleed/strokes/whitespace/round caps/caret alignment at full matrix. Do not execute host logic here; consume H23 evidence with explicit actual result/exit/capture/manual observer identities.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-atlas -Dcase=U07`, then `zig build test-atlas -Dcase=U07 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; pure receipt tests pass; actual U07 closure remains pending until main H23 executes the complete target journeys; mock tests never establish a host pass.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task U08: 6U3c all-target release asset promotion

**Files:** Create `zig/tools/ui_atlas/promote.zig`; extend tests.

**Interfaces:** Consumes U06 candidate, U07 AssetTargetReceipt and complete H01 requirement set. Produces `promote(a:Allocator,candidate:*const Candidate,receipts:[]const AssetTargetReceipt,required:[]const Requirement,approval:*const Approval) ArtifactError!ReleaseManifest`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** `U08 cross_version_receipt_rejected` rejects an old candidate receipt with identical visible glyphs but different raster identity; `missing_target_pending` expects IncompleteMatrix when any required native/browser/backend row missing. `tag_flip_rejected` asserts release-kind alone without witness closure fails. Mutated payload after target receipt rejects on promotion rehash; promotion changes manifest kind/closure only, payload bytes unchanged.
- [ ] **Step 2 — establish compilation.** Run `zig build test-atlas -Dcase=U08 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-atlas -Dcase=U08`. Expected: nonzero with test `U08` failing promotion succeeds after dropping the Ubuntu forced-GL row; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Rehash payloads and validate same selected asset identity, complete required target witness closure and exact approval scope. Emit a new release manifest atomically into owned qualification output; this is 6C-assets-release, not file replacement or complete 6M approval. No regeneration or opaque retagging. Main H25 consumes this manifest; H26 rechecks exact delivered bytes.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-atlas -Dcase=U08`, then `zig build test-atlas -Dcase=U08 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; all-target join tests pass; real promotion waits for all target evidence. Asset overwrite/source retirement remain separately unauthorized.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task U09: Owned offline atlas CLI and stage exits

**Files:** Create `zig/tools/build_ui_atlas.zig`; register `atlas-tool` in `zig/build.zig` through H01 seam.

**Interfaces:** Consumes U00–U08 interfaces; H01 owned output/approval handling. Produces `run(a:Allocator,io:std.Io,args:[]const []const u8) ArtifactError!u8` with inspect/measure/generate/admit/qualify/promote subcommands.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** `U09 offline_only` injects unavailable raster and asserts exit 2, no sub process acquisition, no writes to assets/ui; invalid scope returns 1. Repeated stage command preserves existing immutable candidate; no cache-only fallback/real-profile path. Test exact stage output and pending row list.
- [ ] **Step 2 — establish compilation.** Run `zig build test-atlas -Dcase=U09 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-atlas -Dcase=U09`. Expected: nonzero with test `U09` failing unavailable raster succeeds using copied old outputs; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Keep all orchestration in Zig and dispatch only declared platform calls. Exit 0 means requested stage satisfied, 2 unavailable authorized prerequisite, 1 validation failure. CLI parses explicit output/candidate/approval paths; no default real-profile or release output. No Rust/Python/shell sub process.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-atlas -Dcase=U09`, then `zig build test-atlas -Dcase=U09 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; after scoped dispatch run `zig build atlas-tool -- inspect --sources ../assets/ui`; it reports 205 entries and exact input hashes. Authorized `zig build atlas-tool -- generate --approval ../docs/qualification/platform/atlas-dispatch.json --out zig-out/qualification/atlas` creates isolated outputs; `zig build atlas-tool -- admit --candidate zig-out/qualification/atlas/candidate.json --mode exact-compatible` returns 0 only for actual complete compatibility, otherwise 1 with mismatch report. No current claim of those commands existing or passing.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

## Exit contract and self-review

U00 freezes actual bytes and reviewed expectations; U01 and U02→U03 supply masks; U04→U05 own deterministic transformations; U06 admits candidates; main target hosts→H23 supply witnesses→U07 target admission→U08 all-target release manifest. U09 is an interface consumer/driver, not a prerequisite result that its own modules must already have produced. Unavailable Windows/Linux does not block the U01 macOS measurement or a macOS U07 receipt, but U08 remains pending.

All five Review Focus cases have owning tests. Independent synthetic masks, source geometry vectors and immutable complete existing output comparisons are distinct from new generator output. Arc tolerance must be explicitly reviewed before U03, never widened to hide raster drift; proposed parser caps are not claims about implementation support. Exact compatibility remains unverified. If it fails, preserve assets and the generator gap until a changed-version proposal receives separate approval and qualification. Scope is smaller than the compiler plan because rasterization is a narrow approved platform boundary, not an owned font engine. SDD/user plan review/implementation dispatch and acquisition gates remain separate.
