## Design Document Review: NYON Zig Platform Plans — Fix 1 Scoped Review

### Summary

**CHANGES_REQUESTED.** Six original findings are resolved in the proposed contracts; original I3 remains open because the new release shader loader cannot identify which target artifact to load. **0 critical, 1 major open, 0 minor open; six resolved.** The compiled-payload ownership problem is substantially repaired, but its release consumer needs one further exact interface correction before the plan set is recommended for human execution approval.

This is an independent scoped review on 2026-10-09 of the seven original findings and regressions introduced by their changed contracts. Unchanged architecture retains the prior approved architecture/tooling dispositions; this is not a fresh reopening of that design. Canonical checkout: `/Users/donaldfilimon/dev/active/NYON`, dirty `main`, HEAD `bac2b6e313f8034309bd718d3b8cb4e0448ac20f`. Only this report is owned. Spec approval authorized planning only; human plan review, scoped execution, target acquisition and retirement authority remain separate.

### Frozen inputs

All identities below were independently checked against actual file bytes. References `platform`, `shader`, and `atlas` identify the corresponding revised plans; line numbers refer to this freeze.

| Artifact | SHA-256 |
| --- | --- |
| `docs/superpowers/plans/2026-10-09-nyon-zig-platform-bindings.md` | `8f988a0cc917b6a1732f397b653aa2a56b5ecbc8e70e0ea09196ca44f0ee98be` |
| `docs/superpowers/plans/2026-10-09-nyon-zig-shader-tooling.md` | `aa3c28eb04778c570f126e5d4f0f036641b3745171096dd9580951c55045e5f6` |
| `docs/superpowers/plans/2026-10-09-nyon-zig-atlas-tooling.md` | `0fe79892a04563674d08e21a043008428bb3d6d7eaca14ae48b7a71d46080aa3` |
| `.superpowers/sdd/2026-10-09-nyon-zig-migration/task-6-platform-plan-fix1-report.md` | `030b876ba64a9958f259ae9f334d5c0b83f12fda89ee8e4349f9919dc2e42697` |
| Original raw `task-6-platform-plan-review.md` | `346c3374e232957a89bec5bb1e1419cddff08767f7ac7208523ab5a9181587a5` |
| Approved `docs/superpowers/specs/2026-10-09-nyon-zig-platform-bindings-design.md` | `44ad428bf9171fd4e90f4d8d02184315dbeb2b6c61f86794a074c1a9ac58620e` |
| Exact `task-6-user-approved-platform-spec.md` snapshot | `1d3257c31e7383118e46a78e6550e83752b4f6d2cf2fb74fc6d79b1af0c04d04` |

The three saved before-files match the original raw review: platform `6e2472b91964b04c423900fb2d4a8db935d97f549c88f8c1b2c7ce157e3b37ef`, shader `da3f9f7bb4b74eadd1f393e90e75b07e6ac4d0416e20ed25343f7037319b063f`, atlas `57e41980cf0a9089c7d4b2a4660217df70b67556add05291409f481a11849d86`. They remain under `task-6-platform-plan-fix1-before/`. Original raw findings and their historical open statuses are preserved; this report is the new disposition record.

### Seven-finding disposition

| Original finding | Disposition | Current evidence |
| --- | --- | --- |
| I1 Classic browser persistence owner | Resolved in plan scope | H11b/EClassicString, platform 458–471; actual browser H18a/H23 |
| I2 Startup deadline begins too late | Resolved in plan scope | H18a, platform 603–615 |
| I3 Compiled shader artifact transition | **Open — major; partially fixed** | Owned child is defined, but release selector is absent at platform 132/142/150 |
| I4 Post-lowering verifier | Resolved in plan scope | Shader S04b 160–171; S05–S08 token inputs |
| I5 Runtime diagnostics/privacy | Resolved in plan scope | Platform 277–297, H05, H23 and coverage mapping |
| M6 U01 forward test dependency | Resolved in plan scope | Atlas U01 80–92, U04 119–131, follow-on 145–150 |
| M7 Receiver/prose/Host destruction | Resolved in plan scope | Platform 154–162, H03 302–309, H08 397–403 |

### Issue 1: Classic browser persistence now has a bounded owner

- **Severity**: major in original review; resolved here
- **Section**: Platform EClassicString/H11b, lines 458–471; H18a/H23 actual browser cases.
- **Description**: The prior missing owner is supplied by `zig/src/platform/web/classic_store.zig` and minimal `web/zig/classic_store.js`. Keys, caps, callback signatures, stable context, caller-owned outcome bytes and sanitized failures are explicit. The plan preserves primary absence as the only fallback trigger and forbids copy-on-read and legacy writes. Source confirms those requirements: `src/platform/web.rs:19–20,63–97`; `src/preferences/store.rs:11–12,247–287`; `zig/src/classic_store.zig:12–31,35–115,123–129`. No browser persistence runtime is inferred from these reads.
- **Suggestion**: No further correction for I1. Enforce the named **EClassicString intake before H11b dispatch**: it must establish existing UTF-16/lone-surrogate conversion, including NUL and boundaries, without guessing normalization/replacement. This remains an explicit unknown and a local H11b prerequisite, not blanket approval or a reason to block unrelated native/tooling work. Actual denied/quota/primary precedence and both-record reopen cases remain required on the named browsers.
- **Status**: resolved

### Issue 2: Startup now expires independently of the initialization Promise

- **Severity**: major in original review; resolved here
- **Section**: Platform H18a, lines 607–614.
- **Description**: “deadline=start_ms+20000” is installed before backend import/init, and independent `onClock` can expire a never-settling Promise. First-frame Ready before the deadline cancels it; equality expires first. Preflight has a distinct 5,000 ms bound, controller bootstrap a separate 20,000 ms mechanical failure bound, and late callbacks cannot publish into the fallback owner. Tests cover boundary/tie, forced WebGL2, both-failed, preflight stalls and real never-settling initialization. This corrects the original first-frame start error and preserves the readiness distinction visible at `web/loader.js:105–189`.
- **Suggestion**: No further correction for I2. Treat the new preflight/bootstrap bounds as proposed choices requiring human plan review and later browser evidence. The plan appropriately promises monotonic expiry when callbacks are scheduled, not an unverified wall-clock scheduling guarantee.
- **Status**: resolved

### Issue 3: Release shader loading still lacks an artifact selector

- **Severity**: major
- **Section**: Platform shared types, lines 94–106 and 132; HostOptions/ShaderBinding, line 142; artifact loader, line 150; H25/H26, lines 817–835. This is a remaining gap in original I3's changed producer/consumer contract.
- **Description**: Compiled payloads now survive through an owned `CompiledTarget` and immutable `TargetArtifact`, which correctly binds parent, compiler/options, entry map and exact payload bytes. However, the release arm is `ShaderBinding=union(enum){candidate:QualifiedShader,release:*const ReleaseManifest}`, while `ReleaseManifest` now contains `shader_artifacts:[]const TargetArtifact`. The loader is `loadShaderPayload(a:Allocator,io:std.Io,root:[]const u8,binding:ShaderBinding,payload_index:u32) ArtifactError!LoadedPayload`. `payload_index` belongs to an entry mapping inside an individual TargetArtifact. There is no selected artifact ID, target or artifact index in the release arm or the loader call.

  An all-target release can legitimately contain Vulkan and GL artifacts each using payload index 0. Loading either from the same release/root produces identical declared function inputs, even though the required bytes and formats differ. The candidate arm identifies its artifact through QualifiedShader; the release arm loses that selection. H25 explicitly assembles all target artifacts, and H26's `target` argument is not forwarded to this loader or defined as creating a verified single-artifact manifest view. Inferring from the OS cannot resolve native Vulkan versus GL, or browser WebGPU versus WebGL2. The plan defines no global payload-index remapping that could resolve this instead. An implementer must invent an interface/selection policy at the exact boundary this fix was intended to settle.
- **Suggestion**: Carry an explicitly selected immutable artifact in a release shader binding (for example, release closure plus selected artifact ID and target), or add an exact selection/validation API and pass its checked result to the loader. Define uniqueness, release-closure membership, expected target/format and entry-map validation, borrowed/owned lifetime, and update host/package consumers consistently. Preserve original child/parent bytes and all-target closure; do not retag or mutate the structural candidate. Add a test with at least two artifacts each having payload index 0, verify each request returns its exact format/bytes, reorder the manifest and repeat, and reject an absent/duplicate/wrong-target artifact or out-of-range local index. Include candidate/release/package correspondence without compilation or cache fallback.
- **Status**: open

### Issue 4: Independent target-IR admission now precedes every lowered emitter

- **Severity**: major in original review; resolved here
- **Section**: Shader S04b, lines 160–171; emitter signatures at 177, 190, 205 and 218; S09 at 231.
- **Description**: S04b owns a separate traversal over Lowered IR and independent profile/layout expectations. The opaque VerifiedTarget has a private constructor, explicit allocator destruction and borrowed immutable IR/layout lifetime. Emitters receive only the token and call `targetView` before output; seal/hash checks reject post-verification mutation. The named mutation tests cover use/type, stage/resource, return/merge, layout/binding, bounds, conversion and capability errors. S03b remains the source verifier. S09 retains its source-verified original-WGSL path because it has no target-lowering stage.
- **Suggestion**: No further correction for I4. Compile and mutation tests are still future evidence; the presence of opaque-token prose is not implementation proof.
- **Status**: resolved

### Issue 5: Runtime diagnostics now have explicit privacy and retention contracts

- **Severity**: major in original review; resolved here
- **Section**: Platform diagnostic schema/H02c, lines 277–297; H05 line 335; H23 line 767; spec mapping line 863.
- **Description**: H02c owns closed enum/numeric records, 1,024 bounded records, fixed counters/reservoirs, latched invariant alerts, caller-owned snapshots and an explicit-consent 262,144-byte dump limit. No arbitrary text/payload/path fields are admitted. H05 owns accessible panel binding; adapter/runtime tasks emit safe events and H23 owns actual target privacy/UI checks. `StoreFailure.message` is a real owned string in `zig/src/classic_store.zig:36–65`; the plan explicitly translates it rather than forwarding it. Tests preserve committed/unknown/aborted outcomes, authority and no-blind-retry behavior.
- **Suggestion**: No further correction for I5. Keep H02c unit work on its fixed schema, redaction and bounds. Later H22b's statistical verification and actual H23 UI/privacy witnesses remain distinct from early diagnostic unit GREEN; no new service or telemetry authority is created.
- **Status**: resolved

### Issue 6: U01 no longer calls later placement or geometry code

- **Severity**: minor in original review; resolved here
- **Section**: Atlas U00 types line 54; U01 lines 80–92; U04 lines 119–131; follow-on checkpoint lines 145–150.
- **Description**: U01's independent unit cycle is limited to mask validity, font-byte selection, failure cleanup and ownership. U00 declares the opaque Geometry pointer contract; the path callback is explicitly unavailable until U03. Placement/overflow assertions now belong to U04. Complete 205-entry measurements run twice after U03/U04/U05 and U09 interfaces exist and before real U06 admission. U09 consumes implemented module interfaces, not already successful admission/target results, so the measurement checkpoint does not create a runtime-evidence cycle.
- **Suggestion**: No further correction for M6. Retain complete output equality and repeatability; a successful measurement report alone is not compatibility acceptance, and a changed-version proposal still needs separate approval.
- **Status**: resolved

### Issue 7: Scheduler receiver and asynchronous Host destruction are explicit

- **Severity**: minor in original review; resolved here
- **Section**: Platform close contract lines 154–162; H03 lines 302–309; H08 lines 397–403; H18a line 612.
- **Description**: `Scheduler.begin(self:*Scheduler,...)` now has its receiver. Host open returns an allocated stable pointer owned by the outer HostStorageOwner, whose allocator/scheduler outlive both active and retiring slots. `beginClose`, budgeted `pumpClose` and terminal-only `destroyClosed` replace deinit-as-asynchronous-completion wording. Pending/blocked destruction frees nothing; callback acknowledgements, physical I/O and GPU fences drain before final freeing. Replacements share quotas, only one retiring slot is admitted, and hidden native/browser loops keep pumping. Named tests cover repeated/stale tickets, pending destruction, callback storage survival and exactly-once terminal free. The original specifically cited malformed tokens were corrected.
- **Suggestion**: No further correction for M7. Preserve the stable outer owner and original allocator through terminal destruction during implementation, and keep forced process termination distinct from graceful completion.
- **Status**: resolved

### Strengths

- The revision preserves all 27 LG rows, complete products, full native/browser/manual/accessibility/performance/offline matrices, twenty render entries on six targets and exactly four compute targets. Both GL renderers remain required and only their compute capability is excluded.
- Compiled target ownership is now concrete: nested paths/maps/descriptors have one owner, compiler flags resolve to exact immutable receipts, runtime witnesses bind `shader=TargetArtifact.id` and `candidate_id=structural parent`, and loaded GPU resources survive their fences. Promotion/packaging forbid rebuilding. I3's remaining selector correction can stay narrow.
- The dispatch index has 29 edges and remains acyclic. H11b is browser-specific; available native candidate hosts do not depend on all-target asset release. Target probes and CLI staging do not require their own future runtime results.
- Unknown future F3C–I/F4/F5/F7 declarations and EClassicString remain explicit scoped intake gates. Plan status never promotes them to Current. 6A1 read-only proposals, separately authorized 6A2, changed asset authorization and final source-retirement/commit authority remain intact.
- Final retained owned tooling remains Zig; temporary migration oracles remain protected until qualified replacement. No gate, target witness or asset-version acceptance is inferred from these documentation edits.

### Evidence and limitations

**Current — independently verified:** the raw original review was read in full; revised shader/atlas plans and changed platform contracts were read with the saved originals and writer report. I independently reconstructed all three complete before/current files from the report's full-context diffs and regenerated the full-context diffs byte-for-byte. Each matched its saved file, frozen current file and report. All changed platform hunks were inspected; unchanged architecture was assessed under the supplied prior verdict, not requalified remotely.

Read-only Python validation checked current/before/protected hashes, all local plan links/anchors, balanced code fences, no trailing whitespace and five Review Focus entries per plan. It independently confirmed 45/16/10 tasks, all 27 LG rows and an acyclic 29-edge graph. The original report's 22 source bindings were rehashed with zero drift; canonical serialized aggregate remains `b845876fd295a0b7887e40c6127d4674746c90098c5217f49202c47dfd1f8856`. Current source reads focused on Classic raw persistence, browser scenario/preferences and loader timing, the actual contracts relevant to these fixes. No source changed by another owner was edited.

**Actual exits:** the initial read-only validator returned exit 1 because I incorrectly asserted that `git diff --no-index --check` must exit 0 when files differ; the preceding plan/link/protected/source-hash checks had passed. The corrected independent validator returned exit 0, recording each of the three Git whitespace checks as **exit 1 with empty stdout/stderr**, and independently asserting no trailing whitespace. Exact diff reconstruction returned exit 0. `git rev-parse HEAD` returned exit 0. These are documentation checks, not tests or builds.

**Proposed / unknown:** all new Zig/JS/compiler/host signatures, target SDK ABI behavior, actual metallib/DXIL outputs, independent lowered-IR tests, raster equality, browser timers/localStorage/Unicode behavior, hardware/manual UI/privacy/performance and packaged offline operation remain unexecuted here. EClassicString must still be established. No external SDK capability was independently verified; the previously accepted external-boundary proposal and per-target intake remain the basis for future work.

**Out of scope / not run:** all native/wasm/fuzz/full migration gates, builds, product tests, generator/probe execution, dependency acquisition, asset/profile/source changes, commits/pushes/publication and subagent dispatch. The parent owns its concurrent Task3D gate; this review neither duplicated it nor claimed its outcome. This report cannot close Task6, Task8 or runtime LG rows. No earlier report, raw finding or plan was modified.

### Verdict

Resolve only the remaining I3 release artifact selection contract and its affected consumer/tests, then obtain a fresh scoped disposition. The six resolved fixes do not need architectural reopening unless that correction changes them. **CHANGES_REQUESTED; no execution or runtime acceptance is granted.**
