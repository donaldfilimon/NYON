# NYON Zig Shader Tooling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce source-authoritative shader candidates and independent receipts for every required target.

**Architecture:** An owned Zig frontend validates seven WGSL sources and lowers them into verified typed IR. Independent numeric/layout witnesses and target emitters produce qualification candidates; target-local probes precede release promotion and need no complete game host.

**Tech Stack:** Pinned Zig/std; authorized system Metal/DXC compilers and real Vulkan/GL/WebGPU/WebGL2 implementations.

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

1. Unsupported syntax must fail before output; S01/S02 `unknown_token_preserves_output`, `unsupported_directive` and S10 `no_candidate_on_error` own this.
2. One wrong packed lane can look plausible; S03c/S11 `sentinel_each_lane`, `corrupt_binding_detected` use independent bytes.
3. Descending smoothstep, eager select and derivatives may translate incorrectly; S03b/S04/S11 `descending_edges`, `select_eager`, `derivative_before_divergence` own this.
4. Missing hardware must not erase rendering families or fabricate compute proof; S00/S10 `twenty_entries_six_targets`, `compute_four_targets`, `missing_target_pending` own this.
5. Stale source/tool/options may reuse old witnesses; S10/S12 `identity_bit_change`, `candidate_never_release`, `missing_tool_no_download` own this.

---

## File structure, source fence and interfaces

Own future `zig/tools/shaders/{profile,lex,ast,parse,types,sema,ir,verify,layout,lower,manifest,witnesses,driver,tests}.zig`, `zig/tools/shaders/emit/{msl,hlsl,spirv,glsl}.zig`, `zig/tools/shaders/probe/{metal,dx12,vulkan,gl,web}.zig`, `zig/tools/build_shaders.zig` and `zig/tests/fixtures/shaders/`. Main [H01](2026-10-09-nyon-zig-platform-bindings.md#task-h01-shared-artifact-identities-and-test-harness) owns the shared `artifacts` module/build registration. Coordinate edits to `zig/build.zig`; these plans do not duplicate schemas or reassign an active writer.

H06's target authorization/ABI receipt precedes real probes, not a complete renderer. Minimal probes use owned pinned textures/buffers; atlas release is not a prerequisite. Main H10/H18/H20/H21 qualification consumes the exact `QualifiedShader` binding to immutable TargetArtifact and its compile/interface Witness below. Ordinary release consumers use H01 `selectReleaseShader` with the explicit artifact ID/expected target; local payload indices are scoped to that selected artifact, never to the all-target manifest.

| Defined by | Public type contract |
| --- | --- |
| H01 main | `Allocator=std.mem.Allocator`, `Digest=[32]u8`, `Target`, `Candidate`, `OwnedCandidate`, `Approval`, `Witness`, `OwnedWitness`, `Requirement`, `TargetAuthorization`, `ArtifactError`; import named module `artifacts` |
| S00 profile | `ProfileId=enum{nyon_wgsl_1}`; `SourceId=enum{primitives,space,worlds,routes,ui_sdf,postprocess,advisory}`; `Stage=artifacts.ShaderStage`; `Source{id:SourceId,bytes:[]const u8,hash:Digest}`; `Corpus{sources:[7]Source}`; `Entry{source:SourceId,name:[]const u8,stage:Stage}` |
| S01 diagnostics | `Span{start:u32,end:u32}`; `Token{tag:TokenTag,span:Span}`; exhaustive TokenTag from S00 grammar. `Diagnostic{code:DiagnosticCode,source:SourceId,span:Span}`; `DiagnosticSink{items:[]Diagnostic,len:usize}` caller-owned; full sink returns Limit. `Lexed{tokens:[]Token,allocator:Allocator}` owns tokens with `deinit(*Lexed) void` |
| S02 parser | Opaque internal `Ast`; `Parsed{ast:*Ast,allocator:Allocator}` owns AST arena, `deinit(*Parsed) void`; u32 node IDs and source spans, no raw target-code nodes |
| S03a/b IR | Opaque internal `Module`; `Checked{module:*Module,allocator:Allocator}` owns arena, `deinit(*Checked) void`; `Verified{module:*const Module,ir_version:u32,hash:Digest}` borrows Checked. Typed operations never contain foreign pointers |
| S03c layout | Owned `LayoutSet` with `deinit(*LayoutSet) void`; `FieldLayout{offset:u32,size:u32,alignment:u32,stride:u32,scalar:enum{f32,u32}}`; separate vertex/uniform/storage categories |
| S04/S04b lowering | Owned opaque `Lowered`, `deinit(*Lowered) void`; independent S04b returns opaque `*VerifiedTarget`. Lowered and LayoutSet remain immutable and alive until every token/emitter borrow ends; only the verifier can create the token |
| S05–S09 emission | `Emitted{bytes:[]u8,reflection:[]u8,allocator:Allocator}`, `deinit(*Emitted) void` |
| H01 compiled artifact | Import `TargetArtifact`, `OwnedTargetArtifact`, `CompilerIdentity`, `ShaderEntryBinding`, `CompiledTarget`, `QualifiedShader`, `ShaderSelectionRequest`, `ReleaseShaderBinding`, `LoadedPayload` and `ShaderBinding` from main H01. CompiledTarget owns immutable target payload metadata and compile/interface witness; downstream runtime Witness values bind TargetArtifact.id separately |

`ShaderError = Allocator.Error || error{InvalidSource,UnsupportedSyntax,Limit,TypeMismatch,InvalidStage,InvalidBinding,NonuniformDerivative,UnprovedBounds,InvalidIr,InvalidTargetIr,StaleVerifiedTarget,TargetUnsupported,IoFailure}`. DiagnosticCode mirrors those semantic cases plus unavailable_target. Returned owned values each have one deinit owner; Verified and source slices cannot outlive their owner. All error paths release arenas; stable IR IDs never wrap.

S00 freezes all seven `assets/shaders/*.wgsl` bytes (17,131 bytes at design inspection) and expected-data hashes before frontend dispatch. Exact profile: 64 KiB/source, 256 KiB/corpus, 32,768 tokens/nodes, depth 64, 32 MiB arena. Support every construct in the spec's language table: listed f32/u32/bool/vector/mat 4 types, fixed arrays 3/4/6/7/57/84, struct/global/local/function syntax, zero initialization, typed resource access, bounded structured loops, attributes/flat integers, all used derivative/texture/math builtins. Unsupported features require a reviewed profile upgrade; no source rewrite or regex compiler.

| Source | Render entrypoints required on all six targets |
| --- | --- |
| primitives | vs_main, fs_main |
| space | vs_main, fs_main |
| worlds | vs_main, fs_main |
| routes | route_vs, route_fs, fleet_vs, fleet_fs, halo_vs, halo_fs, particle_vs, particle_fs |
| ui_sdf | panel_vs, panel_fs, vs_main, fs_main |
| post process | vs_main, fs_main |

Twenty rendering entrypoints; target enum is Metal/DX12/Vulkan/native GL/WebGPU/WebGL2. Advisory `score_world`, workgroup 8, is required on Metal/DX12/Vulkan/WebGPU and excluded on both GL baselines. Common parsing still includes advisory. Low retains all families:1 sample/no bloom/quarter decorative particles; High:4× MSAA/bloom/full particles. Independent expected data comes from reviewed declarations/formulas, never the new parser/emitter/reflection.

### Task S00: Freeze profile and eligibility

**Files:** Create `zig/tools/shaders/profile.zig`, `zig/tests/fixtures/shaders/profile-v1.json`, `zig/tools/shaders/tests.zig`.

**Interfaces:** Consumes H01 artifact types and approved spec. Produces `inventory(corpus:*const Corpus) ShaderError![21]Entry`; `eligible(entry:Entry,target:Target) bool`; `validateLimits(bytes:usize,nodes:u32,depth:u16,arena_bytes:usize) ShaderError!void`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** Freeze handwritten entry list; assert `expectEqual(20,render_count)`, `expectEqual(6,target_count)`, `expectEqual(4,compute_target_count)`, `expect(!eligible(score_world,.webgl2))`. Limits pass exactly at maxima and fail +1; source mutation invalidates approval.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S00 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S00`. Expected: nonzero with test `S00` failing wrong count or exclusion; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Implement explicit profile/eligibility tables and source fences; review grammar productions/positive-negative vectors before S01.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S00`, then `zig build test-shaders -Dcase=S00 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; all 21 entries and target exclusions appear in the receipt.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task S01: Bounded tokenizer and diagnostics

**Files:** Create `zig/tools/shaders/lex.zig`; add lexer tests.

**Interfaces:** Consumes S00 Source/profile/limits. Produces `lex(a:Allocator,source:Source,diagnostics:*DiagnosticSink) ShaderError!Lexed`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** Assert exact spans for decimal/fraction/u/hex-u literals, //comments, identifiers, shifts and nested-type punctuation. Assert `expectError(error.UnsupportedSyntax,lex(...))` for unknown token/directive; malformed literal, token 32769 and arena exhaustion reject without output. Exhaust allocation sites and deinit.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S01 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S01`. Expected: nonzero with test `S01` failing unknown tokens accepted or spans wrong; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Cursor tokenizer with checked sizes; retain spelling for contextual numeric materialization.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S01`, then `zig build test-shaders -Dcase=S01 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; span/limit/allocation failures are typed and leak-free.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task S02: Parse every shipped grammar construct

**Files:** Create `zig/tools/shaders/{ast,parse}.zig`; parser fixtures/tests.

**Interfaces:** Consumes S01 Lexed and S00 grammar/source fence. Produces `parse(a:Allocator,source:Source,tokens:[]const Token,diagnostics:*DiagnosticSink) ShaderError!Parsed`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** All seven sources parse; independently specified AST kind/span/child expectations cover optional struct semicolons, attributes, constants, nested constructors, typed zero-init and for/if/discard. Missing delimiters, unsupported directive and malformed constructor syntax return exact diagnostics; never skip tokens.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S02 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S02`. Expected: nonzero with test `S02` failing shipped production or rejection case has wrong AST/outcome; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Explicit recursive grammar and precedence parser bounded by depth 64/node 32768; no semantic lowering yet.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S02`, then `zig build test-shaders -Dcase=S02 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; full source coverage and malformed/limit cases execute.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task S03a: Resolve symbols and typed expressions

**Files:** Create `zig/tools/shaders/{types,sema,ir}.zig`; semantic-expression cases.

**Interfaces:** Consumes S02 Parsed and S00 overload rules. Produces `check(a:Allocator,parsed:*const Parsed,profile:ProfileId,diagnostics:*DiagnosticSink) ShaderError!Checked`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** Assert exact f32 literal bits/zero initialization, u32 shifts/remainder, matrix columns and overloads. Unknown names, wrong constructor/member/assignment, invalid writable swizzle and storage-read write return typed errors. `expectError(error.TypeMismatch,check(...))` must be behavioral.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S03a -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S03a`. Expected: nonzero with test `S03a` failing wrong type/constant bits or illegal write accepted; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Typed symbol/operation IDs and span provenance; preserve source order and eager argument evaluation.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S03a`, then `zig build test-shaders -Dcase=S03a -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; all corpus modules resolve without target strings or source-specific substitutes.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task S03b: Independent IR control-flow and access verifier

**Files:** Create `zig/tools/shaders/verify.zig`; independently mutated IR tests.

**Interfaces:** Consumes S03a Module and S00 stage/bounds contracts. Produces `verify(module:*const Module,diagnostics:*DiagnosticSink) ShaderError!Verified`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** Reject mutated use/type, missing return, illegal stage builtin, duplicate binding/location, non-flat u32, nonuniform derivative and unproved index. Prove advisory invocation 7 guard precedes all access, loops 4×12+4, and draw-array bounds 3/6. Assert every mutation fails and full corpus succeeds.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S03b -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S03b`. Expected: nonzero with test `S03b` failing one malicious IR mutation returns Verified; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Separate verification traversal; explicit stage/uniformity/access/bounds rules; no optimization or derivative hoisting.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S03b`, then `zig build test-shaders -Dcase=S03b -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; verifier mutation tests detect each corruption independently.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task S03c: Independent buffer and vertex layouts

**Files:** Create `zig/tools/shaders/layout.zig`, `zig/tests/fixtures/shaders/layout-v1.json`; layout tests.

**Interfaces:** Consumes S03b Verified and handwritten host declarations. Produces `reflect(a:Allocator,module:Verified,target:Target) ShaderError!LayoutSet`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** Assert stride 36 offsets 0/8/24/32; globals 16; SceneGlobals 112 offsets 0/64/80/96; UiGlobals 16 offsets 0/8/12; arrays 84/57/7 stride 4 bytes 336/228/28. Every binding/rate/signedness/column and other instance stride has a reviewed expectation. Corrupt one lane/stride/binding and assert independent reader detects it.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S03c -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S03c`. Expected: nonzero with test `S03c` failing wrong packed descriptor accepted; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Separate packed vertex from uniform/storage layout; device allocation padding cannot change logical length.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S03c`, then `zig build test-shaders -Dcase=S03c -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; all descriptors match independent byte vectors.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task S04: Portable numeric and coordinate lowering

**Files:** Create `zig/tools/shaders/lower.zig`; numeric/lowering cases.

**Interfaces:** Consumes S03b Verified, S03c LayoutSet and target profile. Produces `lower(a:Allocator,module:Verified,layout:*const LayoutSet,target:Target) ShaderError!Lowered`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** Assert ascending/descending smoothstep polynomial/end points, eager select, derivatives before divergent shape flow, flat ring bits/end points, texture/sampler pairing and dimensions, matrix column order. Double clip-Y/depth/front-face conversion deliberately fails; unproved target feature returns TargetUnsupported.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S04 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S04`. Expected: nonzero with test `S04` failing numeric/order/coordinate witness differs; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Explicit typed lowering with exact literal spellings/bit construction; no fast-math/FMA/reassociation/unreviewed unrolling.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S04`, then `zig build test-shaders -Dcase=S04 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; numeric equality/tolerances remain distinct from exact host packing.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task S04b: Independent post-lowering target IR verification

**Files:** Create `zig/tools/shaders/verify_target.zig`; independent target-IR mutation tests in the shader suite. Keep S03b source verifier unchanged.

**Interfaces:** Consumes S04 Lowered, S03c LayoutSet and frozen S00 target/stage/capability/coordinate profile. Lowerer declarations may be shared, but its validity flags or helper traversal are not trusted. Produces opaque `VerifiedTarget`; `verifyTarget(a:Allocator,ir:*const Lowered,layout:*const LayoutSet,target:Target,diagnostics:*DiagnosticSink) ShaderError!*VerifiedTarget`; `destroyVerifiedTarget(a:Allocator,token:*VerifiedTarget) void`; `targetView(token:*const VerifiedTarget) ShaderError!TargetView` where `TargetView{target:Target,profile_hash:Digest,layout_hash:Digest,ir:*const Lowered}` is a read-only target IR view accessible only after the verifier seal/hash check. Add `InvalidTargetIr` and `StaleVerifiedTarget` to ShaderError.

- [ ] **Step 1 — write tests and minimal compiling scaffolds.** `S04b independently_mutated_target_ir` constructs corrupt target IR without invoking lower(): wrong use/type, illegal target stage/resource, missing return/merge, wrong binding/stride/layout, unproved array index, duplicate or missing coordinate conversion, incorrect target capability. Each `expectError(error.InvalidTargetIr,verifyTarget(...))` occurs before emitter invocation/output. `sealed_view` test-only mutation after successful verification makes targetView return StaleVerifiedTarget and leaves emitter/output counts zero. All seven sources with every eligible target pass; native GL/WebGL2 compute remains excluded, never a missing render entry.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S04b -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S04b`. Expected: nonzero with `S04b` failing one independently malformed lowered module obtains a usable VerifiedTarget token; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Independently traverse lowered control flow, definitions/uses, resource layouts/access bounds, stage legality and coordinate-conversion metadata against the target profile and independent layout expectations. On success allocate an opaque sealed token containing target/profile/layout/IR hashes and immutable borrowed views. Its constructor is private to verify_target; no cast/unchecked token helper is exported. Lowered and LayoutSet are immutable and must outlive the token and every emitter call; caller destroys token before deinitializing either owner. Emitters call targetView before writing any output; a test-only corruption hook is excluded from production. Never replace S03b with this second verification.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S04b`, then `zig build test-shaders -Dcase=S04b -Doptimize=ReleaseSafe`. Expected: both exit 0, all named cases executed, no leaks. every independent target mutation is rejected before output; allocation failure frees the token once. S05–S08 accept only *const VerifiedTarget; S09 packages original source using S03b Verified because it performs no target lowering.
- [ ] **Step 6 — review checkpoint.** Freeze the scoped diff, source/tool/fixture identities, exact command/exit/count and independent review before dependent dispatch. No commit, acquisition or publication is part of this task.

### Task S05: Metal output and authorized target probes

**Files:** Create `zig/tools/shaders/emit/msl.zig`, `zig/tools/shaders/probe/metal.zig`; target tests.

**Interfaces:** Consumes S04b opaque VerifiedTarget and H06 Metal TargetAuthorization. Produces `emit(a:Allocator,token:*const VerifiedTarget) ShaderError!Emitted`; probe `compile(a:Allocator,io:std.Io,input:*const Candidate,auth:*const TargetAuthorization,out_root:[]const u8) artifacts.ArtifactError!artifacts.CompiledTarget`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** Freeze MSL2.4; all 20 render+score_world. Assert exact entry/type/resource declarations and reflected bytes. A test-only process/driver seam injects compile failure, missing SDK and wrong output hash; assert failure/unexecuted rather than pass, no download or release publication. `S05 immutable_compiled_result` injects missing/truncated/wrong-compiler output, wrong entry mapping and changed flags; no TargetArtifact is published on failure. On success assert exact parent ID, payload lengths/hashes and entry mappings; destroying CompiledTarget frees its owned arenas once while persisted immutable output remains.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S05 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S05`. Expected: nonzero with test `S05` failing a target declaration, corrupt artifact or failed compile is accepted; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Implement typed stage attributes and buffer/texture/sampler slots; installed SDK AIR/metallib. SDK argv/options and executable hashes come from authorized H06 evidence, never shell interpolation or automatic acquisition. Qualification candidates precede probes. compile returns H01 CompiledTarget, not a receipt without bytes. It writes actual metallib payloads into a new immutable TargetArtifact, binds compiler/engine version/flags/profile and all eligible entry mappings, and emits the matching compile/interface Witness. It never edits the parent Candidate or attaches mutable runtime status to artifact identity.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S05`, then `zig build test-shaders -Dcase=S05 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; unit GREEN is not target proof. After authorization run `zig build shader-tool -- probe --target metal --authorization ../docs/qualification/platform/metal-auth.json --candidate zig-out/qualification/shaders/metal.json`; expected exit 0 only with real compiler/interface receipt.  Unavailable returns 2 and remains pending.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task S06: DX12 output and authorized target probes

**Files:** Create `zig/tools/shaders/emit/hlsl.zig`, `zig/tools/shaders/probe/dx12.zig`; target tests.

**Interfaces:** Consumes S04b opaque VerifiedTarget and H06 DX12 TargetAuthorization. Produces `emit(a:Allocator,token:*const VerifiedTarget) ShaderError!Emitted`; probe `compile(a:Allocator,io:std.Io,input:*const Candidate,auth:*const TargetAuthorization,out_root:[]const u8) artifacts.ArtifactError!artifacts.CompiledTarget`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** Freeze HLSL shader-model 6.0; all 20 render+score_world. Assert exact entry/type/resource declarations and reflected bytes. A test-only process/driver seam injects compile failure, missing SDK and wrong output hash; assert failure/unexecuted rather than pass, no download or release publication. `S06 immutable_compiled_result` injects missing/truncated/wrong-compiler output, wrong entry mapping and changed flags; no TargetArtifact is published on failure. On success assert exact parent ID, payload lengths/hashes and entry mappings; destroying CompiledTarget frees its owned arenas once while persisted immutable output remains.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S06 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S06`. Expected: nonzero with test `S06` failing a target declaration, corrupt artifact or failed compile is accepted; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Implement explicit semantics/register/root-signature crosswalk and buffer packing; authorized SDK DXC DXIL. SDK argv/options and executable hashes come from authorized H06 evidence, never shell interpolation or automatic acquisition. Qualification candidates precede probes. compile returns H01 CompiledTarget, not a receipt without bytes. It writes actual DXIL payloads into a new immutable TargetArtifact, binds compiler/engine version/flags/profile and all eligible entry mappings, and emits the matching compile/interface Witness. It never edits the parent Candidate or attaches mutable runtime status to artifact identity.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S06`, then `zig build test-shaders -Dcase=S06 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; unit GREEN is not target proof. After authorization run `zig build shader-tool -- probe --target dx12 --authorization ../docs/qualification/platform/dx12-auth.json --candidate zig-out/qualification/shaders/dx12.json`; expected exit 0 only with real compiler/interface receipt.  Unavailable returns 2 and remains pending.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task S07: Vulkan output and authorized target probes

**Files:** Create `zig/tools/shaders/emit/spirv.zig`, `zig/tools/shaders/probe/vulkan.zig`; target tests. Also create `zig/tools/shaders/spirv_reader.zig`; it never calls emitter code.

The independent reader owns `Inspection{entry_count:u32,id_bound:u32,layout_hash:Digest}` and `inspect(words:[]const u32) ShaderError!Inspection`; it checks word lengths, bound/use/type/control-flow/decorations without importing emitter helpers.

**Interfaces:** Consumes S04b opaque VerifiedTarget and H06 Vulkan TargetAuthorization. Produces `emit(a:Allocator,token:*const VerifiedTarget) ShaderError!Emitted`; probe `compile(a:Allocator,io:std.Io,input:*const Candidate,auth:*const TargetAuthorization,out_root:[]const u8) artifacts.ArtifactError!artifacts.CompiledTarget`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** Freeze SPIR-V1.3/Vulkan 1.1; all 20 render+score_world. Assert exact entry/type/resource declarations and reflected bytes. A test-only process/driver seam injects compile failure, missing SDK and wrong output hash; assert failure/unexecuted rather than pass, no download or release publication. Independently reject truncated instruction, reused ID, wrong type/decoration/branch and entry stage. `S07 immutable_compiled_result` injects missing/truncated/wrong-compiler output, wrong entry mapping and changed flags; no TargetArtifact is published on failure. On success assert exact parent ID, payload lengths/hashes and entry mappings; destroying CompiledTarget frees its owned arenas once while persisted immutable output remains.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S07 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S07`. Expected: nonzero with test `S07` failing a target declaration, corrupt artifact or failed compile is accepted; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Implement direct binary emission with IDs/types/capabilities/decorations/structured merges; independent binary reader. SDK argv/options and executable hashes come from authorized H06 evidence, never shell interpolation or automatic acquisition. Qualification candidates precede probes. compile returns H01 CompiledTarget, not a receipt without bytes. It writes actual SPIR-V payloads into a new immutable TargetArtifact, binds compiler/engine version/flags/profile and all eligible entry mappings, and emits the matching compile/interface Witness. It never edits the parent Candidate or attaches mutable runtime status to artifact identity.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S07`, then `zig build test-shaders -Dcase=S07 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; unit GREEN is not target proof. After authorization run `zig build shader-tool -- probe --target vulkan --authorization ../docs/qualification/platform/vulkan-auth.json --candidate zig-out/qualification/shaders/vulkan.json`; expected exit 0 only with real compiler/interface receipt.  Unavailable returns 2 and remains pending.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task S08: Desktop GL and WebGL2 output and authorized target probes

**Files:** Create `zig/tools/shaders/emit/glsl.zig`, `zig/tools/shaders/probe/gl.zig` and the WebGL2 portion of `zig/tools/shaders/probe/web.zig`; target tests.

**Interfaces:** Consumes S04b opaque VerifiedTarget and H06 GL and WebGL2 TargetAuthorization. Produces `emit(a:Allocator,token:*const VerifiedTarget) ShaderError!Emitted`; probe `compile(a:Allocator,io:std.Io,input:*const Candidate,auth:*const TargetAuthorization,out_root:[]const u8) artifacts.ArtifactError!artifacts.CompiledTarget`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** Freeze GLSL3.30 core and GLSL ES3.00; all 20 render on each, no compute. Assert exact entry/type/resource declarations and reflected bytes. A test-only process/driver seam injects compile failure, missing SDK and wrong output hash; assert failure/unexecuted rather than pass, no download or release publication. Assert both GL outputs retain every rendering family and contain no compute declaration. `S08 immutable_compiled_result` injects missing/truncated/wrong-compiler output, wrong entry mapping and changed flags; no TargetArtifact is published on failure. On success assert exact parent ID, payload lengths/hashes and entry mappings; destroying CompiledTarget frees its owned arenas once while persisted immutable output remains.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S08 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S08`. Expected: nonzero with test `S08` failing a target declaration, corrupt artifact or failed compile is accepted; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Implement separate target profiles, ES highp float/int, flat uints, samplers/attributes and real compile/link. SDK argv/options and executable hashes come from authorized H06 evidence, never shell interpolation or automatic acquisition. Qualification candidates precede probes. compile returns H01 CompiledTarget, not a receipt without bytes. It writes actual GLSL 3.30 / GLSL ES 3.00 payloads into a new immutable TargetArtifact, binds compiler/engine version/flags/profile and all eligible entry mappings, and emits the matching compile/interface Witness. It never edits the parent Candidate or attaches mutable runtime status to artifact identity.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S08`, then `zig build test-shaders -Dcase=S08 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; unit GREEN is not target proof. After authorization run `zig build shader-tool -- probe --target gl --authorization ../docs/qualification/platform/gl-auth.json --candidate zig-out/qualification/shaders/gl.json`; expected exit 0 only with real compiler/interface receipt. Repeat with `--target webgl2` and its separate auth/candidate; one target does not prove the other. Unavailable returns 2 and remains pending.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task S09: Original WebGPU WGSL output and authorized target probes

**Files:** Create `zig/tools/shaders/wgsl_package.zig`; extend the WebGPU portion of `zig/tools/shaders/probe/web.zig`; target tests.

**Interfaces:** Consumes S03b Verified, original Source and H06 WebGPU TargetAuthorization. Produces `package(a:Allocator,source:Source,module:Verified) ShaderError!Emitted`; probe `compile(a:Allocator,io:std.Io,input:*const Candidate,auth:*const TargetAuthorization,out_root:[]const u8) artifacts.ArtifactError!artifacts.CompiledTarget`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** Freeze exact original WGSL bytes; all 20 render+score_world. Assert exact entry/type/resource declarations and reflected bytes. A test-only process/driver seam injects compile failure, missing SDK and wrong output hash; assert failure/unexecuted rather than pass, no download or release publication. Assert `expectEqualSlices(u8,source.bytes,output.bytes)` and source-hash mismatch rejection. `S09 immutable_compiled_result` injects missing/truncated/wrong-compiler output, wrong entry mapping and changed flags; no TargetArtifact is published on failure. On success assert exact parent ID, payload lengths/hashes and entry mappings; destroying CompiledTarget frees its owned arenas once while persisted immutable output remains.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S09 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S09`. Expected: nonzero with test `S09` failing a target declaration, corrupt artifact or failed compile is accepted; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Implement package original validated source, independently parsed by real browser, without re-emitting WGSL. SDK argv/options and executable hashes come from authorized H06 evidence, never shell interpolation or automatic acquisition. Qualification candidates precede probes. compile returns H01 CompiledTarget, not a receipt without bytes. It writes actual original validated WGSL payloads into a new immutable TargetArtifact, binds compiler/engine version/flags/profile and all eligible entry mappings, and emits the matching compile/interface Witness. It never edits the parent Candidate or attaches mutable runtime status to artifact identity.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S09`, then `zig build test-shaders -Dcase=S09 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; unit GREEN is not target proof. After authorization run `zig build shader-tool -- probe --target webgpu --authorization ../docs/qualification/platform/webgpu-auth.json --candidate zig-out/qualification/shaders/webgpu.json`; expected exit 0 only with real compiler/interface receipt.  Unavailable returns 2 and remains pending.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task S10: Immutable candidates and stale-evidence rejection

**Files:** Create `zig/tools/shaders/manifest.zig`; candidate/cache cases.

**Interfaces:** Consumes S00–S09 for the requested target and shared H01 schema/admission APIs. Produces `candidate(a:Allocator,corpus:*const Corpus,target:Target,emitted:[]const Emitted,approval:artifacts.Approval) artifacts.ArtifactError!artifacts.OwnedCandidate`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** Assert frontend/IR failure publishes no candidate. Admitted structural candidates retain pending downstream states. Flip source/tool/profile/coordinate/layout/output bit: `expect(!eql(old_id,new_id))`; old witness attachment fails. Every manifest enumerates all 20 rendering entries and exact compute states. Candidate kind fails ordinary release load. `S10 parent_immutable` compares parent payload/manifest bytes before and after successful target compilation and witness attachment; both must remain unchanged. The compiled child has a different TargetArtifact ID and is the only target-loadable shader binding.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S10 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S10`. Expected: nonzero with test `S10` failing stale witness or invalid source is admitted; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Use H01 `candidateId`, `validateCandidate` and `writeCandidate` APIs; no second promotion engine and no mutation of current assets/release outputs.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S10`, then `zig build test-shaders -Dcase=S10 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; all identity/rejection cases pass; missing other target remains pending.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task S11: Independent packing image and compute evidence

**Files:** Create `zig/tools/shaders/witnesses.zig`, `zig/tests/fixtures/shaders/{numeric,packing,image-cases,compute-cases}.json`; extend minimal probes.

**Interfaces:** Consumes H01 QualifiedShader and exact TargetArtifact payloads, independent S03c vectors, frozen Task 3F CPU vectors/tolerance and H06 authority. Produces `runWitness(a:Allocator,io:std.Io,shader:artifacts.QualifiedShader,requirement:artifacts.Requirement,auth:*const TargetAuthorization) artifacts.ArtifactError!artifacts.OwnedWitness`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** Deliberately corrupt each packed lane/binding and a known sample: witness fails. Ten render-pair scenes cover all family/metadata/hazard/ring/alpha/depth/sRGB/scale/post process cases. Compute covers 7 outputs, guarded 8 th invocation, all 84 features/57 weights, metadata/stale/nonfinite/map/device faults with exact reviewed CPU tolerance. Expected bytes/samples come from independent formulas/frozen oracle observations, never new compiler reflection. `S11 artifact_binding` changes one loaded compiled byte or artifact ID and rejects old packing/image/compute witnesses. All readbacks/captures record exact target payload hashes; tests never validate a source Candidate while loading unrelated compiled bytes.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S11 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S11`. Expected: nonzero with test `S11` failing known corruption passes the witness; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Independent readback interpretation and per-region image expectations. Freeze justified measured image tolerances under review before acceptance; no global-average loophole. Minimal probes can close sufficient packing/image/compute cases, integrated hosts only supply their UI-dependent cases.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S11`, then `zig build test-shaders -Dcase=S11 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; authorized `zig build shader-tool -- witness --target metal --candidate zig-out/qualification/shaders/metal.json --case packing --authorization ../docs/qualification/platform/metal-auth.json` plus each named target/case records actual results; skipped/unavailable is never parity.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

### Task S12: Offline driver and tooling closure

**Files:** Create `zig/tools/{build_shaders.zig,shaders/driver.zig}`; driver tests; H01 registers `shader-tool`.

**Interfaces:** Consumes S00–S11 stage APIs and H01 process/output/receipt contracts. Produces `run(a:Allocator,io:std.Io,args:[]const []const u8) artifacts.ArtifactError!u8`; subcommands `inventory`, `emit`, `probe`, `witness`, `verify`.

- [ ] **Step 1 — write the named tests and minimal compiling scaffolds.** Missing SDK returns 2 without acquisition/sub process fallback; invalid output path/changed identity rejects; no cargo/rustc/Python/shell invocation; release cannot load candidate. Instrument process seam to assert argv/hash and no hidden tool. Candidate bytes remain immutable across witness attachment.
- [ ] **Step 2 — establish compilation.** Run `zig build test-shaders -Dcase=S12 -Dcompile-only=true`. Expected: exit 0; no tests executed. Fix interface/import errors before RED.
- [ ] **Step 3 — observe behavioral RED.** Run `zig build test-shaders -Dcase=S12`. Expected: nonzero with test `S12` failing unavailable tool triggers acquisition or stale output accepted; a compile failure or skipped case is not evidence.
- [ ] **Step 4 — implement the smallest owned change.** Orchestrate owned Zig modules; return 0 only for the requested satisfied stage, 2 unavailable prerequisite, 1 failed validation. Record stage and exact exit; no atlas/full-host dependency. Persist CompiledTarget's artifact and compile/interface witness atomically in the qualification root, release its owned memory, and reload QualifiedShader with H01 validation for probes/hosts. Runtime witnesses reference the same TargetArtifact ID; promotion and packaging never recompile. S12 exports each exact artifact ID/target for the H25 release membership and H26 explicit ShaderSelectionRequest list; it does not replace IDs with array offsets or an OS default. Candidate probes retain QualifiedShader. After H25 exists, the H01/H26 correspondence tests load the same bytes through the checked release selection; this is a later integration assertion, not a prerequisite for S12 candidate probes.
- [ ] **Step 5 — verify GREEN and safety mode.** Run `zig build test-shaders -Dcase=S12`, then `zig build test-shaders -Dcase=S12 -Doptimize=ReleaseSafe`. Expected: both exit 0, every named case executed, no leaks; `zig build shader-tool -- inventory --sources ../assets/shaders` reports 21; authorized `zig build shader-tool -- emit --target metal --out zig-out/qualification/shaders --approval ../docs/qualification/platform/tool-dispatch.json` creates only qualification candidates.
- [ ] **Step 6 — review checkpoint.** Freeze only this task's diff and exact command/exit/test-count/source/fixture/tool identities in its owned receipt; independently review specification and code quality before dependent dispatch. No commit or publication is part of this task.

## Probe driver staging

S05–S09 and S11 first qualify pure emitter/probe-seam behavior with injected SDK/browser results. Their real `shader-tool` commands listed above are follow-on target checks run by S12 after its driver exists, not prerequisites for implementing S12. S10 can structurally admit a candidate with pending target evidence. S12 dispatches its authorized compile/interface probes; only their passing receipts open the matching full target host. Image/compute witnesses may use the same minimal probe; no atlas or complete-host prerequisite is introduced. Missing tools return 2 and retain pending evidence. A reviewed unit slice alone never closes a real target gate.

## Exit contract and self-review

Dependency chain: S00→S01→S02→S03a→S03b→S03c→S04→S04b; S05/S06/S07/S08 are per-target branches after S04b+applicable H06; S09 uses verified original WGSL. S10 packages one requested target without waiting for missing others; S11 binds its actual witnesses; S12 drives stages. Main H10/H18/H20/H21 consume structural Candidate + immutable TargetArtifact + matching compile/interface Witness. Pending runtime witnesses permit explicitly dispatched qualification, not release; H25/H26 own final all-product closure.

Coverage: shader grammar/IR/numeric/layout→S00–S04b; six outputs→S05–S09; identities/candidates→S10; independent byte/image/compute proof→S11; offline tooling graph→S12. All five Review Focus lines own named rejection tests. Types/owners above are single-source; assertions are behavioral after compile-only success. Bodies describe decisions rather than entire compiler implementations. No proposed compiler/SDK/output/runtime result is claimed. Preserve current migration checks until qualified replacement and record actual future gates. SDD is already selected; user plan review precedes implementation.
