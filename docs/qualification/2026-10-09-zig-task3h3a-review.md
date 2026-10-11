# Independent H3a implementation review

2026-10-09. **Spec compliance: PASS. Task quality: Approved.** Zero actionable Critical, Important or Minor findings for the frozen pure command-deck/settings semantic slice. Parent aggregate acceptance remains outstanding; this is not full Classic/game acceptance.

## Scope and method

Applied SDD 6.4.2 task-reviewer contract with canonical-checkout/no-commit and sole-review-file ownership overrides. Read home/repository guidance, all three binding documents, their adapter/final-dispatch preparation reviews, complete worker report and review package. The initial package tool output was truncated, so completed the changed-source/test review through bounded reads of all 517 implementation and 979 test lines. No compiler, tests, gate, product mutation, dependency changes, Git mutation or delegated agent was run by this reviewer.

Focused unchanged-code checks addressed two concrete cross-file risks: Rust control/layout/pointer source parity (`src/presentation/ui.rs:150,467,690`) and actual H1 flat-preorder publication (`zig/src/retained_semantics.zig:690`). The latter confirms the repaired Settings ordering, rather than accepting adjacency-only fixtures as sufficient.

## Frozen evidence: Current

Independent hash verification command exited 0. Four current product files, the three final modes' four pre-run source copies, receipt identities and log hashes match final-source JSON. All 150 artifact-manifest entries exist and match byte lengths/SHA-256; no mismatch. The preserved 61-path check is recorded with zero drift in the worker's final preservation receipt; parent separately owns aggregate/current protected verification.

- Binding SHA-256: `12c9b1a3405c960a5dd00d3c1b8b2b47d4f38ebcfc68aab751b8ab753351396d`.
- Worker report: `af0955a3b2b47cd0ad8720e34b8babfc93ae64e09e5395ff43f83cd3a07d78a9`.
- Final-source JSON: `4f4a448db969b0267f89c8a17d90e8784de5bd3c660bbd364ff11a6db268f36c`.
- Full-context diff: `f577c6ec9cf8d3d3f18ea541b45fdaa3c68dd04fe4c2f67b3cc1a9cc6d5806a9`.
- Artifact manifest: `40bb7454de78cdc324f3aa8c800df00402e4fb8536ef52f1a12bd56cfc7a417d`.

Final Debug, ReleaseSafe and ReleaseFast receipts each record actual terminal exit 0, empty source drift and logs ending `All 47 tests passed.` Counts comprise 17 external new H3a tests, one new Header-layout test and inherited imported tests; they are not 47 new H3a tests. Final formatting receipt records exit 0. These are verified archived execution results, not reviewer reruns.

Contemporary admission/publication-adapter/Session/pointer/Settings-preorder RED receipts record exit 1, followed by successful final frozen behavior. The intermediate file named green-settings-geometry actually records exit 1; report identifies the corrected tiny-layout fixture assumption and final fixture evidence supersedes it. Stack flag invocation failure is tooling evidence, not behavioral RED. Intentional qualification diagnostic prints do not indicate compiler warnings or test failures.

## Spec and quality checks

| Requirement | Evidence and assessment | Status |
|---|---|---|
| Stable ownership and complete copying | `zig/src/classic_ui.zig:40,75,92,138,232`: Header is heap allocated once; nullable handle movement preserves addresses. Context, recovery, mutable tray/tooltips and formatted names are copied; global literals supply static identities. Borrowed input expires at deinit. `classic_ui_tests.zig:334` poisons all mutable source bytes, moves the handle and verifies frame survival after Header deinit. | Current / compliant |
| Shared requested-byte accounting and unwind | `classic_ui.zig:75,94,98,103,110,362`: reserve precedes create; OOM cancels; committed Header charge remains frame_scratch; conditional token release and destroy precede saved-charge release. Publish checks stable budget address before H1 work. `classic_ui_tests.zig:575,617,796` observes requests, cancellation/free ordering, both ten-ordinal transitions, retries and exact Header admission. | Current / compliant |
| Atomic publication and action capability | `classic_ui.zig:362,373`: prepared/current revision and actual owner checked, replace succeeds before scalar assignment; actual published view action must exist, be enabled and focusable, then pass closed map. `classic_ui_tests.zig:17,744` covers revision/owner fields, stale HUD after failed Settings, budget mismatch, retain overflow, absent actions and repeated replacement. Trusted scalar, serialization and reference-slot admission are explicit caller contracts. | Current pure seam; host enforcement Pending |
| Source-faithful semantics, topology and geometry | `classic_ui.zig:213,267,308,341,501`: exact control order/labels/tooltips, selected roles, full Alert name/value, settings underlay replacement and source hit/opaque-chrome policy. Settings groups are emitted with descendants in DFS preorder. `classic_ui_tests.zig:127,181,208,266,293,935` independently checks literals/actions, actual publications, all settings choices, geometry/breakpoint/scales/tiny/nonfinite, disabled tooltip/focus and existing Session navigation. | Current / compliant |
| Recovery categories and synchronous adapter | `classic_ui.zig:389,401,452`: full numeric widths and exhaustive closed category mapping; inconsistent record rejected. Local Controls/Tray/hovered names/scratch stay alive through synchronous copy. No Session owner copy, recovery clearing, persistence or command enqueue. `classic_ui_tests.zig:412,466,875` verifies every category, priority/raw details and local-name lifetime with actual Session tick failure. | Current / compliant with accepted category policy |
| Eight limits and target envelope | `classic_ui.zig:36,67,117`; `classic_ui_tests.zig:796`: all fixed policy caps and populated exact/one-less limits, target Header 33232, handle 8, optional revision 16. Requested conservative peak 186448 plus named 300 = 186748; actual Header substitution 182816/183116. `zig/README.md:540` accurately excludes wrapper/descriptors, allocator overhead and transitive stack. Source-bound assembly records local frames only, with transitive stack UNKNOWN. | Current bounded request evidence; complete stack UNKNOWN |
| Narrow integration | `zig/src/root.zig:19,43`; `zig/README.md:521`: one additive export/test import and accurate measured slice documentation. Marker omitted; Session emits no occurrences; explicit ready/preferences_changed input does not invent event ownership. | Current / compliant |

## Findings

### Critical

None. Status: closed; no corrective suggestion required.

### Important

None. Status: closed; no corrective suggestion required.

### Minor

None. Status: closed; no corrective suggestion required. Owning handle copies and nonempty-input unwraps remain the explicitly documented existing Zig move/borrow convention, not an unbound hostile-reference security surface.

## Assessment and residuals

**Task quality: Approved.** The implementation keeps preparation ownership, actual shared-budget charges, immutable semantic snapshots and trusted revision publication separate and transactional. Tests exercise real H1 Settings/HUD publication and actual allocation failure schedules rather than substituting mocks or only checking adjacency.

**Pending / out of scope:** parent aggregate gate; H2 sighted materialization/renderer; native/browser accessibility, IME and visual runtime; actual 3I source revision/events/token-slot registry/help routing; H3c safe-area marker; editor/guide/full tactical/advisory semantics; whole-game acceptance. These boundaries are stated in accepted bindings and README and do not become implementation defects solely because they remain unqualified. No LG or full-game closure follows from this approval.
