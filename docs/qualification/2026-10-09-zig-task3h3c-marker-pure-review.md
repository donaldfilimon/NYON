# H3c pure marker independent full implementation review

Status: CHANGES REQUIRED. One Important correctness finding and one explicit execution-order evidence gap. No other code-quality findings identified. This review does not execute compiler/tests/aggregate gate or grant host/integration acceptance.

## Frozen identity and audit

Read complete execution brief, pure dispatch binding and its review, canonical geometry binding/review, full 9,186-byte production module, full 20,922-byte tests, root/README accepted-baseline differences, worker report and actual accepted math/model/generator/camera/scene/picking/geometry context. Independently recomputed four live final-source identities, all 67 protected identities, and all 1,490 artifact byte lengths/hashes: zero mismatches. Verified captured assembly prologue excerpts appear exactly in the frozen assembly.

Final-source JSON SHA256 `ac760eaf7727b343b89b96390fa868845aadbbab11ce1d2b06f079ecec9f9b1d`.
Module SHA256 `e9cf0fa1c340e1ce27648ac94fa3b89bb662e1f5ad183563671139b1efff97c4`.
Tests SHA256 `7d3be04be8b712f0ca697e5a28a74fc7c82d27fc990530f1c44a3646fe9cd0a5`.
Context patch SHA256 `bd9180f137894c2b975ff83fb222f42383e632d8dca97d30a9631237960b2654`.
Manifest SHA256 `2850dbcf4acdec422658e0d424129f7385873218a1348932568630889624f72f`.
Report SHA256 `9d1fe1bab201ad0c435fe1f71030ecc94d260a25dc3a00b9e3323bd622a0bcc8`.

Captured frozen Debug/ReleaseSafe/ReleaseFast receipts each report exit 0 and 16 passing tests; frozen fmt check reports exit 0. All share captured source manifest `d1ba94586da6ef5e8923fc5520f392f47965c2eb1dbd0650ece4feb3e7c113b6`. This is inspected worker-run evidence, not rerun execution by this reviewer.

## Important I1 — valid collapsed safe axis can yield overflow instead of absence

`zig/src/classic_start_marker.zig:90` deliberately admits finite collapsed safe axes, including coordinates beyond ordinary small HUD geometry, as the binding requires. At `:95`–`:97`, emptiness is inferred only after inset and extent arithmetic. At sufficiently large finite f32 coordinates, those additions/subtractions lose all their small increments, making a zero-width axis look like a nonempty placement interval.

Concrete valid numeric seam: ordinary initialized identity-camera/Union fixture, viewport.x=f32(1e30), viewport.y=900, safe.min=(viewport.x,0), safe.max=(viewport.x,900), no zones and zero edges. All admission checks succeed. For x=f32(1e30)=1.0000000150474662e30, strict f32 x+1, x-1, x-180 and x+180 each equal x. Thus inset.min.x=max_min.x=x and the line 97 test is false. Candidate point.x is finite inside the viewport; label x clamps to x. Its collapsed endpoints satisfy inset and viewport containment at `:115`, then `validateMarkerGeometry` rejects lost positive extent with ArithmeticOverflow at `:35`/`:135`.

The authoritative pure binding explicitly admits collapsed safe axes and requires too-small/collapsed safe to produce no_label_candidate. This input has no possible positive 180-wide label; its accepted zero width must not become a geometry error through rounding. The existing collapsed-safe fixture at coordinates 2000 does not cover lost inset arithmetic. This is source reasoning plus independent Python IEEE754 f32 characterization, not a Zig reproduction claim.

Required correction: after complete input validation and preserved not_initial/not_playing priority, recognize an actual collapsed safe axis explicitly and return no_label_candidate before inset arithmetic. Add a contemporary failing initialized fixture for the large collapsed case before the product fix; preserve malformed-input rejection, inactive priority, normal tiny absence, and genuine noncollapsed lost-label-extent ArithmeticOverflow. No unrelated numeric policy or camera change is required.

## Execution evidence gap P1 — required behavioral RED ordering missing

`task-3h3c-marker-pure-execution-brief.md` requires contemporary runnable behavioral RED before GREEN for meaningful fixture families. Worker report/proof explicitly states product implementation preceded the first runnable fixtures. The initial exit 1 is a compile/API diagnostic, and the misleadingly named before-red receipt is exit 0; neither establishes behavioral RED. Current comprehensive GREEN fixtures do not retroactively satisfy that ordering. This is a disclosed process/spec evidence gap requiring parent disposition, not proof of another production bug. Do not mutate otherwise correct production code to manufacture historic RED. A genuine I1 regression RED/fix/GREEN can establish that correction's ordering only.

## Remaining spec and quality review

Unique seven valid IDs imply complete 0..6 sets without an additional mask equality; lookup matches ownership by actual ID and selection uses actual scene source order. Range checks precede indexing/narrowing. All used names and scene geometry are validated even for inactive flags. Element pointer loops avoid reading undefined unused scene lanes and name tails. Validated inline names survive input reuse; no output/source slice or stack pointer escapes. Read/read aliasing is safe under immutable caller input lifetime.

Selection, inclusive projected viewport, exact positive clip w, no depth clipping, safe label clamp, rounded containment, closed enabled/tray/help touches and positive disabled/guidance/tooltip intersections match Choice B. First admitted action-only candidate remains first. Ring/pick failures hide both optional fields; normalized accepted Ray constructor and same seven scene spheres supply pick eligibility. Finite arbitrary matrices remain a numeric seam, not camera provenance proof. No Core/Session/HUD adapter is invented.

Shared geometry helper uses strict f32 canonical endpoint additions and exact connector/radius/nullability checks. Fractional hand literals verify subtracted span mismatch without weakening endpoint equality. Actual malformed endpoint, option matrix, finite lost extent and nonfinite errors are separated. No uninitialized backing comparison or invalid pointer fabrication was found in tests.

Default twelve fixture expectations are literal/source analytical, independent of projector output: all seven generator positions, world0 energy/defense, scene position/radius, 0.02px point/label expectations, exact ID/name/mode/options and explicit same-world pick. Independent HUD formulas match accepted main controls, field choices/adjustment flags, advisory, skip, tray/guidance and stored versus legacy scale. No prediction was regenerated from production output. Coverage is substantial but missed I1; captured green counts do not cover that omitted numeric edge.

No allocator/token/event API, hidden dynamic collection, callback or persistent workspace exists. Named target layouts and the <=128 Marker assertion are honest scoped evidence. Direct Debug prologues total project 32+4096+416=4544 bytes and helper432; their captured source/assembly identities match. They do not establish optimized/transitive/other-target/process stack. Root changes are only module export/test import; README distinguishes pure numeric behavior from pending snapshots/attachment/capability/host routing.

Only this new ignored review file was written. Parent aggregate gate, fix qualification, accepted pure rebind, retained integration, host materialization and whole-game acceptance remain pending.
