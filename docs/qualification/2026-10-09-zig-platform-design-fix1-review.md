## Design Document Review: NYON Task 6 platform bindings — fix 1

### Summary

**APPROVE the scoped fix round.** Original Important issues I1–I3 and related Minor issues M4–M5 are **ADDRESSED**; no new breakage was found in the eight fix-only hunks. This accepts the corrections to the design, which remains **Proposed**; it supplies no user architectural, foreign-language, dependency or acquisition authorization and closes no runtime or product acceptance row.

Reviewed on 2026-10-09 against revised design SHA-256 `0a42aec686430da1a7e35342f5cd2f1dfb60199a5f0465b705d9140ce077b7da`. Design references below are to `task-6-binding-design.md` in this directory; other paths are relative to the NYON repository. Scope is the original findings and breakage introduced by their fixes, not a fresh whole-design review.

### Issue I1: Independent scheduler bounds and durable retirement

- **Original severity**: major / Important — blocking.
- **Section**: Threads, scheduling, and bounded retirement; slice 6B; LG-07.
- **Status**: **ADDRESSED**.
- **Verification**: Design lines 119–122 independently specify `remaining_visits`, `pending_boundaries` ≤20 and `publications_this_update` ≤20. The counter starts only at outer update entry; pending requests include the request owned by the prospective boundary. Physical host tickets are separately counted and explicitly Proposed, so host saturation cannot consume Living request slots. The twenty-one immediately-ready/recycled-ticket witness directly covers the original defect.
- **Source and effect**: Design lines 132–140 require every child to use “the current remaining allowance,” charge actual failed visits exactly once, and share cleanup/history/restore/compare/boundary/private-presentation work. Invalid allowances do zero work, terminal polls do zero visits, variable encoding stays within 256 bytes per visit, and callbacks cannot reset counters or publish. Runtime-owned retirement survives hidden/no-session updates, retains rejected owners, and drains before prospective admission. These match `docs/superpowers/plans/2026-10-02-living-resident-runtime.md:15,19–23,33–47` and `docs/superpowers/plans/2026-10-02-complete-bounded-living-resident-boundary.md:8–24`. Integer pace, debt clearing/dropping, exactly-one Step and accepted-command retention remain explicit. Slice 6B at design line 347 and LG-07 at line 380 carry these contracts into integration evidence.
- **Evidence boundary**: Current `src/sandbox_session.rs:3537–3554` still calls `finish_synchronously` and drops retired output; lines 3619–3650 retain the transitional capped-request path. The revised design correctly calls this transitional evidence, not proof of the proposed scheduler. Required tests are specified, not executed by this review.

### Issue I2: Shader eligibility and target-specific prerequisites

- **Original severity**: major / Important — blocking.
- **Section**: Shader eligibility and common/target gates; slice 6C and dependent hosts.
- **Status**: **ADDRESSED**.
- **Verification**: Design lines 189–197 list all seven source families, twenty rendering entrypoints and `score_world`. Every rendering row remains required for Metal, DX12, Vulkan, native GL, WebGPU and WebGL2. The advisory row requires compute on the four selected compute-capable targets, with explicit “X: CPU baseline” for native GL and “X: no compute” for WebGL2 / GLSL ES 3.00. Line 199 prohibits dropping a rendering family from WebGL2 Low, preserves its particle/bloom policy, and requires a separate gate for any later native GL compute extension.
- **Source and effect**: Read the actual entrypoint declarations in `assets/shaders/primitives.wgsl:23,39`, `space.wgsl:21,34`, `worlds.wgsl:34,49`, `routes.wgsl:29,47,71,95,114,119,127,136`, `ui_sdf.wgsl:53,62,67,78`, and `postprocess.wgsl:12,25`. They account for the twenty rendering entrypoints. `assets/shaders/advisory.wgsl:13–18` declares the three storage buffers and workgroup-size-8 compute entrypoint. Design lines 201–207 separate common validation/reflection from each target's eligible compilation/link probe; required failure, required unexecuted work and capability exclusions have distinct manifest states. The host rows at lines 349,351,354–355 depend on their applicable gates rather than every foreign compiler. No unsupported compute prerequisite or rendering-scope deletion remains.
- **Evidence boundary**: Entrypoint and source-contract inspection establishes the corrected specification only. Translation compatibility, target compilation, driver execution and actual advisory parity remain unverified.

### Issue I3: Noncircular proposal, authorization and tooling sequence

- **Original severity**: major / Important — blocking.
- **Section**: Overview; shader tooling; offline dependencies; slices 6A1/6A2/6C.
- **Status**: **ADDRESSED**.
- **Verification**: Design lines 298 and 345 allow 6A1 to inventory existing target headers/packages and propose missing prerequisites before dependency authorization. “No install/download/new linking/build probe” makes that collection phase read-only; absent packages stay unverified. Line 346 puts new library linkage/acquisition and target ABI probes in 6A2 after the relevant proposal, authorized implementation scope and any explicit user dependency/tool authorization. Existing macOS framework probes do not depend on Linux approval.
- **Source and effect**: Design lines 17,185,317 and 348 put the recorded offline-generator-language decision before 6C starts; Task8 audits the prior decision. If the proposed offline Rust boundary is disallowed, an approved alternative must precede 6C. This resolves both original cycles and agrees with `docs/superpowers/plans/2026-10-09-nyon-zig-migration.md:196–210`, which requires a reviewable binding design before adding production dependencies. The new text explicitly says independent design approval supplies no external user authorization and cannot create a language exception.
- **Remaining gate**: Concrete target versions/closure/notices, the language decision and any required scoped authorization remain future prerequisites. Their absence is not represented as a failed or completed implementation slice.

### Issue M4: Concrete numeric conversions and packing

- **Original severity**: minor — renderer-interface recommendation.
- **Section**: Numeric-boundary contract; logical geometry; 6A2/6C.
- **Status**: **ADDRESSED**.
- **Verification**: Design lines 213–224 distinguish source-preserved operations from Proposed host guards. They specify finite/range checks before narrowing, authoritative integer host sizes, zero-extent suspension, physical rounding ties, outward scissor rounding then clamping/narrowing, checked count/addition/stride/capacity arithmetic, and boundary/invalid-input tests. Canonical integer authority values never receive presentation rounding.
- **Source and effect**: The primitive/UI viewport distinction matches `src/engine/render.rs:230–247` and `src/engine/ui_renderer.rs:240–245`; checked uploads and capacity fallback match `src/engine/render.rs:354–378`. Primitive radius/line/corner/color checks and quad clamping order match `src/engine/primitives.rs:48–115,143–164`. The retained f32 ring quantizer and kind match that file's lines 3–6 and 182–189, with 36-byte layout and offsets 0/8/24/32 witnessed at lines 327–333. The proposed contract preserves operation order and avoids claiming a blanket raw-input rule or color clamp absent from the source.
- **Evidence boundary**: No numeric execution, packing test or target ABI probe was run. The document now supplies the concrete behavior needed for those future checks.

### Issue M5: Pure interfaces before integrated player qualification

- **Original severity**: minor — dispatch clarity.
- **Section**: Dispatch dependency graph; 6G/6H/6L/6M; final acceptance mapping.
- **Status**: **ADDRESSED**.
- **Verification**: Design lines 27–56 explicitly route reviewed pure 3A–3I, 4A–4H and 5A–5K/headless 5L interfaces into applicable 6B–K integration. Graphical 3J, durable player Forge and Task5 adoption are resulting qualification obligations. Line 353 says 6H “produces graphical 3J and Task5 adoption evidence, does not require them in advance.” Task7 corpus/review/freeze and separately approved strategic architecture/specification/plan feed 6L without requiring final Task6 or Task7 player completion first. Lines 357–358 and 403–405 retain full integrated acceptance before 6M/Task8 retirement.
- **Source and effect**: The graph matches `docs/superpowers/plans/2026-10-09-nyon-zig-migration.md:95–106,127–130,146–158,212–231` and `docs/qualification/2026-10-09-zig-living-inventory.md:119–120`. The clarified prerequisite names do not waive the runtime handoff or make pure-core results whole-task completion. Tasks3/5/6/7 remain Partial until their integrated qualification.

### New breakage in the fix diff

**None found.** No new critical, major/Important, minor or nit finding is raised by this scoped review. The eight hunks correct the findings without removing another requirement. All LG-01 through LG-27 rows remain at design lines 374–400; complete Classic/Forge/V3/strategic products remain at 401–407. The required hosts, browser fallback paths, 46 tools, seven experiments, interaction journeys and unavailable-host Pending status remain at 413–421. Strategic approval remains separate from this platform review.

### Out-of-scope observations

None raised. Existing unanswered architecture choices and unexecuted external-platform evidence remain explicit future gates; this re-review does not reopen untouched design or endorse exact foreign API signatures.

### Strengths

- The scheduler correction names independent counters and tests the recycled-ticket failure mode directly.
- Shader eligibility preserves complete rendering scope while distinguishing intentional exclusions from failure or missing evidence.
- The corrected prerequisite graph makes useful read-only preparation possible without manufacturing dependency or language authorization.
- Numeric conversion semantics and final integrated acceptance remain inspectable alongside byte-layout and pure-interface contracts.

### Checks and limitations

Read the complete revised design, original review, fix report, fix-only patch, preparation brief, home/repository instructions and scoped re-review prompt; verified the cited source/contract sections with bounded reads. The fix report includes exact document-check commands/output and actual exit codes, including its initial checker failure and corrected exit 0. Its keyword checks are document evidence only, not behavioral tests.

Independent read-only identity/patch check: **exit 0**. Python `hashlib` verified the revised design, saved before-snapshot (`4c6da68f810a2346c9f6d6aaf8206962214e1a40bd253175f861ac3a57037f59`) and original review (`ef0de73ef5016dc4744388d6df81991ad85344efcf19b1a4ddaae593ad6c8ea6`) identities. A line-by-line in-memory application of all eight supplied patch hunks checked each old/context line against the saved snapshot and reconstructed the revised design exactly, without modifying any input. Output: `PASS frozen design/base/review identities and 8 fix-only hunks reconstruct revised design exactly; no files written`.

No repository gate, unit test, shader/GPU/compiler probe, native/browser journey, external documentation fetch, installation, dependency mutation, profile write, commit or publication was run. Source reads do not qualify runtime behavior, remote hosts or manual acceptance. The only written artifact is this review. Review structure/whitespace and final frozen-input identity verification are recorded by the review-validation command's actual exit code.

### Verdict

**All five findings ADDRESSED; no new Critical/Important breakage.** Accept fix round 1 as the corrected design prerequisite, preserving the named architectural decisions, explicit user authorization boundaries, target gates and full integrated runtime/product acceptance.
