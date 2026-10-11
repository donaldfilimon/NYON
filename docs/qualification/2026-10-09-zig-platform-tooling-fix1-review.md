## Design Document Review: NYON Task 6 — Zig-only tooling FIX1

### Summary

**APPROVED** for the scoped re-review of tooling review Issue 1. The revision removes the asset-admission/host cycle, preserves the independent shader-probe path, and introduces no new blocking issue in the changed paragraphs, graph or host rows.

This is independent design-review acceptance of this fix, not Donald's written-specification approval, implementation dispatch, dependency/tool authorization, asset-version approval, source retirement or product acceptance. The design remains **Draft/Proposed**. No Task6, Task8, LG or runtime row is closed.

### Review scope and frozen identities

Reviewed on 2026-10-09 in canonical `/Users/donaldfilimon/dev/active/NYON`, on already-dirty `main`. Read the complete 598-line revised design, the fix author report and prior tooling Issue 1; examined every changed design paragraph and relevant current source. The supplied fix package was reconstructed against its immutable pre-fix snapshot entirely in memory. Only this review file was written. Prior raw review/status files remain unchanged.

| Artifact | SHA-256 |
| --- | --- |
| Revised `task-6-binding-design.md`, 141,596 bytes / 598 lines | `50c5b1e6925f2fd4077d30dd2d2d283ea31ce2a4436a432139ea053130560add` |
| `task-6-binding-design-zig-tooling-fix1-before.md` | `fd45871344e69840c38b2b65db78cb70787a0f68f8bc9e6a00509371879209c1` |
| `task-6-binding-design-zig-tooling-fix1-package.patch`, five hunks | `742a321601f7d096e1b23c900a0c28d3276d9237ec1c2b6770fbb9936fbab504` |
| Author `task-6-binding-design-zig-tooling-fix1-report.md` | `6abbc387907d2237b520a026ac3d2a8f7ee5557e6b4b38501d8518c148dda7e1` |
| Prior `task-6-binding-design-zig-tooling-review.md`, unchanged | `9e7b3565c799285dc85b6881fae637965207ff6b907c7aec5671282128c2043f` |

The earlier tooling report and root-owned durable Proposed specification also retain their supplied identities, independently checked below. Their contents/status were not updated as part of this review.

### Disposition of prior Issue 1: Asset-version acceptance requires the host that the asset gate prevents

- **Original severity**: major / Important, blocking.
- **Status in this re-review**: resolved for the frozen FIX1 design. The original review's raw `open` status is preserved.
- **New findings**: none within the assigned fix and its changed dependency surface.

The fix answers the original issue in all affected locations:

| Design section and exact location | Evidence and disposition |
| --- | --- |
| “Shader eligibility and common/target gates,” lines 205–208 | Defines 6C-assets as final all-target release admission and states it “is not a prerequisite for executing the qualification hosts that produce those witnesses.” An authorized target host consumes its own common/shader/pure/ABI/service prerequisites and a 6U3a receipt. Missing Windows/Linux evidence stays pending without preventing macOS qualification. Release manifests and ordinary packaging remain gated. |
| “Owned Zig atlas generator and compatibility gate,” lines 271–273 | Exact-compatible admission requires all generated atlas/metrics/hash bytes to match frozen identities and repeat deterministically. A mismatch rejects that mode. Changed bytes need a separately approved version proposal before candidate admission; full actual target witnesses follow admission and precede final release. Current generated assets remain preserved, and release-manifest promotion alone does not authorize replacing them. |
| “Qualification candidates, target witnesses, and release promotion,” lines 277–285 | The candidate ID “hashes the frozen payload/identity manifest, not mutable test status.” Payload length/hash, source/profile/tool/layout/raster/options and approval-scope identities are pinned; immutable witness receipts separately bind the candidate to exact host/JS/Wasm, environment, backend/scale, fixture and result identities. Changed inputs invalidate affected witnesses. Isolation, explicit expected candidate/target IDs, no fallback and no hot swap prevent accidentally qualifying other bytes. |
| Same section, lines 282–287 | Admission failures reject execution; pending downstream checks are allowed only for the explicitly dispatched qualification stage. “Ordinary release loaders and packagers reject the candidate manifest kind.” Promotion verifies receipt closure, emits a distinct release manifest and does not rebuild payloads or waive exact final executable/package checks. 6U3a → applicable host → 6U3b-target → 6U3c → 6C-assets-release has no release-to-host prerequisite. |
| Shader manifest paragraph, line 248; shader stages, line 289; 6S3/6S4 rows, lines 302–304 | Structurally valid shader candidates reach authorized compiler/sentinel/pipeline and sufficient image/compute probes before release. “These minimal probes do not require a release manifest, 6C-assets-release or a complete game host.” Compile/interface witnesses open the target shader gate; remaining runtime witnesses continue to block release/host acceptance. Test textures/buffers can be pinned independently of atlas release. |
| Tool subdivision rows, lines 308–310; dependency graph and explanation, lines 312–341 | The structural/reproducibility, target witness and all-target promotion responsibilities are separately executable and reviewable. Independently parsed graph has 21 nodes, 25 edges, no directed cycle and no release-to-host/probe or host-to-probe path. Its stated target instantiation and explicit 6U3c join agree with the prose. |
| Sequential implementation rows 6C/6D/6F/6I/6J/6M, lines 505–515 | All four affected host rows explicitly consume a 6U3a candidate receipt and run before 6C-assets-release. 6M requires final asset admission plus other shader/runtime/product gates and rejects qualification-only packaging. The risk row at line 489 now uses the same staged semantics. |
| Permission boundaries, lines 291, 295 and unchanged 6A1/6A2 rows at 501–502 | Specification approval, implementation/qualification dispatch, new tool/dependency acquisition/linkage, changed asset-version approval and later replacement/retirement remain separate. “Issuing a receipt grants no approval” at line 282 prevents admission evidence from becoming authorization. The wording does not promote “Revise for Zig-only tooling” into host or dependency approval. |

No correction to the unchanged compiler/atlas architecture is requested by this re-review. The protected source-authoritative compiler profile, independent witnesses, target eligibility/exclusions, scheduler, numeric rules, stores, pure-interface dependency graph, final tooling closure, alternatives, acceptance mapping and runtime matrix remain outside redesign scope and compare exactly to the before snapshot.

### Strengths

- The fix updates the candidate contract, prose, dependency graph, risk and actual host prerequisites together, eliminating the original planning contradiction.
- Immutable payload identity and separately bound witness receipts avoid a self-referential candidate identity and prevent a changed candidate from inheriting another candidate's passing evidence.
- Available targets can produce useful evidence while missing targets still prevent all-target release. Both browser backends and the complete final matrix remain required.
- Qualification-only artifacts remain observable and isolated, and final release promotion cannot silently authorize asset replacement.
- Shader compile/interface admission is distinct from later runtime acceptance, preserving the minimal-probe route needed to produce those proofs.

### Current source evidence and proof limits

**Current, read-only source evidence:** `tools/build-ui-atlas.rs:199` calls `font.rasterize`; line 237 calls `resvg::render`. Its lines 141–148 distinguish checking the three current outputs from writing them. `src/ui.rs:29–30` embeds the committed atlas and metrics. `tests/ui_assets.rs:35–72` checks payload/coverage contracts, and its lines 74–101 define the recorded-file hash checks. These support the original compatibility risk and the need for an isolated proposed qualification route. They do not implement that route.

**Current, independent read-only checks:** the revised draft and supplied artifact hashes match, the five-hunk patch reconstructs it exactly, thirteen protected sections plus 6A1/6A2/6B and all 27 LG rows are unchanged, and the graph/rows agree structurally. Current data contains 205 atlas entries, 15 SVGs, 1,048,576 R8 bytes and 21 recorded files whose hashes match. This was an ad hoc read-only hash check, not execution of the Rust tests.

**Proposed/unverified:** candidate manifests, receipt validation, release-loader/packager rejection, promoter code and authorization enforcement have not been implemented or executed by this review. CoreText/CoreGraphics compatibility, repeated generator output, target compiler/probe behavior, real UI/accessibility/browser/GPU witnesses, performance and exact release packaging remain unverified. No external system or documentation reference was requalified here; the fix introduces no new external dependency claim needed for this verdict. Source inspection cannot establish runtime behavior.

**Out of scope / not run:** the repository native/wasm/fuzz gates, `./zig/tools/check.sh`, builds, generators, compiler probes, installation/download, real/disposable profile operations, runtime qualification, commits, pushes or external messages. The migration gate was read, not executed. The explicit design-only dispatch excludes those commands. No prior tool or runtime claim was promoted to current acceptance.

### Independent document/source validation

Working directory: `/Users/donaldfilimon/dev/active/NYON`. The command below reads only files and reconstructs the patch in memory; it creates no product tooling. Actual originating process exit: **0**.

```bash
python3 - <<'PY'
from pathlib import Path
import difflib, hashlib, json, re
b=Path('.superpowers/sdd/2026-10-09-nyon-zig-migration')
p=b/'task-6-binding-design.md'
before=b/'task-6-binding-design-zig-tooling-fix1-before.md'
patchpath=b/'task-6-binding-design-zig-tooling-fix1-package.patch'
sha=lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
frozen={
 p:'50c5b1e6925f2fd4077d30dd2d2d283ea31ce2a4436a432139ea053130560add',
 before:'fd45871344e69840c38b2b65db78cb70787a0f68f8bc9e6a00509371879209c1',
 patchpath:'742a321601f7d096e1b23c900a0c28d3276d9237ec1c2b6770fbb9936fbab504',
 b/'task-6-binding-design-zig-tooling-review.md':'9e7b3565c799285dc85b6881fae637965207ff6b907c7aec5671282128c2043f',
 b/'task-6-binding-design-zig-tooling-report.md':'77a71b3a6f4faa781588ba999801fc05182359b02bf39a6d7ae860aad90a1b6f',
 Path('docs/superpowers/specs/2026-10-09-nyon-zig-platform-bindings-design.md'):'f8ae6bb5b052784303b8a2ae0cc6c4cadb3a7ce2a4fe8f9b859d07076d8ac85e',
}
for path,digest in frozen.items(): assert sha(path)==digest, path
old,new=before.read_text(),p.read_text()
assert p.stat().st_size==141596 and len(new.splitlines())==598
base=old.splitlines(keepends=True)
patch=patchpath.read_text().splitlines(keepends=True)
assert patch[0].startswith('--- ') and patch[1].startswith('+++ ')
out=[]; cursor=0; i=2; hunks=0
while i<len(patch):
    h=re.fullmatch(r'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@\n',patch[i])
    assert h, patch[i]
    start=int(h[1])-1
    assert start>=cursor
    out.extend(base[cursor:start]); cursor=start
    assert len(out)==int(h[3])-1
    i+=1; consumed=emitted=0
    while i<len(patch) and not patch[i].startswith('@@ '):
        line=patch[i]; tag=line[0]; assert tag in ' +-'
        if tag in ' -':
            assert base[cursor]==line[1:]
            cursor+=1; consumed+=1
        if tag in ' +': out.append(line[1:]); emitted+=1
        i+=1
    assert consumed==int(h[2] or 1) and emitted==int(h[4] or 1)
    hunks+=1
out.extend(base[cursor:])
assert ''.join(out)==new
part=lambda text,a,z:text.split(a,1)[1].split(z,1)[0]
protected=[
 ('### Dispatch dependency graph','## Architecture and permitted foreign boundary'),
 ('### Threads, scheduling, and bounded retirement','## Selected platform services'),
 ('| Shader family and exact source entrypoints','6C has separate gates:'),
 ('### Owned Zig shader compiler: exact corpus and supported language','### Owned emitters, manifests, and independent witnesses'),
 ('### Owned emitters, manifests, and independent witnesses','Manifest identity is a tuple'),
 ('Independent evidence is essential','### Owned Zig atlas generator and compatibility gate'),
 ('### Owned Zig atlas generator and compatibility gate','A small SVG subset is feasible'),
 ('### Numeric-boundary contract','## Durable stores, Continue, and transfer'),
 ('## Durable stores, Continue, and transfer','## Offline delivery, dependencies, and packaging'),
 ('### Final project tooling inventory and language closure','## Alternatives and decisions requiring review'),
 ('## Alternatives and decisions requiring review','## Security, privacy, and failure observability'),
 ('## Complete acceptance mapping','### Runtime and manual matrix'),
 ('### Runtime and manual matrix','## Document validation and remaining work'),
]
for a,z in protected: assert part(old,a,z)==part(new,a,z),a
row=lambda text,prefix:next(line for line in text.splitlines() if line.startswith(prefix))
for prefix in ['| 6A1 read-only proposal collection','| 6A2 authorized target ABI/probes','| 6B scheduler/lifecycle']:
    assert row(old,prefix)==row(new,prefix),prefix
lg=lambda text:[line for line in text.splitlines() if re.match(r'^\| LG-\d\d\b',line)]
assert lg(old)==lg(new) and len(lg(new))==27
assert {re.match(r'^\| (LG-\d\d)',line)[1] for line in lg(new)}=={f'LG-{i:02}' for i in range(1,28)}
scope=part(new,'### Tool prerequisites and review-sized subdivisions before 6C','### Numeric-boundary contract')
fence=chr(96)*3
graph=re.search(fence+r'mermaid\n(.*?)\n'+fence,scope,re.S)[1]
edges=re.findall(r'^\s+(\w+)(?:\[[^\]]*\])? --> (\w+)(?:\[[^\]]*\])?$',graph,re.M)
assert len(edges)==sum('-->' in line for line in graph.splitlines())
nodes={v for edge in edges for v in edge}
adj={v:[] for v in nodes}
for a,z in edges: adj[a].append(z)
visiting=set(); visited=set()
def visit(v):
    assert v not in visiting,('cycle',v)
    if v in visited: return
    visiting.add(v)
    for nxt in adj[v]: visit(nxt)
    visiting.remove(v); visited.add(v)
for v in nodes: visit(v)
def reach(a,z):
    todo=[a]; seen=set()
    while todo:
        v=todo.pop()
        if v==z: return True
        if v not in seen: seen.add(v); todo.extend(adj[v])
    return False
for a,z in [('UA','Host'),('Host','UB'),('UB','UC'),('UC','CA'),('CA','Delivery'),
            ('SC','SP'),('SP','ST'),('ST','CT'),('CT','Host')]:
    assert (a,z) in edges
for a,z in [('CA','Host'),('UC','Host'),('CA','SP'),('UA','SP'),('Host','SP')]:
    assert not reach(a,z),(a,z)
for prefix in ['| 6D macOS Low host','| 6F browser pair','| 6I Windows','| 6J Ubuntu']:
    line=row(new,prefix)
    assert '6U3a candidate receipt' in line and 'before 6C-assets-release' in line
    assert '+ 6C-assets' not in line
links=[x for x in re.findall(r'\]\(([^)]+)\)',new) if '://' not in x]
assert all((p.parent/x.split('#')[0]).exists() for x in links)
assert sum(line.startswith(fence) for line in new.splitlines())==10
assert all(line==line.rstrip() for line in new.splitlines())
cols=None
for line in new.splitlines():
    if line.startswith('|'):
        n=len(re.findall(r'(?<!\\)\|',line))
        cols=n if cols is None else cols
        assert n==cols,line
    else: cols=None
m=json.loads(Path('assets/ui/hashes.json').read_text())
for f in m['files']: assert sha(Path(f['path']))==f['sha256']
assert len(json.loads(Path('assets/ui/atlas-metrics.json').read_text())['entries'])==205
assert Path('assets/ui/atlas.r8').stat().st_size==1048576
assert len(list(Path('assets/ui/icons').glob('*.svg')))==15
print(f'PASS six frozen artifact hashes; revised draft 141596 bytes / 598 lines')
print(f'PASS exact in-memory fix patch reconstruction; hunks={hunks}')
print(f'PASS {len(protected)} protected sections, 6A1/6A2/6B and all 27 LG rows unchanged')
print(f'PASS graph DAG nodes={len(nodes)} edges={len(edges)}; candidate-host-witness-release paths')
print('PASS no release-to-host/probe, atlas-candidate-to-probe or host-to-probe path; all four host rows')
print(f'PASS local links={len(links)}, fences=10, table shape and whitespace')
print(f'PASS current atlas: 205 entries, 15 icons, 1048576 bytes, {len(m["files"])} matching hashes')
print('NOT RUN product gates, builds, generators, compiler probes, runtime or external systems')
print('Author fix report sha256='+sha(b/'task-6-binding-design-zig-tooling-fix1-report.md'))
print('Changed new-line spans='+str([(j1+1,j2) for op,i1,i2,j1,j2 in difflib.SequenceMatcher(None,old.splitlines(),new.splitlines(),autojunk=False).get_opcodes() if op!='equal']))
PY
```

Observed output:

```text
PASS six frozen artifact hashes; revised draft 141596 bytes / 598 lines
PASS exact in-memory fix patch reconstruction; hunks=5
PASS 13 protected sections, 6A1/6A2/6B and all 27 LG rows unchanged
PASS graph DAG nodes=21 edges=25; candidate-host-witness-release paths
PASS no release-to-host/probe, atlas-candidate-to-probe or host-to-probe path; all four host rows
PASS local links=7, fences=10, table shape and whitespace
PASS current atlas: 205 entries, 15 icons, 1048576 bytes, 21 matching hashes
NOT RUN product gates, builds, generators, compiler probes, runtime or external systems
Author fix report sha256=6abbc387907d2237b520a026ac3d2a8f7ee5557e6b4b38501d8518c148dda7e1
Changed new-line spans=[(205, 206), (208, 208), (248, 248), (271, 271), (273, 291), (302, 302), (304, 304), (308, 310), (321, 324), (330, 338), (341, 341), (489, 489), (505, 506), (508, 508), (511, 512), (515, 515), (595, 598)]
```

The first attempt to submit this check failed in the JavaScript orchestration parser before any shell command ran because literal Markdown fence backticks terminated its script string. The corrected submission uses `chr(96)*3` and produced the exit/output above. That tool-submission error is not a repository or product failure.

### Review conclusion

Issue 1 is resolved in the frozen proposed design, and no new critical, major, minor or nit issue was identified within the changed surface. The next boundary remains Donald's review of the revised written specification; this APPROVED verdict does not supply that approval or dispatch implementation.

