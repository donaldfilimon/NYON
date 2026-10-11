## Design Document Review: NYON Zig Platform, Shader and Atlas Implementation Plans

### Summary

**CHANGES_REQUESTED — specification compliance and implementation readiness.** The plans preserve the approved architecture, full product/host scope and acyclic candidate qualification path, but five Important gaps prevent recommending execution as written. **0 Critical, 5 Important (major), 2 Minor** findings follow; all are plan findings, not claims of failing implemented code.

Reviewed independently on 2026-10-09 in canonical `/Users/donaldfilimon/dev/active/NYON`, already-dirty `main`, HEAD `bac2b6e313f8034309bd718d3b8cb4e0448ac20f`. Read all three plans and the approved platform design, the writer report, prior architecture/tooling findings and their scoped dispositions; inspected the relevant current source contracts. Only this review file is owned by this dispatch. Approval of the specification at 06:52 EDT authorized planning; this review authorizes no implementation, dependency acquisition, changed assets, source retirement, commit or publication.

Line references below refer to the frozen files in the next table. `platform`, `shader`, `atlas`, and `spec` abbreviate those file names, not different documents.

| Artifact | SHA-256 |
| --- | --- |
| `docs/superpowers/plans/2026-10-09-nyon-zig-platform-bindings.md` (platform, 43 tasks) | `6e2472b91964b04c423900fb2d4a8db935d97f549c88f8c1b2c7ce157e3b37ef` |
| `docs/superpowers/plans/2026-10-09-nyon-zig-shader-tooling.md` (shader, 15 tasks) | `da3f9f7bb4b74eadd1f393e90e75b07e6ac4d0416e20ed25343f7037319b063f` |
| `docs/superpowers/plans/2026-10-09-nyon-zig-atlas-tooling.md` (atlas, 10 tasks) | `57e41980cf0a9089c7d4b2a4660217df70b67556add05291409f481a11849d86` |
| `docs/superpowers/specs/2026-10-09-nyon-zig-platform-bindings-design.md` (spec) | `44ad428bf9171fd4e90f4d8d02184315dbeb2b6c61f86794a074c1a9ac58620e` |
| `.superpowers/sdd/2026-10-09-nyon-zig-migration/task-6-user-approved-platform-spec.md` | `1d3257c31e7383118e46a78e6550e83752b4f6d2cf2fb74fc6d79b1af0c04d04` |
| `.superpowers/sdd/2026-10-09-nyon-zig-migration/task-6-platform-plan-report.md` | `68e284981a6ee73dcd11b45e7dfc2104d0e8c6d82120f1c1cba690d0f9e74d40` |

### Issue 1: Classic browser persistence has no concrete implementation owner

- **Severity**: major
- **Review class**: Important — blocking completeness gap
- **Section**: Platform H11, lines 377–389; H13, lines 416–428; H18a/b, lines 520–545; file map, lines 55–60. Spec “Identities and compatibility,” line 376, and 6G, line 515.
- **Description**: H11 says to “Adapt ... existing RawStore,” while H13 implements only the Workshop/Living/coordinator IndexedDB databases and H18's files/services omit a Classic storage bridge. The existing Zig `RawStore` is an injected interface; it is not a browser implementation. Neither the scenario/preferences localStorage key pair nor the browser implementation is assigned a task. Generic final Classic/browser journeys do not tell an implementer which adapter to produce. This leaves browser scenario and preference persistence, primary-versus-legacy precedence, and denied/quota outcomes unimplemented or invites moving them into the wrong IndexedDB schema.
- **Evidence**: `zig/src/classic_store.zig:99–115` defines callbacks through `context`, `load_fn` and `save_fn`; its concrete implementations are fallback, unavailable, memory and native stores. Current browser scenario code is `src/platform/web.rs:19–20,47–97`, using `nyon.scenario.v1` and read-only `intergalactic-warfare.scenario.v1`; browser preference code is `src/preferences/store.rs:11–12,226–287`. The approved spec explicitly includes `web/zig/classic_store.js` in 6G and preserves 65,536-byte scenario / 16 KiB preference limits.
- **Suggestion**: Add a bounded Classic browser adapter task or explicit H11/H18 sub-slice with owned Zig and minimal JS files, exact RawStore callback/ownership signatures, both current and legacy keys, byte caps and typed errors. Add compiling unit RED→GREEN cases and actual-browser tests for absent primary fallback, present invalid/oversized/errored primary preventing fallback, no copy-on-read, primary-only writes, unavailable storage, quota/permission failure and reopen. Keep the existing schema and accepted Task2 policy.
- **Status**: open

### Issue 2: The browser startup timeout begins after the event it must bound

- **Severity**: major
- **Review class**: Important — blocking correctness mismatch
- **Section**: Platform H18a Step 1, line 526; spec browser delivery, line 439.
- **Description**: `fallback20s` specifies a “readiness deadline 20 seconds from first frame.” The first successfully rendered frame is the readiness condition. If GPU initialization never produces that frame, this start point never occurs, so the deadline cannot drive fallback or actionable failure. This changes the preserved graphics-ready/failed handshake.
- **Evidence**: `web/loader.js:105–173` installs the graphics observer and timeout; `:176–189` creates it before importing/initializing the artifact. The spec says readiness follows required resources and a rendered frame and retains the existing 20-second handshake; it does not start a further timer after readiness. The current source is a timing-contract reference, not a claim that every current Promise stall is already handled correctly.
- **Suggestion**: Define a per-attempt monotonic start and 20,000 ms deadline before initialization, with expiry able to progress even if the initialization Promise never settles. Readiness cancels that deadline only after the required first frame. Add independent-clock cases for never-ready, ready just before expiry, expiry, late ready/error after fallback, forced WebGL2 timeout and both-failed cleanup. State whether preflight has its own bounded deadline; do not conflate its timeout with successful first-frame time.
- **Status**: open

### Issue 3: Compiled Metal/DX12 payloads disappear at the shader API boundary

- **Severity**: major
- **Review class**: Important — blocking artifact/ownership contract gap
- **Section**: Shader public types, lines 50–52; S05/S06, lines 160–182; S10, lines 227–237; platform H10, lines 364–376, and H25/H26, lines 730–754.
- **Description**: S10 freezes a candidate from `emitted:[]const Emitted`. S05/S06 then compile that candidate with system compilers, but `compile(...)` returns only `ShaderTargetReceipt{candidate_id,target,compile_interface,runtime,allocator}`. No declared output contains the resulting metallib/DXIL bytes, payload descriptors, or a mapping from the pre-compilation candidate to immutable compiled payloads. Downstream hosts and release closure consume the original candidate. The plan simultaneously requires immutable candidate bytes, exact output hashes and no rebuilt payload during promotion. An implementer must invent where compiler outputs live, what hashes bind them, and how loaders distinguish them from emitted MSL/HLSL.
- **Evidence**: The approved spec's target table (lines 243–244) requires system-produced metallib/DXIL, its manifest contract (line 254) binds system compiler/options and output hashes, and candidate fields (lines 286–291) require payload identity before loading and at promotion. Shader S12's staging clarification solves driver availability, but does not supply this missing artifact transition. This is an internal plan interface gap; no SDK execution is asserted.
- **Suggestion**: Define an owned compiled-target artifact/result with immutable payload paths, lengths/hashes, entrypoint mapping, parent structural-candidate ID and compiler/flags identity. Choose explicitly whether compilation creates a new immutable target candidate or a separate bound compiled artifact. Specify exact signatures for probe output, witness attachment, host loading and release closure. Add corruption/deletion/wrong-compiler-output tests and show that the exact compiled bytes read by the host are the bytes promoted and packaged, without mutating the structural candidate or rebuilding them at promotion.
- **Status**: open

### Issue 4: Independent verification stops before target lowering

- **Severity**: major
- **Review class**: Important — blocking compiler safety requirement gap
- **Section**: Shader S03b, lines 121–132; S04 and emitter interfaces, lines 147–164; public `Lowered` description, line 50. Spec typed IR paragraph, line 235.
- **Description**: The spec requires independent rechecking “after lowering; malformed IR cannot reach an emitter.” The only defined verifier is S03b `verify(module:*const Module,...) -> Verified`, before S04. S04 produces `Lowered`, and every translated emitter accepts `*const Lowered` directly. “Immutable after target verification” names no verifier owner, signature, checked result type or post-lowering mutation tests. Source-IR verification and numerical examples do not establish the validity of newly introduced target operations, bindings or control flow.
- **Suggestion**: Assign an independent target-IR verifier after S04 and before S05–S08 emission, with its exact input/output/error/lifetime contract. Emitters should consume a verified target view/token that cannot be produced by unchecked lowering. Add independently mutated lowered-IR cases for wrong type/use, illegal target stage/resource, missing return/merge, invalid binding/layout, unproved index and coordinate-conversion metadata; prove emitter admission fails before target output publication. Preserve S03b as the independent source-IR check.
- **Status**: open

### Issue 5: Runtime observability and privacy requirements are absent from task ownership

- **Severity**: major
- **Review class**: Important — blocking specification coverage gap
- **Section**: Platform file map, lines 46–63; H22b/H23, lines 665–689; spec “Security, privacy, and failure observability,” lines 479–485.
- **Description**: The approved design requires structured local runtime diagnostics, specific redaction, bounded opt-in dumps, lifecycle/store/GPU counters and a local diagnostic UI for invariant failures. H22b owns performance qualification traces, H23 owns test receipts, and individual adapters expose errors, but no task owns the runtime diagnostic schema, retention, rendering or redaction. The claimed every-section coverage table omits this spec section. Implementers could complete all listed task signatures without producing the diagnostic surface or preventing raw clipboard/save/path text from reaching logs.
- **Evidence**: Spec line 483 explicitly excludes save content, names, clipboard text, personal paths and notebook text from logs; line 485 names queue/budget/fence/stale/CAS/recovery/close-age metrics and local invariant alerts. Current `zig/src/classic_store.zig:35–66` can carry caller/platform diagnostic messages, making safe translation at the new host boundary a concrete requirement. Performance artifact receipts are a different consumer from runtime operator diagnostics.
- **Suggestion**: Assign a small common diagnostic slice with exact bounded record/counter interfaces, approved fields, redaction and local UI ownership, then wire adapter errors into it. Add payload/path/clipboard marker tests asserting no sensitive values reach records or dumps, bounded overflow behavior, correct committed/unknown/aborted disposition, and invariant alerts that preserve authority without blind mutation retry. Add the missing section-to-task mapping. Keep telemetry disabled and do not treat this as permission for a new service.
- **Status**: open

### Issue 6: U01's unit test cycle depends on later atlas implementation

- **Severity**: minor
- **Review class**: Minor — task ordering correction
- **Section**: Atlas U01 Step 1, line 86; U03, lines 106–118; U04, lines 119–131; staging paragraph, line 64.
- **Description**: U01 calls `place(&mask,&cell)` in its required unit test, but U04 owns that function. U01 also names generated-cell differences for all glyphs/icons before U03 geometry and U04 SDF exist. Deferring the real `measure` CLI until U09 addresses the measurement driver, but not U01's earlier compile-only/GREEN test dependency. Forward scaffolding would require U01 to touch a later owner's file or leave its own required test failing until U04.
- **Suggestion**: Keep U01's independent test cycle on mask validity, font-byte selection and raster ownership. Move placement failure assertions to U04 and complete generated-cell comparisons to a clearly named U01 follow-on measurement checkpoint after U03–U05/U09. Alternatively explicitly reorder those pure prerequisites. Do not weaken full 205-entry compatibility or candidate gates.
- **Status**: open

### Issue 7: Exact-contract prose still contains malformed tokens and one missing receiver

- **Severity**: minor
- **Review class**: Minor — clarity/type consistency
- **Section**: Platform shared host contract, line 120; H03, line 234; H18a/b, lines 526–542; final self-review, line 825; analogous scattered spacing in the tooling plans.
- **Description**: H03 declares `Scheduler.begin(owner:Owner,payload_bytes:u32)` without a scheduler receiver, unlike its poll/cancel/update operations and the later native-store instance methods. The prose also contains damaged tokens such as “DO Maudio GPU handlersandownerbeforefallback,” “authentic ated,” “JSO Nsave,” and “name downing tests.” These survived the writer's reported spacing repair. They make an already large implementation contract harder to consume accurately. Separately, `deinit(*Host) void` is said to begin asynchronous retirement; the task should explicitly identify who retains and pumps that host until a terminal retirement outcome before its allocator storage can be freed.
- **Suggestion**: Correct the receiver and perform a prose/token pass that preserves code identifiers. Clarify the asynchronous host close/retirement/final-destruction API and owner rather than using ordinary `deinit` wording for a nonterminal operation. This is a narrow contract edit, not a request to rewrite the architecture or future pure-domain APIs.
- **Status**: open

### Strengths

- The plans retain all 27 LG rows, full Classic/Workshop/Living/V3/strategic products, all six render targets, twenty rendering entries and exactly four compute targets. Both GL rendering paths remain required. All 46 creator tools, seven experiments, eighteen-step journeys, named screen readers, viewport/scaling matrix, actual storage/readback, performance/endurance and cold-offline delivery remain explicit obligations.
- The original architecture review's scheduler, shader eligibility and dependency-proposal corrections remain represented. The original tooling cycle is not reintroduced: U06 candidate admission precedes available target hosts, H23/U07 gather witnesses, U08 joins all targets, then H25/H26 admit release/delivery. The explicit graph is acyclic. Shader/atlas CLI implementation consumes module interfaces rather than requiring its own future executed receipts.
- F3C–I/F4/F5/F7 source-signature intake is explicit, refuses invented APIs and does not make complete graphical products prerequisites of the pure cores that enable them. Deferring exact not-yet-accepted domain signatures to reviewed annexes is appropriate. It does not excuse gaps in the new platform/tool interfaces identified above.
- The three scheduler counters remain separate from physical tickets; shared failed-work charging, no-session retirement, private publication, commit/cancel truth, independent coordinator CAS and Workshop abandonment are concrete tests. Current source confirms the RawStore ownership/primary precedence and actual Living/coordinator generation keys and transaction-complete semantics.
- Zig owns retained project tooling, including replacements for Python formula/Classic drivers and shell orchestration. Minimal platform adapters and external system compilers/rasterizers stay distinct. No source deletion or changed asset version is authorized by a passing candidate receipt.
- Atlas exact compatibility remains complete byte equality across all three outputs plus repeated generation. A mismatch requires explicit changed-version authorization before candidate admission; copying the old atlas or weakening pixel thresholds cannot qualify a generator.

### Evidence, independent checks and limitations

**Current — independently verified:** all three frozen plan hashes, writer-report hash, both spec identities and the writer's 22-file source binding match. No source-binding drift was found; canonical sorted binding digest remains `b845876fd295a0b7887e40c6127d4674746c90098c5217f49202c47dfd1f8856`. A read-only Python check returned originating process **exit 0**, validating task counts 43/15/10, local links/anchors, balanced fences, trailing whitespace, 27 LG rows and the explicit 22-edge DAG. It independently counted 21 shader entries (20 render/one compute) in 17,131 source bytes and checked all 21 recorded asset hashes, 205 metrics entries and the 1,048,576-byte atlas. These are structural/data checks, not compilation or behavior proof.

The read-only check used `hashlib.sha256` over actual file bytes, parsed the writer's source-binding JSON, compared each recorded source hash, parsed local Markdown links/anchors/task headings, traversed graph edges with an active-node cycle check, enumerated actual WGSL entry declarations and validated `assets/ui/hashes.json` against each named file. It created no script or product artifact. `git rev-parse HEAD` returned exit 0 with the HEAD above. The assigned report path was absent before creation. A bounded cwd-holder observation returned no matching open cwd; that cannot prove absence of concurrent owners, and the known shared dirty work was preserved.

**Source read:** current `zig/build.zig`, the migration gate, Classic AppCore/commands/RawStore sections, real browser Classic scenario/preferences stores, browser startup observer, semantic input ownership, Living/coordinator IndexedDB protocol, atlas generation/placement/SDF and relevant shader/resource declarations. The gate still invokes the temporary Python/Rust migration comparisons. It was read, not executed. Source contracts establish the review's factual anchors; none of the proposed host/compiler/generator functions exists merely because a plan names it.

**Review guidance:** read `~/AGENTS.md`, active-container/repository guidance and installed `/Users/donaldfilimon/.codex/plugins/cache/openai-curated-remote/superpowers/6.4.2/skills/writing-plans/SKILL.md`. That installed writing-plans directory contains `SKILL.md` and `agents/openai.yaml`; no plan-reviewer prompt/reference was found there or by the bounded plugin filename search (search exit 1, no matches). Applied its task/interface/self-review criteria and the supplied senior-review format. This missing prompt did not prevent independent substantive review. Memory supplied only the caution to keep narrow NYON observations distinct from acceptance; all review findings and source identities were checked from current documents/source.

**Proposed / unknown:** SDK/library availability and ABI correctness, system shader outputs, exact platform raster equivalence, actual browser/GPU/IME/accessibility/touch behavior, Windows/Linux crash semantics, throughput/latency/memory and packaged offline acceptance remain unverified. No linked external platform documentation was independently requalified; this review relies on the approved external-boundary proposal and raises no newly verified SDK capability claim. A concrete target proposal/probe remains required before using those APIs.

**Out of scope / not run:** repository native/wasm/fuzz gates, `./zig/tools/check.sh`, builds, unit/runtime tests, shader/atlas generation, dependency installation/download, profile mutations, final artifact packaging, commits, pushes and external messages. No product source, plan, asset, earlier raw finding or prior review status was modified. Prior raw findings retain their historical status; the scoped fix reviews remain the disposition records. This review cannot close Task6, Task8 or any LG/runtime row.

Review-file validation returned originating process **exit 0**: seven issue sections, five major/two minor severities, seven open statuses, balanced fences and no trailing whitespace. The same check rehashed all three plans after this review was written and confirmed their frozen identities remained unchanged.

### Verdict

Resolve Important issues 1–5 before recommending the plan set for execution approval; correct Minor issues 6–7 before their task dispatch. Preserve the approved scope, source-signature intake, full matrices and acyclic candidate paths. Re-review the corrected contracts and changed dependency edges; user plan approval and scoped execution remain separate subsequent gates.
