## Design Document Review: NYON Task 6 — Zig-only tooling revision

### Summary

**CHANGES_REQUIRED** for written-specification user review: the owned Zig compiler and atlas architecture is viable as a staged proposal, but the asset-version fallback introduces a host/asset acceptance cycle that needs one explicit qualification-artifact path. No Critical issue was found; there is one Important issue below. This verdict concerns the new tooling design, not implementation correctness or user authorization.

Reviewed on 2026-10-09: `task-6-binding-design.md`, 128,916 bytes, SHA-256 `fd45871344e69840c38b2b65db78cb70787a0f68f8bc9e6a00509371879209c1`; the seven-hunk, 100,810-byte tooling package patch against `task-6-binding-design-zig-tooling-before.md`; and the complete author report. Source inspection was read-only. The only owned write is this review file. Canonical `main` was already dirty and ahead of its recorded remote; no branch, source, dependency, generated asset, profile or prior review was changed.

Severity terminology: Critical corresponds to critical; Important to major/blocking; Minor to minor/nonblocking.

### Issue 1: Asset-version acceptance requires the host that the asset gate prevents

- **Severity**: major — Important; blocks recommending this revision as ready for written-spec approval.
- **Section**: “Owned Zig atlas generator and compatibility gate,” draft line 271; “Tool prerequisites and review-sized subdivisions before 6C,” lines 291–314; “Sequential implementation plan and rollout,” lines 481–488. Related manifest admission at lines 209 and 249.
- **Description**: The fallback requires a separately approved asset version to “qualify all UI/image/layout/scale/accessibility journeys before 6C-assets closes.” Yet 6U3 opens 6C-assets only after its complete witnesses pass, the graph puts `CA --> Host`, and the 6D/6F/6I/6J prerequisites all include 6C-assets. Therefore, if CoreText/CoreGraphics does not exactly reproduce fontdue/resvg output, an engineer cannot reach the integrated hosts needed to qualify the new assets without first declaring the assets qualified. This is a concrete branch of the chosen design, not a hypothetical new feature: the existing generator calls `font.rasterize` at `tools/build-ui-atlas.rs:199` and `resvg::render` at line 237, while the new raster seam changes both implementations. The draft correctly treats exact compatibility as unverified, so its approved-version fallback needs an executable qualification route. Line 205 allows shader-only probes with labelled test resources, but that does not specify how candidate assets can exercise the full actual UI and accessibility journeys. “All” journeys behind one global asset gate also risks making a macOS candidate wait for unavailable Windows/Linux hosts, despite the target-local progress rule.
- **Suggestion**: Define an explicit candidate-asset receipt/manifest in an isolated qualification output directory, usable by an authorized target host before release asset admission. It should pin the candidate bytes, source/grammar/tool/raster identities and approved version-proposal identity; retain current committed assets; record pending checks honestly; and be rejected by ordinary release packaging. Split reproducibility/structural admission from per-target UI/scale/accessibility qualification and final all-target release promotion. Permit the applicable host to start from the candidate receipt after its own ABI/common/shader prerequisites, without closing 6C-assets or claiming complete host acceptance. Update the 6U3 row, graph and host prerequisites together. State the same candidate-versus-release distinction for 6S3/6S4 compiler probes, whose sentinel/pipeline checks need executable candidate artifacts before final manifest publication. Preserve 6A1/6A2 authorization, separately approved asset changes, exact old-asset compatibility mode, all twenty entries and all six final targets.
- **Status**: open

### Strengths

- The source-authoritative WGSL frontend is a real lexer/parser/semantic checker with typed structured IR, separate verification/layout, explicit limits and fail-closed publication. The inventoried syntax, builtins and resource/stage shapes cover the inspected seven-file corpus at this architecture level. Exact grammar, overload and lowering vectors are appropriately required in 6S0 before implementation; this review does not claim a compiler exists or that those future proofs have passed.
- Source inspection confirms twenty render entrypoints, the four compute-capable targets and both GL exclusions. Existing rendering families remain required even in Low. The independent document check confirms that eligibility, the pure-interface dependency graph, scheduler, numeric rules, stores, runtime matrix and all 27 LG rows remain unchanged from the approved-fix1 baseline.
- New layout contracts agree with the inspected host/source declarations: packed primitive stride 36, 16-byte primitive/UI globals, 112-byte SceneGlobals, and storage arrays with 84/57/7 f32 elements. Sentinel uploads, hand-authored expected bytes, independent reader checks and deliberate corruption cases address self-confirming reflection errors. Full target layout/driver proof remains future work.
- The derivative and descending-smoothstep risks are source-backed: `primitives.wgsl:48` calculates `fwidth` before shape branches; `space.wgsl:41` and `routes.wgsl:122–139` use descending edges. The pinned [WGSL smoothstep definition](https://www.w3.org/TR/2026/CRD-WGSL-20260921/#smoothstep-builtin) confirms the clamped-polynomial basis. Explicit lowering and independent edge-order/domain vectors are appropriate; no target intrinsic equivalence or equal-edge result is assumed here.
- Atlas ordering, dimensions, threshold and SDF arithmetic agree with `tools/build-ui-atlas.rs`, including outside-cell treatment, the empty-cell special case, f32 operations and output serialization order. The fifteen SVGs contain the named element/path-command subset. The design preserves original inputs, assets and provenance, rejects a copied-atlas generator claim, and explicitly gates raster differences on compatibility or a separate approved asset version.
- Independent image/compute expectations are required instead of compiler-generated goldens. In particular, 6S4-target explicitly permits runtime image/compute execution in a minimal probe or later host, leaving unrun witnesses pending. I do not find a separate mandatory full-host shader cycle in that row; the candidate-artifact clarification in Issue 1 should preserve this distinction. Missing other-target tools do not block common validation or an available target's probes.
- The final tooling inventory includes the actual Python Classic checks invoked by `zig/tools/check.sh`, independent Living formula scripts, vertex/cross-build tools and shell/Rust orchestration. Replacements must preserve independent expectations and mutation detection before retirement. No retained project Rust/Python/shell implementation is grandfathered into final tooling; external SDK/OS tools and minimal adapters have a separate declared boundary.
- The new work does not make graphical 3J or final Task5 runtime adoption prerequisites of the host that creates them. Review/specification, implementation, external acquisition and asset-version approvals remain distinct; this review grants none of them.

### Verified evidence and proof limits

Read in full: the revised design, tooling patch and author report; all seven WGSL sources; `tools/build-ui-atlas.rs`; its Cargo manifest/include wrapper; and all fifteen SVG inputs. Focused source reads covered host resource/layout declarations, shader/asset tests, the migration gate, and named Python tool imports/subprocesses/formula provenance. Those reads establish current source/data contracts, not executed behavior. The author's check claims were not accepted as independent test evidence; the checks below were run separately.

The current committed atlas has 205 entries, 15 SVG inputs, 1,048,576 R8 bytes and 21 matching recorded hashes. No raw intermediate fontdue/resvg mask fixture was generated. CoreText/CoreGraphics equivalence, OS ABI execution, target compilers/validators, SPIR-V validation, browser compilation, GPU image/compute results, host runtime, performance and offline delivery are **unverified**. External references outside the newly relevant WGSL smoothstep definition were not requalified in this scoped review. Unchanged previously reviewed platform/store architecture was not reopened.

No product gate, build, generator, compiler probe, fixture regeneration, installation/download, runtime/profile operation, commit, push or external message was run. `./zig/tools/check.sh` and the repository native/wasm/fuzz gates were inspected or read from instructions, not executed, consistent with this design-only dispatch.

### Exact independent document/source check

Working directory: `/Users/donaldfilimon/dev/active/NYON`. This ad hoc Python check reads documents/source/data only. It is not retained product tooling or a language exception. The patch is reconstructed entirely in memory; nothing is applied to disk.

```bash
python3 - <<'PY'
from pathlib import Path
import hashlib, json, re
b=Path('.superpowers/sdd/2026-10-09-nyon-zig-migration')
p=b/'task-6-binding-design.md'
new=p.read_text(); old=(b/'task-6-binding-design-zig-tooling-before.md').read_text()
assert hashlib.sha256(p.read_bytes()).hexdigest()=='fd45871344e69840c38b2b65db78cb70787a0f68f8bc9e6a00509371879209c1'
patch=(b/'task-6-binding-design-zig-tooling-package.patch').read_text().splitlines(keepends=True)
base=old.splitlines(keepends=True); out=[]; cursor=0; i=2; hunks=0
while i<len(patch):
    m=re.fullmatch(r'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@\n',patch[i]); assert m, patch[i]
    start=int(m[1])-1; out.extend(base[cursor:start]); cursor=start; i+=1; removed=added=0
    while i<len(patch) and not patch[i].startswith('@@ '):
        row=patch[i]; tag=row[0]; assert tag in ' +-'
        if tag in ' -': assert base[cursor]==row[1:]; cursor+=1; removed+=1
        if tag in ' +': out.append(row[1:]); added+=1
        i+=1
    assert removed==int(m[2] or 1) and added==int(m[4] or 1)
    hunks+=1
out.extend(base[cursor:]); assert ''.join(out)==new
sections=[('### Dispatch dependency graph','## Architecture and permitted foreign boundary'),('### Threads, scheduling, and bounded retirement','## Selected platform services'),('| Shader family and exact source entrypoints','6C has separate gates:'),('### Numeric-boundary contract','## Durable stores, Continue, and transfer'),('## Durable stores, Continue, and transfer','## Offline delivery, dependencies, and packaging'),('### Runtime and manual matrix','## Document validation and remaining work')]
for a,z in sections: assert new.split(a,1)[1].split(z,1)[0]==old.split(a,1)[1].split(z,1)[0]
lg=lambda s:[x for x in s.splitlines() if re.match(r'^\| LG-\d\d\b',x)]
assert lg(new)==lg(old) and len(lg(new))==27
links=[x for x in re.findall(r'\]\(([^)]+)\)',new) if '://' not in x]
assert all((p.parent/x.split('#')[0]).exists() for x in links)
assert new.count('```')==10 and all(x==x.rstrip() for x in new.splitlines())
ncols=None
for row in new.splitlines():
    if row.startswith('|'):
        n=len(re.findall(r'(?<!\\)\|',row)); ncols=n if ncols is None else ncols; assert n==ncols
    else:ncols=None
entries=[]; sizes=0
for name in ['primitives','space','worlds','routes','ui_sdf','postprocess','advisory']:
    raw=Path('assets/shaders/'+name+'.wgsl').read_bytes(); sizes+=len(raw)
    found=re.findall(r'@(vertex|fragment|compute)\b[\s\S]*?\bfn\s+(\w+)',raw.decode())
    entries.extend(found); row=next(r for r in new.splitlines() if r.startswith('| `'+name+'.wgsl`:'))
    assert all(entry in row for _,entry in found)
    if name!='advisory': assert len(re.findall(r'\| R(?= \|)',row))==6
assert sizes==17131 and sum(stage!='compute' for stage,_ in entries)==20 and sum(stage=='compute' for stage,_ in entries)==1
metrics=json.loads(Path('assets/ui/atlas-metrics.json').read_text()); assert len(metrics['entries'])==205
manifest=json.loads(Path('assets/ui/hashes.json').read_text())
for f in manifest['files']: assert hashlib.sha256(Path(f['path']).read_bytes()).hexdigest()==f['sha256']
assert len(list(Path('assets/ui/icons').glob('*.svg')))==15
assert Path('assets/ui/atlas.r8').stat().st_size==1048576
print('PASS frozen design SHA-256; patch reconstructs exact design, hunks='+str(hunks))
print('PASS six protected sections and 27 LG rows unchanged; seven local links, fences, whitespace and tables')
print('PASS source inventory 17131 bytes, 7 families, 20 render entries x 6 targets, 1 compute entry')
print('PASS atlas 205 entries, 15 SVGs, 1048576 bytes, '+str(len(manifest['files']))+' matching hashes')
print('NOT RUN product gates, compiler, generator, GPU, raster or host qualification')
PY
```

Actual originating process exit: **0**. Output:

```text
PASS frozen design SHA-256; patch reconstructs exact design, hunks=7
PASS six protected sections and 27 LG rows unchanged; seven local links, fences, whitespace and tables
PASS source inventory 17131 bytes, 7 families, 20 render entries x 6 targets, 1 compute entry
PASS atlas 205 entries, 15 SVGs, 1048576 bytes, 21 matching hashes
NOT RUN product gates, compiler, generator, GPU, raster or host qualification
```

Regex in this check inventories declarations/document structure only. It neither parses WGSL nor validates the proposed compiler. Initial retrieval attempts using ignored-file searches returned no matches; a `wc` invocation also used a mistaken `.patch.md` suffix. Correct explicit paths and ignored-file retrieval resolved those inspection errors; neither was represented as a passing check or product failure.

### Decision and next review scope

Keep the architecture Proposed. Resolve Issue 1 in the document, then re-review only the candidate artifact/admission graph and its host/target scope. The compiler decomposition, original asset preservation, explicit raster risk and final Zig-only tooling closure do not otherwise need redesign on the evidence inspected. All implementation and runtime acceptance remains pending.

Final review-file structural check, run from the same working directory. The first harness invocation returned exit **1** at its raw fence-substring count: it counted literal fence strings inside the recorded validator as well as Markdown fences. Corrected the harness to count fence/issue lines; no finding or source contract changed.

```bash
python3 - <<'PY'
from pathlib import Path
import ast, hashlib
b=Path('.superpowers/sdd/2026-10-09-nyon-zig-migration')
r=b/'task-6-binding-design-zig-tooling-review.md'
t=r.read_text()
assert sum(line.startswith('```') for line in t.splitlines())==6
assert all(line==line.rstrip() for line in t.splitlines())
assert sum(line.startswith('### Issue ') for line in t.splitlines())==1 and '- **Status**: open' in t
code=t.split("python3 - <<'PY'\n",1)[1].split('\nPY\n',1)[0]
ast.parse(code)
assert hashlib.sha256((b/'task-6-binding-design.md').read_bytes()).hexdigest()=='fd45871344e69840c38b2b65db78cb70787a0f68f8bc9e6a00509371879209c1'
assert (b/'task-6-binding-design-zig-tooling-package.patch').stat().st_size==100810
print('PASS review structure, recorded validator syntax, unchanged design identity and patch size')
PY
```

Actual originating process exit: **0**; output: `PASS review structure, recorded validator syntax, unchanged design identity and patch size`.
