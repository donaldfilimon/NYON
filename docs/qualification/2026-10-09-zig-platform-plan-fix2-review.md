## Design Document Review: NYON Zig Platform Plans — Fix 2 Scoped I3 Review

### Summary

**APPROVED for the scoped plan correction.** Original I3 is resolved: release shader loading now names one immutable artifact and expected target, and host recovery, browser fallback, release closure and packaging preserve that selection. **0 Critical, 0 Important/major, 0 Minor open; no new findings.** The six findings resolved by fix 1 remain intact. Human review of the proposed plans and subsequent scoped implementation are separate gates.

Reviewed independently on 2026-10-09 in canonical `/Users/donaldfilimon/dev/active/NYON`. Only this new report is owned. The original raw review and fix-1 disposition were read in full; all three current plans were read, and the two complete changed-file diffs were independently reconstructed and compared with the saved fix-2 before-files. This is an I3/regression disposition, not a new qualification of unchanged architecture or external platform capabilities.

### Frozen inputs

`platform`, `shader` and `atlas` below name the corresponding files in `docs/superpowers/plans/`. Line references identify these exact frozen bytes.

| Artifact | SHA-256 |
| --- | --- |
| `2026-10-09-nyon-zig-platform-bindings.md` | `df9ac3aa72b3b6825ab3e6fd0c322a0fef9657b5157255e369f9338bcc3fb357` |
| `2026-10-09-nyon-zig-shader-tooling.md` | `746619f194cf164484f59bc57e393121fcb681895a33e369092bd5a44d3f0593` |
| `2026-10-09-nyon-zig-atlas-tooling.md` — unchanged | `0fe79892a04563674d08e21a043008428bb3d6d7eaca14ae48b7a71d46080aa3` |
| `task-6-platform-plan-fix2-report.md` | `4e7560077646ebc3906050c4bcc930b473765c20c1e97e4cd41a49e4e383d74a` |
| Original `task-6-platform-plan-review.md` | `346c3374e232957a89bec5bb1e1419cddff08767f7ac7208523ab5a9181587a5` |
| `task-6-platform-plan-fix1-review.md` | `cbe83678e9648cd2a38daa79a21a9e7776db30441e8c656e9bde2c75bcb34f6a` |
| Approved platform spec | `44ad428bf9171fd4e90f4d8d02184315dbeb2b6c61f86794a074c1a9ac58620e` |
| Exact user-approved spec snapshot | `1d3257c31e7383118e46a78e6550e83752b4f6d2cf2fb74fc6d79b1af0c04d04` |

The saved fix-2 before-files independently match fix-1 identities: platform `8f988a0cc917b6a1732f397b653aa2a56b5ecbc8e70e0ea09196ca44f0ee98be`, shader `aa3c28eb04778c570f126e5d4f0f036641b3745171096dd9580951c55045e5f6`, atlas `0fe79892a04563674d08e21a043008428bb3d6d7eaca14ae48b7a71d46080aa3`. Both original writer reports, the approved documents and prior raw findings remain unchanged.

### Issue 1: I3 release artifact selection is now explicit and preserved

- **Severity**: major in the original review; resolved here
- **Section**: Platform shared types, lines 107–110/148/152; “Checked release shader selection,” lines 160–168; H01, lines 254–267; H10/H18a/H20a/H21a; H25/H26. Shader shared imports, lines 40/52, and S12, lines 266–278.
- **Description**: The prior release arm supplied only `*const ReleaseManifest`, so Vulkan and GL could present identical loader inputs for their distinct local payload 0. It now supplies `ReleaseShaderBinding{manifest,request}`, where `ShaderSelectionRequest{artifact_id,target}` names the immutable artifact and expected renderer. `selectReleaseShader(io,root,manifest,request)` validates that binding; every payload consumer revalidates it. `LoadedPayload` returns bytes with artifact ID, target, format and local index, preserving correspondence at the consumer boundary.

  The correction is complete within the assigned scope:

  | Concern | Verified plan disposition and future owning test |
  | --- | --- |
  | Unique release member and target | Selection authenticates the full release closure, rejects absent requested IDs, duplicate IDs including identical duplicates, duplicate target members, missing required targets, wrong target/format and invalid entry maps. Canonical membership sorts target/ID, independently of array order. H01 `release_selection_rejects`; H25 `unique_target_membership` and `closure_order_independent`. |
  | Local payload indices | No cross-artifact remapping or ambient OS/default selection exists. Selection validates entry-map indices; the loader bounds the caller index before payload access, reads only the selected artifact directory and rehashes length/bytes. H01 `release_selection_local_zero` independently fixes different Vulkan/SPIR-V and GL/GLSL330 bytes at local 0, selects both, reorders the manifest and repeats. The six named negative outcomes require zero payload reads. |
  | Borrow and ownership | The allocation-free selection borrows the immutable manifest and ends before its destruction/replacement. Host initialization deep-copies required manifest/closure metadata, rebinds and validates before returning; recovery uses that retained copy. Loaded bytes belong to the caller. H01 `release_selection_borrow` uses independent fixtures/fake I/O; H10/H20a/H21a `release_selection_host_lifetime` destroys the caller manifest after initialization, exercises recovery/reload, rejects a wrong effective target and checks failed-copy cleanup. |
  | Browser fallback | H18a obtains a new explicit artifact-ID/target request per WebGPU/WebGL2 attempt and validates it against the effective renderer/authorization before pipeline creation. `release_selection_fallback` exercises both local indices 0, retained ownership and rejection of a stale reused selection. Existing independent startup deadlines and close pumping are preserved. |
  | Promoted and packaged bytes | H25 receives an explicit artifact root, validates all six target artifacts and retains exact child metadata/bytes. H26 receives that root plus an explicit request list, owns copied requests and uses the same selector for copying and packaged-root reload. Its independently frozen delivery requirements enforce Metal, DX12, Vulkan+GL or WebGPU+WebGL2 as applicable. `package_selection_correspondence` compares candidate/release/package bytes, format and ID for both local-0 artifacts under reordered manifests/requests; duplicate/absent/wrong-target requests and out-of-range access reject. Compiler invocation and developer-cache reads must remain zero. |
  | Full closure versus package subset | Authentication retains every required target's immutable evidence; a platform package may omit other platforms' payload bytes while preserving the unchanged all-target closure. Its selected payloads and every required backend in its explicit package set must be present and rehashed. This avoids incorrectly requiring Windows/Linux payloads merely to load a macOS package without weakening qualification. |
  | Producer alignment and staging | Shader S12 exports exact artifact IDs/targets and retains `QualifiedShader` for candidate probes. The shared H01 definitions are imported once. H01 early tests use independently authored fixtures; Host lifetime tests belong to host tasks, and package correspondence belongs to H26. The later release correspondence assertion is explicitly not a prerequisite for S12 candidate probes. |

- **Suggestion**: No further plan correction for I3. Execute and review these named cases when the owning tasks are authorized; their presence in the plan is not an executed test result.
- **Status**: resolved

### Preservation of the six prior dispositions

Independent before/current comparison shows that only main task bodies H01, H10, H18a, H20a, H21a, H25 and H26 changed, plus shader S12 and the shared selector/import contract. Every other task body is byte-identical. Atlas is entirely byte-identical. The shared asynchronous-close contract, explicit graph and complete acceptance section are also byte-identical.

Accordingly I1's Classic browser adapter/EClassicString intake, I2's pre-init independent expiry, I4's independent post-lowering verifier, I5's bounded diagnostic/privacy owner, M6's atlas test staging and M7's scheduler receiver/terminal Host destruction retain their fix-1 dispositions. H18a adds only explicit shader selection/fallback cases and consumption; it does not change its timing semantics. No finding is reopened, and no historical raw status was edited.

### Strengths

- The former two-artifact/local-0 counterexample is directly assigned an independent test, rather than resolved by array position or backend inference.
- Borrowed bindings, owned Host metadata and caller-owned loaded bytes have explicit lifetimes and failure cleanup. Recovery cannot retain the caller's deinitialized manifest.
- Selection, closure and packaging share one H01 contract and preserve immutable parent/child identities. Qualification candidates remain usable before all-target release; ordinary release still requires its full authenticated closure.
- All 71 task IDs, 27 LG rows, six render targets, twenty render entries, four eligible compute targets, complete product/manual/performance/offline matrices and the acyclic 29-edge graph remain intact.
- No new dependency, tool acquisition, asset replacement, authority behavior, source retirement or platform architecture is introduced by this correction.

### Evidence and limits

**Current — independently verified:** a read-only Python validator returned originating process **exit 0**. It rehashed all three current and three saved before-files; checked seven protected report/spec identities; regenerated both complete full-context diffs byte-for-byte; reconstructed both complete before/current file pairs from those diffs; checked task/body preservation, 45/16/10 task counts, six steps per task, all 27 LG rows, all seven local plan links/anchors, balanced fences and trailing whitespace; traversed the byte-identical 29-edge graph with an active-node cycle check. Both Git `--no-index --check` subprocesses returned **exit 1 with empty stdout/stderr** because the files differ; independent whitespace assertions passed. These are document checks, not compilation or behavior tests.

The original writer's 22 source bindings independently rehash with zero drift; canonical serialized aggregate is `b845876fd295a0b7887e40c6127d4674746c90098c5217f49202c47dfd1f8856`. Actual seven WGSL source files contain 17,131 bytes, twenty vertex/fragment entries and one compute entry. The pin remains `0.18.0-dev.120+9fe22a29b`. Current `zig/build.zig` and migration gate were read: they do not yet implement the proposed platform/tool tasks, and the gate still invokes temporary Python migration drivers. `zig/tools/platform/artifact.zig` is absent; the new selector/loader/closure APIs remain Proposed.

**Source read:** `src/engine/shader.rs` embeds current WGSL and validates before graphics API use; focused current engine/advisory source references identify `create_shader_module`, entrypoint use and `score_world`. This corroborates source-authoritative entry inputs, not a current immutable TargetArtifact implementation. An initial focused search used nonexistent `src/gfx` and returned **exit 2**; source routing was corrected from `rg --files src` to the real `src/engine` paths. That lookup error is not a product/gate failure.

**Proposed / unknown:** every new selector/ownership/closure/package test remains unexecuted. SDK ABI compatibility, actual metallib/DXIL outputs, GPU/browser/raster behavior, platform availability, runtime lifetimes, IME/accessibility/touch, hardware performance/crash behavior and offline delivery remain unverified here. No external SDK capability was independently requalified; the approved platform boundaries and separately scoped target proposals/probes remain their future basis.

**Not run / out of scope:** builds, native/wasm/fuzz gates, `./zig/tools/check.sh`, unit/runtime tests, compilers/generators/probes, downloads/acquisitions, real profiles, asset/source changes, commits/pushes/publication and subagents. The parent's concurrent migration gate is separate and is not claimed by this report. Only this new report was written; no earlier report, plan or source was changed.

### Verdict

**APPROVED — I3 resolved; 0 Critical, 0 Important/major, 0 Minor open.** The corrected frozen plan set is ready for human plan review. Approval of the written spec authorized planning only; the [writing-plans handoff](/Users/donaldfilimon/.codex/plugins/cache/openai-curated-remote/superpowers/6.4.2/skills/writing-plans/SKILL.md) still requires review of the plans before implementation under the already selected SDD method. EClassicString/future pure-interface intake, separately authorized target acquisition/link/probes, raster compatibility or changed-version approval, all actual target/runtime/manual/performance/offline rows, full release/delivery closure and later retirement/publication authority remain open. This review closes no Task6/Task8 or LG runtime acceptance row.
