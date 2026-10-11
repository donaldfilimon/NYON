# Task3G2 independent frozen spec and quality review

2026-10-09. Current scoped correctness: PASS; aggregate acceptance remains Pending parent gate. Reviewed the complete four-path fresh-baseline full-context.patch (new scene files absent at baseline, root/README additive changes), brief, report, identities and supplied focused evidence. No builds/tests rerun; no product changes. Only this review file is owned by this reviewer.

## Spec review

Critical: none. Important current implementation defects: none found.

The bounded insertion scans every input fleet, retains ascending IDs with original input order for ties, safely shifts only initialized indexes, and evicts the highest retained entry. Endpoint filtering occurs after the 512 selection without backfill. Numeric array endpoint indexing is preserved even when stored world IDs differ. Route deduplication preserves selected valid fleet encounter order and first control, with the capacity check before deduplication. Seven endpoints and three factions imply 147 possible keys; preserving the 256 cap without claiming saturation coverage is correct.

SceneCampaign field names/types bind actual CampaignView without importing AppCore, avoiding a future dependency cycle. Snapshot binding uses the actual F3 type and priority-indexed score; no new freshness/score guard appears. Current G1 matrices and current yaw/pitch are used. Scene output owns initialized values, retains no campaign/scratch pointers, and exposes only const populated prefixes from a stable live frame. Undefined tails are intentionally excluded from comparisons.

Against scene.rs and fleet_progress_ratio, reviewed seed constants/wrapping, integer sector clamps, radius association, normalization/no extra field clamp, owner branches, hazard propagation, f64 ratio before f32 conversion, alpha/display progress, strength, exact staged curve association, quality/motion/contrast choices, reduced bloom, time sanitization and advisory copying. No semantic divergence found. Floating helpers explicitly use strict mode; clamp preserves source signed-zero comparison behavior. Cross-target transcendental bit identity remains unqualified.

## Quality and process review

Critical: none. Important process limitation: strict comprehensive test-first provenance is PARTIAL. The recorded compiled sector assertion fails with expected -5/found 0 and then passes. The expanded behavioral suite followed complete implementation. Compile/setup repairs are not assertion RED, and the final green suite does not retroactively establish test-first development. The report states this honestly. Parent must explicitly assess this deviation separately from present correctness; no retrospective RED or repeated implementation should be manufactured.

Minor: the signed-zero test supplies -0 alpha but checks ordinary progress, so it does not discriminate sign preservation itself. Current clamp code follows the source comparison semantics; this is a narrow coverage limitation, not a demonstrated product defect. Some route-control assertions call the production helper, but the separate analytic start/end/control/midpoint fixture independently qualifies its arithmetic. The ratio fixture uses independently supplied rational operands and proves that premature f32 conversion differs; the seed reference uses widened modular arithmetic rather than producer-generated goldens.

Coverage inspected includes reverse 520-fleet saturation, invalid-before-truncation/no-backfill, stable ties at capacity, unique-ID permutations, encounter routes/dedupe, all 147 keys, indexed endpoints, world fields/colors/hazards, progress edge cases, analytic curves, preference grid, current matrix copying, arbitrary advisory scores, seed reference, immutable inputs and retained frame copies after input backing release. No allocation/retained-owner dependency is introduced. O(N*512) performance is explicitly unmeasured.

## Evidence and disposition

Read-only SHA verification: all 21 handoff identities, 14 input identities and 51 protected identities matched; zero protected changes. All three focused receipts bind the same live four source identities and corresponding logs. Debug/ReleaseSafe/ReleaseFast each exit 0 with 15 tests: 9 scene cases plus 6 imported model/generator tests. Fmt receipt exits 0 with empty log. Compiler recorded as 0.18.0-dev.120+9fe22a29b with isolated mode caches. These are supplied execution receipts independently inspected, not reviewer reruns.

Recommend scoped acceptance after the parent aggregate gate and explicit acknowledgment of PARTIAL TDD provenance. This review does not establish AppCore/Session integration, digest/queue lifecycle isolation, render/upload ABI, layout/HUD/picking routing, GPU/native/browser/DPR/manual/accessibility, performance, or full-game completion. Those remain later owners. No Critical or Important current correctness issue requires an implementation revision from this review.
