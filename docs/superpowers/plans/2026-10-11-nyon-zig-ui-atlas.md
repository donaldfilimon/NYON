# NYON owned Zig UI atlas implementation plan

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Canceled on 2026-10-19:** Donald canceled further Zig rewrite work. Unfinished Zig tasks and continuation instructions in this document are superseded; retain its historical evidence and existing source. See [the current implementation decision](../../../REWRITE-DECISION-2026-10-19.md).
<!-- END 2026-10-19 rewrite decision -->

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Donald selected subagent development. Dispatch one bounded implementer, then a fresh specification reviewer and a fresh code-quality reviewer for each task; resolve findings before its dependent task starts.

**Goal:** Replace the Rust atlas generator with an owned, deterministic Zig V2 generator, authenticated candidate publication, and independently qualified retained-UI admission.

**Architecture:** Pure parsers feed fixed geometry, coverage, SDF, packing and serialization. Owned IO authenticates compiled sources and publishes candidates separately from qualified releases. Tasks 1–10 stay isolated; Task 11 hands integration/retirement to existing owners.

**Tech Stack:** Exactly Zig `0.18.0-dev.146+35accc06e`, project source and pinned Zig std; no new dependency, foreign rasterizer, font service, Rust, Python, JavaScript or shell subprocess in the delivered generation/check graph.

**Spec:** `docs/superpowers/specs/2026-10-11-nyon-zig-ui-atlas-design.md`, SHA-256 `b65ded7fbb2b7b920e6ec4a72b7c91009bb52d5c0188afcd1865832ee5b4f0db`. Read it alongside this plan. `docs/qualification/2026-10-11-zig-ui-atlas-design-fix2-review.md` is PASS **for planning**; preserve that review and both earlier raw reviews.

## Global constraints

- Proposed implementation; no generator, filesystem durability, cross-host, runtime, visual or retirement qualification is established by this plan.
- Use the canonical shared checkout; preserve other agents' changes. No branch/worktree, commit, push, AGENTS edit or source deletion is part of these tasks.
- Own only new `zig/tools/ui_atlas/**`, `zig/tools/build_ui_atlas.zig`, `zig/ui_atlas_tests.zig`; later tasks may update these. Existing build/root/main/gates/retained runtime/assets/Cargo remain read-only until Task 11 owner handoff.
- Asset identity `nyon-ui-atlas-v2`; 205 entries, 1024×1024 R8, 64×64 cells, 16 cells/row, 190 printable-ASCII glyphs and 15 icons. Preserve all 190 V1 advance f32 bits, font origin X=8, baseline Y=48, size=40, icon advance=40, threshold=96, spread=8. No user-save changes.
- Font ≤1 MiB each; SVG ≤16 KiB each; license ≤64 KiB each; source manifest ≤64 KiB; tables ≤32; glyphs ≤4,096; contours ≤128/glyph; expanded points ≤4,096/glyph; composite depth ≤8 and references ≤64/glyph; instructions ≤65,535 bytes/glyph; SVG elements/subpaths ≤128 each and operands ≤4,096/icon; segments ≤8,192/entry; subdivision depth ≤16; numeric token ≤32 bytes; generation arena ≤64 MiB. Process one entry at a time; checked capacity refusal never drops geometry.
- Q32.32 coordinates, checked i256 intermediates, half-away rounding, exact operation order from the spec; no floating raster math. 8×8 centered samples, nonzero winding fonts, capsule-union strokes, frozen independent 65-pair SDF LUT.
- Preserve original source fonts/icons/licenses, archive provenance, V1 reference hashes/metrics and all existing gates. Rust retirement is authorized only after equivalent replacement and delivered-artifact qualification, never as an upfront source wipe.
- CLI generation succeeds only as `CandidateDurable`; promotion only as `CommittedDurable`; read-only checks only as `Verified`. These are separate types. Unsupported native durability refuses publication before its boundary; model tests cannot establish power-loss guarantees.
- Native Metal/DX12/Vulkan/approved GL and browser WebGPU/WebGL2 acceptance, actual host generation determinism and final Rust-free package evidence are separate gates. Missing evidence remains **unverified**.

## Review focus

1. Old executable beside edited sources, renamed executable or substituted receipt: every CLI must authenticate the actual running image and reject stale/forged binding (Tasks 7, 9).
2. Crash-left candidate and cancellation during its rename/re-sync: exact existing bytes must be re-synced; selection must never be accessed by generation (Task 8).
3. Valid-looking malformed fonts/SVGs that exceed expanded work rather than input bytes: refuse atomically with checked arithmetic and no leaks (Tasks 2–5).
4. Behavior-neutral source changes and relocated output: source edits change candidate identity, while output-root relocation cannot (Tasks 6, 7, 9).
5. Same-size corrupted atlas or rejected metadata replacing resident state: verify expected identity and payload hashes before schema/atomic replacement; preserve layout/alias budgets (Task 10).

## Ownership, commands and test protocol

Read-only references: `tools/build-ui-atlas.rs`, `tools/ui-atlas/Cargo.toml`, `assets/ui/{atlas-metrics.json,hashes.json}`, `zig/src/{retained_atlas,retained_text}.zig` and `zig/build.zig`. Existing build has no atlas steps. V1 pixels are comparison data, not a V2 oracle.

Create `zig/ui_atlas_tests.zig`, importing each task's tests explicitly (`comptime { _ = @import("tools/ui_atlas/profile_tests.zig"); }`). Production CLI imports `ui_atlas/cli.zig` and Task 7's generated named identity module. From repository root:

```sh
/Users/donaldfilimon/.zvm/bin/zig version
/Users/donaldfilimon/.zvm/bin/zig test zig/ui_atlas_tests.zig -O Debug
/Users/donaldfilimon/.zvm/bin/zig test zig/ui_atlas_tests.zig -O ReleaseSafe
/Users/donaldfilimon/.zvm/bin/zig fmt --check zig/ui_atlas_tests.zig zig/tools/ui_atlas
```

Compiler version was verified during planning; new-file commands are **proposed**. For every task, use the absolute compiler above with `--test-filter` and the quoted test name, first on a compiling behavioral stub such as `return error.UnsupportedProfile;`. Record actual nonzero test count, assertion/error mismatch and exit. Compile errors and zero matched tests are not RED. Implement, run the exact test and both complete isolated modes, then fresh specification and quality reviewers. Fix findings before dependencies proceed. No commits. Execution evidence stays in ignored `zig/zig-out/qualification/ui-atlas/`; preserve other documents/ledgers.

In snippets `t=std.testing`; modules import the named local production/fixture modules. Signatures are owned project contracts; inspect pinned std APIs before implementing adapters. Tests must not reference unavailable future helpers. All errors map to the shared set; all allocations use the accounted context.

## Shared types and file map

Task 1 creates `types.zig`, `profile.zig`, `fixed.zig`, `fixtures/{numeric.zig,sdf_pairs.zig,v1_advances.zig,PROVENANCE.md}`, `profile_tests.zig`, `fixed_tests.zig`, and the test root. Tasks 2–11 create the exact additional files listed below. Fixture source is handwritten/reviewed evidence, not generated product goldens. No subdirectory contains an independent dependency manifest.

`types.zig` owns:

```zig
pub const Hash = [32]u8;
pub const Cell = [4096]u8;
pub const Atlas = [1048576]u8;
pub const Point = struct { x: i64, y: i64 }; // raw Q32.32
pub const Segment = struct { a: Point, b: Point };
pub const FillMode = enum { nonzero, stroke_union };
pub const AtlasError = error{
    InputHashMismatch, UnsupportedProfile, MalformedBounds, UnsupportedScalar,
    MissingGlyph, NumericOverflow, NumericDegeneracy, Capacity, CellOverflow,
    OutOfMemory, StaleOutput, StaleExecutable, ConcurrentModification,
    PublicationConflict, UnsupportedDurablePublication, Io, Cancelled,
    InvalidReceipt, InvalidSelection, InvalidArguments, AliasConflict,
};
pub const Budget = struct { live: usize = 0, peak: usize = 0,
    limit: usize = 64 * 1024 * 1024, denied_limit: bool = false };
pub const Context = struct { allocator: std.mem.Allocator, budget: *Budget };
pub const Shape = struct { segments: []Segment, mode: FillMode, radius: i64,
    pub fn deinit(self: *Shape, ctx: *Context) void;
};
pub const Entry = struct { index: u16, advance_units: u16 };
pub const BuildIdentity = struct { closure: Hash, compiler_version: []const u8 };
pub const SourceSet = struct { regular: []const u8, semibold: []const u8,
    icons: [15][]const u8, licenses: [2][]const u8,
    manifest: []const u8, build: BuildIdentity };
pub const Outputs = struct { atlas: []u8, metrics: []u8, hashes: []u8, manifest: []u8,
    pub fn deinit(self: *Outputs, ctx: *Context) void;
};
```

`Entry.advance_units` is selected glyph hmtx units; icons use 2048, hence 40 pixels. Index derives identity/cell deterministically from `profile.zig`. `Shape` segments are caller-owned and its mode/radius are explicit; closure occurs in parser/geometry, not raster guessing. Returned slices have exactly one owner and never alias immutable inputs or stack locals. Task 1 provides `budget.Accounted{backing:std.mem.Allocator,budget:*Budget}` with `allocator(self:*Accounted) std.mem.Allocator`; it accounts alloc/resize/remap/free and latches budget refusal separately from backing OOM. Keep this wrapper alive through allocation release. Translate latched limit refusal to Capacity, other allocation refusal to OutOfMemory; inject each allocation failure.

### Task 1: Freeze independent fixtures and checked numerical/profile contracts

**Files:** Create the Task 1 files above plus `zig/tools/ui_atlas/budget.zig`. This task owns all shared types and constants; subsequent changes require consumer review.

**Interfaces:** `profile.entry(index: u16) AtlasError!EntryIdentity` with `EntryIdentity` a union of `{glyph: {codepoint:u21, semibold:bool}, icon: enum{play,pause,speed,reset,settings,help,crosshair,save,load,check,close,zoom,field_atmosphere,field_hydrosphere,field_topology}}`; `profile.requireScalar(u21) AtlasError!void`; `fixed.roundRatio(i256,i256) AtlasError!i64`, `fixed.qsqrt(i256,i256) AtlasError!i64`, `fixed.decimal([]const u8) AtlasError!i64`, `fixed.sourcePosition(i64) AtlasError!i64`, `fixed.sourceLength(i64) AtlasError!i64`. Denominators must be positive; intermediate/narrowing overflow rejects.

- [ ] Before implementation freeze all 190 V1 advance bits, table/glyph mappings, 65 SDF pairs and square/triangle/disk/capsule/arc samples including coverage 0..64. Derive from independent raw font offsets, V1 JSON and high-precision/rational arithmetic. Keep bounded extractor `fixtures/reference.zig`; `fixtures/PROVENANCE.md` records inputs/hashes/calculations/method/reviewer. Review every SDF pair against V1 formula; no production helpers/output as oracle.
- [ ] Add compiling stubs and behavioral tests:

```zig
test "atlas fixed ratios and midpoint square root" {
    try t.expectEqual(@as(i64, -2), try fixed.roundRatio(-3, 2));
    try t.expectEqual(@as(i64, 1), try fixed.qsqrt(2, 1));
    try t.expectEqual(@as(i64, 2), try fixed.qsqrt(9, 4));
    const q = try fixed.decimal("12.8");
    try t.expectEqual(@as(i64, 54975581389), q);
    try t.expectEqual(@as(i64, 91625968982), try fixed.sourceLength(q));
    try t.expectEqual(@as(i64, 143165576534), try fixed.sourcePosition(q));
    try t.expectError(error.NumericOverflow, fixed.roundRatio(1, 0));
}
```

- [ ] Capture RED: `atlas fixed ratios and midpoint square root`. Test ±half/near-half extremes, sqrt boundaries, smallest vector, narrowing overflow, invalid decimal/exponent/token/six-place limits, 205 identities/scalar refusal and allocation/resize/remap/failure live=0.
- [ ] Implement annex R/Qsqrt with checked i256; midpoint=`4*N >= D*(2*k+1)^2`, scale=`R(5*q,3)`. Freeze revision/limits/icon map. Both modes/reviews.

### Task 2: Bounded sfnt table admission, cmap and metrics

**Files:** Create `zig/tools/ui_atlas/{ttf.zig,ttf_tests.zig,fixtures/ttf.zig}`; update test root. Production consumes byte slices. Test-only bounded reads use `std.Io.Dir.cwd().readFileAlloc(t.io, path, allocator, .limited(cap))` from repository root, verify frozen hash, then pass slices; no out-of-package embedFile or dependency on future IO helpers.

**Interfaces:** Consumes `Context`, `AtlasError`, profile limits. Produces `ttf.Font` (borrowed immutable font bytes and bounded table descriptors), `ttf.open(bytes: []const u8) AtlasError!Font`, `Font.glyphId(codepoint: u21) AtlasError!u16`, `Font.advance(glyph_id: u16) AtlasError!u16`. `Font` borrows its input through the caller's use; no hidden allocation. `fixtures/ttf.zig` defines independent literal complete font `trailing_hmtx` with two mapped glyphs sharing final advance 576 and separate bearings, correct independent checksums, plus `bad_range_offset`.

- [ ] Write the admission test against compiling stubs:

```zig
test "atlas ttf trailing hmtx and bounded cmap" {
    const font = try ttf.open(&fixture.trailing_hmtx);
    try t.expectEqual(@as(u16, 576), try font.advance(2));
    try t.expectEqual(@as(u16, 2), try font.glyphId('!'));
    try t.expectError(error.MalformedBounds, ttf.open(&fixture.bad_range_offset));
}
```

- [ ] Capture RED: `atlas ttf trailing hmtx and bounded cmap`. Test every header/table truncation, duplicate/unknown/padded-overlap/checksum errors, offset overflow, table/glyph cap±1, loca order/end, odd range offsets, terminal cmap, delta wrap/zero result and nonselected cmap bounds. Production requires 95 ASCII mappings; no format 12 fallback.
- [ ] Implement spec table allowlists, sfnt0x00010000, table/whole-font checksum, unitsPerEm2048, long loca, hhea/hmtx/trailing bearings, platform 3/encoding 1/format 4. Bounds precede slices. Compare all frozen mappings/advances; distinguish empty space/missing glyph. Both modes/reviews.

### Task 3: Simple/composite font outlines and deterministic flattening

**Files:** Create `zig/tools/ui_atlas/{geometry.zig,geometry_tests.zig,outline_tests.zig}`; extend `ttf.zig`, `fixtures/ttf.zig` and test root.

**Interfaces:** `ttf.outline(ctx:*Context,font:*const Font,glyph_id:u16) AtlasError!Shape`; `geometry.flattenQuadratic(ctx:*Context,p0:Point,p1:Point,p2:Point) AtlasError!Shape`; `geometry.flattenCubic(ctx:*Context,p0:Point,p1:Point,p2:Point,p3:Point) AtlasError!Shape`. Shapes from these helpers have `.nonzero`, radius=0; SVG changes mode only when assembling its final stroked shape. Every flatten call and concatenation charges the same per-entry segment/work budget. Contour endpoint/point storage is temporary, owned and released before returning segments.

- [ ] Add `fixtures.ttf` literals `translated_composite` and `cycle`; selected glyph advance is 576 even if USE_MY_METRICS child differs. Independently freeze expected segment points (including implied midpoint wrap) in `fixtures/numeric.zig`.

```zig
test "atlas composite translation preserves selected advance" {
    var budget: types.Budget = .{};
    var accounted: budget_mod.Accounted = .{ .backing=t.allocator, .budget=&budget };
    var ctx: types.Context = .{ .allocator=accounted.allocator(), .budget=&budget };
    const font = try ttf.open(&fixture.translated_composite);
    var shape = try ttf.outline(&ctx, &font, 2);
    defer shape.deinit(&ctx);
    try t.expectEqual(@as(u16, 576), try font.advance(2));
    try t.expectEqualDeep(@as([]const types.Segment, &numeric.translated_segments), shape.segments);
}
```

- [ ] Capture RED: `atlas composite translation preserves selected advance`. Test flag repeat/delta/instruction overflow, off-curve wrap, holes, nested translations/cycles, depth/reference/point cap±1, conflicting USE_MY_METRICS, forbidden attachment/transforms and every allocation failure.
- [ ] Decode simple/translation-only composites; never execute instructions. Translate in font units before annex cell mapping; preserve winding/closure/selected advance. Flatten left-first with stagewise R(a+b,2), ≤1/256 distance to endpoint segment including outside projections; reject unresolved depth 16/segments 8192/cell hull overflow. Test seven composites in both fonts and 190 advance bits. Both modes/reviews.

### Task 4: Strict SVG tokenizer, path state and circular geometry

**Files:** Create `zig/tools/ui_atlas/{svg.zig,svg_tests.zig,fixtures/svg.zig}`; extend `geometry.zig`, `geometry_tests.zig`, numeric fixtures and test root.

**Interfaces:** `svg.parse(ctx:*Context,bytes:[]const u8) AtlasError!Shape`; `geometry.Arc = struct { start:Point,end:Point,radius:i64,large:bool,sweep:bool }`; `geometry.ArcConstruction = struct { center:Point,radius:i64,midpoint:Point }`; `geometry.constructArc(Arc) AtlasError!ArcConstruction` for noncoincident/nonzero-radius arcs; `geometry.flattenArc(ctx:*Context,arc:Arc) AtlasError!Shape` handles no-op/line degeneracies. Final SVG shape has `.stroke_union` and radius `R(5*S,3)`.

- [ ] Add literal XML fixtures and test the normative numeric example before implementing arc construction:

```zig
test "atlas svg corrected clockwise semicircle" {
    const s: i64 = 1 << 32;
    const arc = try geometry.constructArc(.{ .start = .{ .x=0, .y=0 },
        .end=.{ .x=4*s, .y=0 }, .radius=s, .large=false, .sweep=true });
    try t.expectEqual(@as(i64, 2*s), arc.radius);
    try t.expectEqual(types.Point{ .x=2*s, .y=0 }, arc.center);
    try t.expectEqual(types.Point{ .x=2*s, .y=-2*s }, arc.midpoint);
}
```

- [ ] Capture RED: `atlas svg corrected clockwise semicircle`. Test four flags, quarter midpoint 3037000500, antipodal/major/branch crossings, tiny vector, coincident/zero/negative/unequal radius, rotation refusal, ordered endpoints, rounded-rect floor(extent/2) clamp and normalization degeneracy.
- [ ] Implement spec XML/decimal/root grammar; only path/circle/line/rect. Reject unknown/duplicate attributes, entities/comments/DTD/PI/groups/style/transforms/resources. Parse relative/absolute M/L/H/V/C/S/A/Z, repetition/M-to-L, sign separators, close/reopen and reflection reset before scaling once. Preserve annex arc order and clockwise circle/rect construction. Test malformed/trailing/Q/T, empty/zero-length paths, caps±1, allocations and 15 actual icons. Both modes/reviews.

### Task 5: Coverage, fixed SDF and deterministic cell packing

**Files:** Create `zig/tools/ui_atlas/{raster.zig,sdf.zig,packing.zig,raster_tests.zig,sdf_tests.zig,packing_tests.zig}`; update test root.

**Interfaces:** `raster.coverage(shape:*const Shape,out:*Cell) AtlasError!void`; `raster.coverageByte(count:u7) AtlasError!u8`; `sdf.encode(alpha:*const Cell,out:*Cell) AtlasError!void`; `packing.write(atlas:*Atlas,index:u16,cell:*const Cell) AtlasError!void`; `packing.origin(index:u16) AtlasError!struct{x:u16,y:u16}`. Callers own distinct fixed buffers; overlap errors must be checked where aliases are possible. Never allocate per sample. SDF reads original alpha, not partially encoded output.

- [ ] Test alpha threshold and the independent LUT:

```zig
test "atlas sdf threshold empty and single pixel" {
    var alpha: types.Cell = @splat(0);
    var encoded: types.Cell = undefined;
    alpha[32*64+32] = 95;
    try sdf.encode(&alpha, &encoded);
    try t.expectEqualSlices(u8, &(@as(types.Cell, @splat(0))), &encoded);
    alpha[32*64+32] = 96;
    try sdf.encode(&alpha, &encoded);
    try t.expectEqual(fixture.pairs[1].inside, encoded[32*64+32]);
    try t.expectEqual(fixture.pairs[1].outside, encoded[32*64+33]);
}
```

- [ ] Capture RED: `atlas sdf threshold empty and single pixel` (zero-fill stub). Test independent coverage 0..64, square/triangle/disk/capsule, edge ties, horizontal/half-open crossing, holes/union, full/single/border/corner masks, threshold 95/96/97, exterior/strict min and 65 literal LUT pairs.
- [ ] Implement odd-sixteenth samples, `(count*255+32)/64`, checked i256 inclusion; reject cell overflow. SDF scans±8, initial distance²64, strict smaller opposite-class replacement, empty→zero. Fixed packing rejects index≥205, unused cells zero. Test bleed/alias/length/overflow. Both modes/reviews.

### Task 6: Canonical metrics, manifests and pure generation

**Files:** Create `zig/tools/ui_atlas/{metrics.zig,manifest.zig,generator.zig,generator_tests.zig,manifest_tests.zig,fixtures/atlas-inputs-v2.json}`; update test root. The fixture manifest is independently authored source authority in this isolated phase; Task 11 owns installing identical approved bytes at the spec's `assets/ui/atlas-inputs-v2.json` path and rebuilding identities.

**Interfaces:** `metrics.serialize(ctx:*Context,entries:*const [205]Entry) AtlasError![]u8`; `manifest.hashes(ctx:*Context,sources:*const SourceSet,atlas:[]const u8,metrics:[]const u8) AtlasError![]u8`; `manifest.encode(ctx:*Context,sources:*const SourceSet,atlas:[]const u8,metrics:[]const u8,hashes:[]const u8) AtlasError![]u8`; `manifest.candidateId(manifest_bytes:[]const u8) Hash`; `generator.generate(ctx:*Context,sources:*const SourceSet) AtlasError!Outputs`; `generator.verifyOutputs(ctx:*Context,sources:*const SourceSet,outputs:*const Outputs) AtlasError!void`; `manifest.requirePath(path:[]const u8,scope:enum{repository,artifact}) AtlasError!void`. Hash using pinned `std.crypto.hash.sha2.Sha256`, no process.

- [ ] Freeze exact tiny serializer examples and path tests before writing serializers:

```zig
test "atlas artifact paths forbid publication identity" {
    try manifest.requirePath("atlas.r8", .artifact);
    try t.expectError(error.MalformedBounds, manifest.requirePath("../atlas.r8", .artifact));
    try t.expectError(error.MalformedBounds, manifest.requirePath("generated/atlas.r8", .artifact));
    try t.expectError(error.MalformedBounds, manifest.requirePath("assets//ui/a", .repository));
}
```

- [ ] Capture RED above; test absolute/colon/backslash/NUL/percent/Unicode/dot/empty/extra/missing paths. Authored input manifest pins 19 sources (2fonts,15icons,2licenses), both archives/profile/order; old21-record manifest also has two outputs. Independently rehash originals; preserve licenses/Feather notice.
- [ ] Implement 205 ordered entries, V1 descriptor/field order, format_version2, exact dyadic decimals/integral `.0`, two-space/LF/single newline/≤64KiB. Hash graph: sorted repository sources then metrics/atlas artifact basenames; manifest→atlas/metrics/hashes, candidate ID→manifest. Include profile/source-manifest/closure/compiler version; exclude executable/receipts/output paths/time. Generate one entry at a time, check190 advance bits. Test relocation, neutral-source identity change, all payload/hash mutations, shuffled traversal, allocation failure/arena peak. Twice regenerate fresh in both modes; compare four payloads and emit all 205-cell V1/V2 differences: changed count,max delta,bounds,threshold area,advances. Differences are evidence, not acceptance. Fresh reviews.

### Task 7: Owned IO, immutable compiled snapshot and trusted executable receipt

**Files:** Create `zig/tools/ui_atlas/{io.zig,io_posix.zig,io_windows.zig,inputs.zig,attest.zig,build_driver.zig,identity_template.txt,source-closure.json,toolchain-inventory.json,attest_tests.zig,io_tests.zig,fixtures/io_recorder.zig}`; update test root. Build outputs/snapshots are ignored, never hand-edited generated source.

**Interfaces:** `io.Fs` is an owned capability object with explicit methods below, backed by pinned `std.Io`; only `build_driver` may invoke the pinned compiler. `io.Handle` is opaque; `io.Stat` carries kind, size and stable identity. All paths resolve from an opened root handle without following symlinks; inspect every ancestor. `io.NativeCapabilities = struct{candidate_durable:bool,selection_durable:bool}`; `RenameResult = enum{installed,not_installed,ambiguous}`. Methods return `AtlasError!T` except rename ambiguity is the result, never collapsed into Io:

```zig
openRoot(path:[]const u8) AtlasError!Handle;
openDirectory(root:Handle,path:[]const u8) AtlasError!Handle;
openFile(root:Handle,path:[]const u8) AtlasError!Handle;
capabilities(root:Handle) NativeCapabilities;
readBounded(ctx:*Context,root:Handle,path:[]const u8,cap:usize) AtlasError![]u8;
stat(root:Handle,path:[]const u8) AtlasError!Stat;
createDirectory(root:Handle,path:[]const u8) AtlasError!Handle;
createExclusive(root:Handle,path:[]const u8) AtlasError!Handle;
writeAll(file:Handle,bytes:[]const u8) AtlasError!void;
syncFile(file:Handle) AtlasError!void;
syncDirectory(dir:Handle) AtlasError!void;
renameNoReplace(root:Handle,from:[]const u8,to:[]const u8) RenameResult;
replace(root:Handle,from:[]const u8,to:[]const u8) RenameResult;
removeOwned(root:Handle,path:[]const u8,owner:Hash) AtlasError!void;
close(handle:Handle) void;
```

`inputs.OwnedSources { value:SourceSet, pub fn deinit(*OwnedSources,*Context) void; }`; `inputs.read(ctx:*Context,fs:*io.Fs,root:io.Handle,identity:BuildIdentity) AtlasError!OwnedSources` stat/read/stat-checks exact allowlisted bytes once, hashes those buffers and rejects drift. `attest.Snapshot` owns sorted `{path:[]const u8,bytes:[]const u8,sha256:Hash}` records with `deinit(*Snapshot,*Context) void`; `attest.snapshot(ctx:*Context,fs:*io.Fs,root:io.Handle) AtlasError!Snapshot`; `attest.digest(snapshot:*const Snapshot,version:[]const u8) Hash`; `attest.authenticate(ctx:*Context,fs:*io.Fs,root:io.Handle,receipt_path:[]const u8,embedded:BuildIdentity) AtlasError!Session`; `Session` owns original closure/input hashes and actual running-image identity, with `revalidate(*Session,*Context,*io.Fs) AtlasError!void` and `deinit(*Session,*Context) void`. It validates input changes once sources are attached via `Session.bindInputs(*Session,*const SourceSet) void`.

- [ ] Add independent digest framing fixture and recorder. Recorder owns a bounded in-memory filesystem, appends typed operation/path records, injects a specified failure/cancellation callback by operation ordinal, and rejects out-of-scope paths; `init(backing:std.mem.Allocator) AtlasError!Recorder`, `Recorder.deinit(*Recorder) void`, `Recorder.fs(*Recorder) *io.Fs`, and `Recorder.setFault(*Recorder,index:usize,fault:Fault) void` own an internal accounted context. `Fault=enum{io,not_installed,ambiguous,cancel}`; seed/read files through recorder-only fixture methods `put(path:[]const u8,bytes:[]const u8) AtlasError!void`/`get(path:[]const u8) AtlasError![]const u8`; test harness helpers may not implement production behavior.

```zig
test "atlas snapshot digest rejects behavior neutral source drift" {
    var rec = try recorder.init(t.allocator); // test adapter wraps/account-checks it
    defer rec.deinit();
    try rec.seedApprovedClosure(); // literal records in fixture, no host checkout
    const before = try rec.snapshotDigest(); // calls attest, owns temporary context
    try rec.put("zig/tools/ui_atlas/profile.zig", "// independently changed\n");
    const after = try rec.snapshotDigest();
    try t.expect(!std.mem.eql(u8, &before, &after));
}
```

The three convenience recorder methods used above (`seedApprovedClosure() AtlasError!void`, `snapshotDigest() AtlasError!Hash`, `put([]const u8,[]const u8) AtlasError!void`) are implemented in this task; seed is literal fixture data, not production-directory copying. Capture RED with exact filter. Add missing/extra/dynamic import, parent escape, symlink, stat/read race, duplicate/case-colliding record and maximum record/byte cap tests. Closure manifest max=64KiB, ≤256 files, ≤1MiB each and ≤16MiB total; receipts max=64KiB. Include these reviewed parser caps in profile identity.

- [ ] Close exact source list over CLI/modules/adapter/template/profile, `source-closure.json`, `zig/build.zig`, `zig/.zigversion` and every import/embed input. Exclude only noncompiled tests/fixtures, generated identity/executable/receipts. Reject dynamic/unlisted/escaping imports. Approved compiler distribution is independently hashed per host; semantic identity includes version only. Inventory uses the same reviewed all-host records everywhere. Hash domain `NYON-ATLAS-BUILD-V1\0`, LEu64 length-prefixed version and sorted path/byte-length/hash. Debug/ReleaseSafe share semantic digest; compiler binary/options/target belong only in receipt.
- [ ] Driver flags: `--root PATH --compiler PATH --optimize Debug|ReleaseSafe --out PATH --cache PATH`. Copy owned immutable snapshot; generate fixed identity module; compile only snapshot with explicit modules; independently hash approved compiler/output executable; rehash closure before/after compile; drift discards build. Emit `nyon-ui-atlas.build.json` binding invocation/options/target/distribution/snapshot/executable, no candidate ID. Authenticate actual running image, never argv[0]; renamed/replaced path races reject unless image identity remains proven. Receipt trust comes from independent approved build provenance, not self-claims/matching supplied strings. Test invocation recorder now; only Task9 real generator can be admitted.
- [ ] Implement POSIX same-filesystem no-replace/replace, nonfollowing handle IO, owned locks and directory sync. Qualify actual Linux fsync/macOS stronger flush before enabling durability. Windows initially read/check only, both capability flags false; no FlushFileBuffers-only success. Unknown/network filesystems refuse durability. Test stat/open/rename substitution and handle lifetime; model tests do not prove power-loss behavior. Both modes/reviews.

### Task 8: Candidate/selection publication state machines and fault truth tables

**Files:** Create `zig/tools/ui_atlas/{publish.zig,publish_tests.zig,release.zig,release_tests.zig}`; extend recorder/test root.

**Interfaces:** `publish.Cancellation` exposes `requested(*Cancellation) bool` with caller-owned state; never allocator/IO callbacks after deinit. `publish.Freshness` holds `session:*attest.Session,ctx:*Context` and `check(*Freshness,*io.Fs) AtlasError!void`. `publish.Request={root:io.Handle,outputs:*const Outputs,freshness:*Freshness,cancel:*Cancellation}`. Outcomes carry `{id:Hash,phase:Phase,cause:?AtlasError,created:bool,cancellation_deferred:bool}` as appropriate; `Phase=enum{validate,lock,ancestors,files,staging_sync,artifact_rename,artifact_parents,artifact_final,existing_resync,selection_cas,selection_temp,selection_replace,selection_parent,cleanup}`.

`CandidatePublicationOutcome=union(enum){CandidateNotPublished:Diagnostic,CandidateDurable:Publication,CandidateDurabilityIndeterminate:Diagnostic}`; `SelectionCommitOutcome=union(enum){NotCommitted:Diagnostic,CommittedDurable:Publication,CommitIndeterminate:Diagnostic}`; `Diagnostic={id:Hash,phase:Phase,cause:?AtlasError,previous:?Hash,next:?Hash,created:bool}`; `Publication` includes id/path/created/deferred flag and separate optional cleanup diagnostic. `publishCandidate(ctx:*Context,fs:*io.Fs,request:Request) CandidatePublicationOutcome`; `promoteSelection(ctx:*Context,fs:*io.Fs,request:Request,receipt:*const release.ValidatedReceipt,expected_previous:?Hash) SelectionCommitOutcome`. `release.validate(ctx:*Context,manifest:[]const u8,receipt:[]const u8,witnesses:[]const []const u8) AtlasError!*ValidatedReceipt` validates exact witness closure/profile/host rows and hashes; `ValidatedReceipt` is opaque; `deinit(self:*ValidatedReceipt,ctx:*Context) void` releases it. Expose read-only `bytes(*const ValidatedReceipt) []const u8` and `candidateId(*const ValidatedReceipt) Hash` accessors, not public construction.

- [ ] Extend recorder with `candidateScenario(fault:?FaultAt,existing:bool) AtlasError!CandidatePublicationOutcome` and `promotionScenario(fault:?FaultAt) AtlasError!SelectionCommitOutcome`; `FaultAt={phase:Phase,edge:enum{before,after},fault:Fault}`. Scenarios use valid literal outputs/receipts and actual protocol functions. `assertNoSelectionOrReceiptAccess() !void` fails even on **reads**. Baseline selection bytes and prior artifact set are stored before each test.

```zig
test "atlas candidate failure cannot imply selection commit" {
    var rec = try recorder.init(t.allocator);
    defer rec.deinit();
    const result = try rec.candidateScenario(.{ .phase=.artifact_parents,
        .edge=.before, .fault=.io }, false);
    try t.expect(result == .CandidateDurabilityIndeterminate);
    try rec.assertNoSelectionOrReceiptAccess();
}
```

- [ ] Capture RED. Implement spec candidate steps 1–3/selection steps 1–5: exclusive lock, prevalidation, source recheck under lock immediately before publication; ancestor create→sync child+parent top-down; files write/sync/reread/hash→staging sync→no-replace rename→both parents sync→final-dir sync. Existing candidate verifies/re-syncs files+final-dir+parent. Promotion stages separate release+receipt, CAS previous selection, sync temp file+temp-name parent before replace, then final parent sync. No rollback after visible replace; stale locks need explicit ownership recovery.
- [ ] Freeze and execute every truth-table row plus fault injection **before and after every operation**, including ancestor/file/staging/rename/parent/final/existing-directory barriers:

| Operation/events | Required result | Exit later mapped by Task 9 |
| --- | --- | --- |
| generate fresh, barriers complete | CandidateDurable(created) | 0 |
| generate exact existing, all re-sync complete | CandidateDurable(existing) | 0 |
| generate pre-rename validation/IO/unsupported | CandidateNotPublished | 1/2 |
| generate pre-rename or pre-re-sync cancel | CandidateNotPublished | 7 |
| generate rename installed then parent/final sync failure | CandidateDurabilityIndeterminate | 6 |
| generate rename ambiguity or existing re-sync failure | CandidateDurabilityIndeterminate | 6 |
| generate cancel after rename/re-sync starts, barriers complete | CandidateDurable, deferred flag | 0 |
| check-candidate exact bytes | Verified, no lock/write/sync claim | 0 |
| promote artifact/receipt directory sync failure | NotCommitted, prior selection identical | 2 |
| promote selection replace installed then parent sync failure/ambiguity | CommitIndeterminate, retain both IDs | 5 |
| promote all barriers complete | CommittedDurable | 0 |

- [ ] Assert exact operation order/bytes/owned cleanup/forbidden accesses. Test competing writers, conflicting final dir, invalid/missing witness, definitely-not-installed replace, cancellation before/during/after boundaries and cleanup failure preserving durable outcome. Expose `recoverSelection(ctx:*Context,fs:*io.Fs,root:io.Handle,cancel:*Cancellation) SelectionCommitOutcome`: lock, validate observed closure, re-sync, report observed identity without selecting/reverting. Replay durable-state crashes in model; actual power-loss stays unverified. Both modes/reviews.

### Task 9: Authenticated CLI and read-only full regeneration checks

**Files:** Create `zig/tools/build_ui_atlas.zig`, `zig/tools/ui_atlas/{cli.zig,cli_tests.zig,verify.zig,cli_harness.zig}`; update closure and test root.

**Interfaces:** `cli.Operation=enum{generate,check_candidate,promote,check,recover_selection}`; `verify.VerificationOutcome=union(enum){Verified:Hash,Rejected:publish.Diagnostic}`; `verify.candidate(ctx:*Context,fs:*io.Fs,session:*attest.Session,directory:[]const u8) VerificationOutcome`; `verify.selected(ctx:*Context,fs:*io.Fs,session:*attest.Session,selection_path:[]const u8) VerificationOutcome`; `cli.run(ctx:*Context,fs:*io.Fs,args:[]const []const u8,embedded:BuildIdentity) AtlasError!cli.Result`; `Result={exit_code:u8,diagnostic_json:[]u8}` has `deinit(self:*Result,ctx:*Context) void`. Allocation failure escapes as AtlasError; main prints a fixed allocation-free diagnostic and exit2. `cli.exitFor(operation:Operation,outcome:cli.Outcome) u8` consumes tagged union `{candidate:CandidatePublicationOutcome,selection:SelectionCommitOutcome,verification:VerificationOutcome}` and rejects invalid operation/type combinations. `main(init:std.process.Init) !void` only adapts args/std.Io/result and the fixed allocation-failure diagnostic; inspect existing `check-planet-cli.zig` for pinned process conventions.

- [ ] Add parser/mapping tests with exact field values, duplicate flags, missing receipt/paths and mismatched outcome type. Use Task 8 literal outcome constructors:

```zig
test "atlas CLI keeps candidate uncertainty exit distinct" {
    const d = publish.Diagnostic{ .id=@splat(0), .phase=.artifact_parents,
        .cause=error.Io, .previous=null, .next=null, .created=true };
    try t.expectEqual(@as(u8, 6), cli.exitFor(.generate,
        .{ .candidate=.{ .CandidateDurabilityIndeterminate=d } }));
    try t.expectEqual(@as(u8, 2), cli.exitFor(.promote,
        .{ .selection=.{ .NotCommitted=d } }));
}
```

- [ ] Capture RED with filter above. CLI accepts `generate --build-receipt R --root P --out O`; `check-candidate --build-receipt R --root P --candidate MANIFEST`; `check --build-receipt R --root P --selection S`; `promote --build-receipt R --root P --candidate MANIFEST --release-receipt R2 --witness-root W --expected-previous HASH|absent`; `recover-selection --build-receipt R --root P --selection S`. Explicit root is required. Ordinary check resolves fixed release path from strict selection, validates receipt closure, regenerates all four buffers and compares exact bytes. Snapshot selected pointer/manifest once and compare pointer/source/input identity again before returning. Candidate check never reads selection/receipt, locks or writes. Recover cannot synthesize/promote a new identity. Diagnostic JSON always has operation, corresponding typed outcome, stage, bounded relative error context and identities; timing/peak allocation/host data are evidence-only, never deterministic outputs.
- [ ] Authenticate every command, bind inputs then revalidate before success/publication. Checks forbid lock/write, generation forbids selection/receipt accesses. Exact design exits0..7; stale/drift4, contention3. Inject selection/source change during check→ConcurrentModification; IO remains IO; Verified claims no durability.
- [ ] Finish actual snapshot compilation. In ignored owned test closures build A/edit module/profile/template/pin to B; invoke A for every command with original receipt→StaleExecutable/4 and zero publication writes. Test renamed/replaced executable, forged receipt/digest/compiler, omitted import, snapshot/compile/generate races and neutral rebuild B→new manifest/ID even with same pixels. Never race canonical sources.
- [ ] Run proposed owned build and CLI harness (after it creates exact compiler inventory from independently verified distribution evidence):

```sh
/Users/donaldfilimon/.zvm/bin/zig run zig/tools/ui_atlas/build_driver.zig -- --root . --compiler /Users/donaldfilimon/.zvm/bin/zig --optimize ReleaseSafe --out zig/zig-out/atlas-isolated --cache zig/zig-out/atlas-cache
/Users/donaldfilimon/.zvm/bin/zig run zig/tools/ui_atlas/cli_harness.zig -- --root . --tool zig/zig-out/atlas-isolated/nyon-ui-atlas --build-receipt zig/zig-out/atlas-isolated/nyon-ui-atlas.build.json
```

Harness `main(std.process.Init)` calls owned CLI, checks exits/JSON/bounded directory manifests and operation logs; resolves returned candidate hash, never guesses it. Qualified native primitives permit generate/check-candidate; otherwise assert UnsupportedDurablePublication and use pure/read-only fixture checks. Include CLI fmt; both suites/reviews.

### Task 10: Pure versioned loader and retained-contract bridge tests

**Files:** Create `zig/tools/ui_atlas/{retained_loader.zig,retained_loader_tests.zig,retained_bridge_tests.zig}`; update test root. Existing retained implementation stays untouched.

**Interfaces:** `retained_loader.Expected={asset_id:enum{nyon_ui_atlas_v2},candidate_id:Hash,manifest_hash:Hash,atlas_hash:Hash,metrics_hash:Hash,admission:union(enum){qualification:void,release:*const release.ValidatedReceipt}}`; `VersionedMetrics` has the existing descriptor/generator/205-entry logical fields with version=2 and owned fixed-size data (no input aliases). `loadVersionedMetadata(expected:*const Expected,manifest_bytes:[]const u8,metrics_bytes:[]const u8,atlas_bytes:[]const u8,scratch:[]u8) AtlasError!VersionedMetrics`; `replaceVersionedMetadata(resident:*VersionedMetrics,expected:*const Expected,manifest_bytes,metrics_bytes,atlas_bytes,scratch) AtlasError!void`. All three payload byte arguments are `[]const u8`, scratch is `[]u8`. Separate production release entrypoint `loadSelectedMetadata` has the same signature except expected admission must be release and is validated; qualification entrypoint cannot be routed there implicitly. Production has no checksum bypass.

- [ ] Define literal test `LoaderCase` in `retained_loader_tests.zig` with `init(allocator)`, `deinit`, `expected`, `manifest`, `metrics`, `atlas`, `scratch`, and `resident`, using valid canonical fixture records and independent hashes. No V1 frozen hash is changed.

```zig
test "atlas versioned replacement preserves resident on pixel corruption" {
    var c = try LoaderCase.init(t.allocator);
    defer c.deinit();
    const before = c.resident;
    c.atlas[1] ^= 1;
    try t.expectError(error.InputHashMismatch, loader.replaceVersionedMetadata(
        &c.resident, &c.expected, c.manifest, c.metrics, c.atlas, c.scratch));
    try t.expectEqualDeep(before, c.resident);
}
```

- [ ] Capture RED. Hash payloads before strict schema/admission and atomic swap; require expected identity, lengths/paths/version, ≤64KiB JSON/1MiB scratch. Reject duplicate/unknown fields, nonfinite/negative advance/geometry, cell/order/count errors, alias/short scratch, unknown version, candidate-only release admission and checksum bypass. Mutate every payload/hash at constant size; resident bytes stay unchanged.
- [ ] Test-root imports `src/retained_atlas.zig` and `src/retained_text.zig`; production tool cannot. Compare V2 fields with frozen V1, exhaustively map 205 identities into existing AtlasMetrics in test code. Exercise `measure`, `replaceBoundaries`, `appendPlacements`: sizes 12/16/20/40, both weights, unsupported UTF-8/scalars, zero capacities, aliases, nonfinite geometry, clip/UV correspondence, atomic emission/caret. Keep V1 loader/digest unchanged. This is contract compatibility, not application integration. Both modes/reviews.

### Task 11: Qualification runner, owner handoff and conditional retirement

**Files:** Create `zig/tools/ui_atlas/{qualify.zig,qualification_tests.zig,integration-handoff.md}`; update test root. Qualification receipts/diffs/captures go in ignored output. No existing integration file is owned by this task.

**Interfaces:** `qualify.Status=enum{passed,failed,unverified}`; `qualify.Row={kind:enum{numeric,parser,metric,candidate,source_binding,durability,generation_host,runtime,package},artifact:Hash,profile:Hash,status:Status,evidence_hash:Hash}`; `qualify.admit(rows:[]const Row,expected:Hash) AtlasError!void` rejects absent/duplicate/mismatched rows and all non-passed required rows. `qualify.main(std.process.Init)` invokes only the owned build/tool/test binaries, records actual exits/counts/hashes, and never manufactures unavailable target receipts. Runtime witness schema and required rows are the platform design's candidate→target→release closure, not a newly invented substitute.

- [ ] Add compiling admission stub returning success, then behavioral RED:

```zig
test "atlas qualification refuses missing target evidence" {
    try t.expectError(error.InvalidReceipt, qualify.admit(&.{}, @splat(0)));
}
```

- [ ] Capture RED: `atlas ttf trailing hmtx and bounded cmap`. Test every header/table truncation, duplicate/unknown/padded-overlap/checksum errors, offset overflow, table/glyph cap±1, loca order/end, odd range offsets, terminal cmap, delta wrap/zero result and nonselected cmap bounds. Production requires 95 ASCII mappings; no format 12 fallback.
- [ ] On actual macOS ARM64/Linux x64/Windows x64 run two empty-cache generations in Debug/ReleaseSafe; compare all four payloads/IDs/advance bits across locale/timezone/traversal/root relocation. Windows pure-memory comparison is allowed without publication claims. Record full/raster/SDF time and peak allocation: macOS≤30s, owned arena≤64MiB. Reviewed exact-output optimization is required on budget failure.
- [ ] Qualify actual local filesystem interruption/flush/replacement separately from recorder replay; record OS/filesystem/API and precise power-loss scope. Unsupported Windows publication remains unsupported; promote on an independently qualified host. Native packages can consume qualified assets regardless of local generator durability.
- [ ] Obtain actual target observers with candidate/shader/host/backend hashes: 723×802 at 100/115/130%;1280×720,1440×900,1920×1080 at 100/130%; native DPI changes/browser DPR1/2; fonts 12/16/20/40 both weights; icons 16/24/40; disabled/high-contrast/partial clips. Metal/DX12/Vulkan/applicable approved GL/WebGPU/WebGL2 each record all-entry legibility/holes/punctuation/whitespace/caps/joins/baseline/advance/caret/clipping, retained screen layout/input/accessibility. Screenshots alone are not acceptance. Consume platform journeys; do not implement hosts/shaders. Missing required rows block release.
- [ ] `integration-handoff.md` freezes interfaces/file hashes/test receipts/input-manifest bytes/candidate/pending rows and concrete proposed diffs for build, retained runtime, package selection, assets and owned Zig gate. Coordinating owner applies after review; this task cannot edit those paths. Wire `test-atlas`, `atlas-tool` via snapshot driver and `check-atlas-retained`. Root registration changes source identity: rebuild/regenerate/requalify; no reuse of stale receipts.
- [ ] Owner installs approved source manifest, V2 loader/hashed payload descriptor and release-only ordinary selection; qualification requires explicit candidate hash. Preserve V1 witness. After wiring, execute from `zig/`:

```sh
/Users/donaldfilimon/.zvm/bin/zig fmt --check tools/build_ui_atlas.zig tools/ui_atlas src/retained_atlas.zig
/Users/donaldfilimon/.zvm/bin/zig build test-atlas
/Users/donaldfilimon/.zvm/bin/zig build test-atlas -Doptimize=ReleaseSafe
/Users/donaldfilimon/.zvm/bin/zig build atlas-tool -Doptimize=ReleaseSafe
./zig-out/bin/nyon-ui-atlas check --build-receipt zig-out/bin/nyon-ui-atlas.build.json --root .. --selection ../assets/ui/selected-atlas.json
/Users/donaldfilimon/.zvm/bin/zig build check-atlas-retained -Doptimize=ReleaseSafe
```

Before selection, owner runs generate/check-candidate via CLI harness and records candidate-only evidence. Promote exact admitted outputs without rebuilding, then selected check. Existing native/wasm/Rust gates remain owned/applicable until replacement proof removes their paths.
- [ ] In a fresh network-disabled environment without Rust/Python/Node/font tools or caches, trace executable calls and inspect transitive build/link/package inputs (including absolute tools/precompiled Rust); PATH stubs are insufficient. Runtime embeds selected R8/metrics and licenses, no generator/parser service. After regeneration, retained contracts, every required runtime row and delivered-artifact proof, owning integrator uses existing retirement authority to remove `tools/ui-atlas/`, `tools/build-ui-atlas.rs` and required Cargo atlas checks. Preserve source/license/V1 evidence/unported contracts; owners update provenance/docs/gates. Rerun final owned Zig game/atlas gate. Rollback selects previous qualified immutable identity; post-retirement rollback stays Zig-only.
- [ ] Both isolated suites, full owned diff, fresh specification/quality reviews. Report implemented and pending evidence separately. Missing targets retain candidate/pending rows; no retirement/release claim.

## Self-review and execution boundary

Coverage map: design input limits/math→1; sfnt/cmap/hmtx→2; simple/composite/flattening→3; XML/path/arc→4; coverage/SDF/cells→5; canonical hash graph/metrics→6; closed snapshot/source authenticity/owned native IO→7; candidate/selection barriers/outcomes/receipts→8; CLI exits/freshness/read-only checks→9; versioned retained admission→10; cross-host/runtime/final graph/retirement→11. All five Review Focus conditions have owning tests.

Before execution, independently review this complete plan against the pinned design and resolve interface/cap/ownership findings in this file only. During execution, each task requires behavioral RED, fresh Debug/ReleaseSafe GREEN and separate specification/quality review before its dependents proceed. This plan is a planning artifact; document review and hash checks do not run the product gate or qualify runtime behavior.
