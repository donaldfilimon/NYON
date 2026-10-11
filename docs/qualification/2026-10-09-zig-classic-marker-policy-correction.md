# Proposed H3c marker policy correction and availability analysis

2026-10-09. Additive document; original source binding/report/review immutable. Addresses review871c5327d08997c61a150c461fdf9a17d312170204d8a2e5a4ae68030c7fe94d I1/I2. Parent has not yet approved revised policy. Source-only double-precision analytic evaluation, no compiler/f32/runtime/visual qualification. No transient H3a product accepted/read as dependency.

## Concrete default-camera conflict

Default sole Union is ASTER VALE ID0, position(1037,5027)→(-3.963,0,.027), camera target0/yaw0/pitch55/distance15/FOV45, viewport aspect clamp. With z=.027, depth=15-z*cos55; screen x=w/2+(-3.963)*cot22.5*h/(2*depth), y=h/2+z*sin55*cot22.5*h/(2*depth). Independent Python scalar arithmetic evaluated actual source formulas; it does not run product code or matrix builder. Safe s=max(.72,HUDscale). [classic_camera.zig:109-120; classic_math.zig:188-205; generator.zig:80-103]

| Viewport | Scale% | Projected point x/y | Safe left/right | Source label x/y | Nearest full-label clamp x/y | Strict point/ring |
|---|---:|---|---|---|---|---|
|723x802|85|105.464/402.429|172.800/521.400|15.464/310.429|172.800/310.429|fails both|
|723x802|100|105.464/402.429|172.800/521.400|15.464/310.429|172.800/310.429|fails both|
|723x802|115|105.464/402.429|198.720/491.160|15.464/310.429|198.720/310.429|fails both|
|723x802|130|105.464/402.429|224.640/460.920|15.464/310.429|224.640/310.429|fails both|
|960x600|85|288.452/301.069|172.800/758.400|198.452/209.069|198.452/209.069|passes|
|960x600|100|288.452/301.069|172.800/758.400|198.452/209.069|198.452/209.069|passes|
|960x600|115|288.452/301.069|198.720/728.160|198.452/209.069|198.720/209.069|passes|
|960x600|130|288.452/301.069|224.640/697.920|198.452/209.069|224.640/209.069|passes|
|1440x900|85|432.678/451.604|204.000/1202.000|342.678/359.604|342.678/359.604|passes|
|1440x900|100|432.678/451.604|240.000/1160.000|342.678/359.604|342.678/359.604|passes|
|1440x900|115|432.678/451.604|276.000/1118.000|342.678/359.604|342.678/359.604|passes|
|1440x900|130|432.678/451.604|312.000/1076.000|342.678/359.604|342.678/359.604|passes|

Thus nearest full-label clamp fixes original115/130 compact label failure, but retaining strict source point/ring/selectable-scene requirement still removes sole target at REQUIRED723x802 for all four scales. Camera focus/pan would change camera/session policy and cannot be silently added to pure marker. This concrete conflict requires parent ruling; no claim strict policy meets all admitted tutorial windows. Source button directly SelectGuidanceWorld validates initial-step/currentUnion and does not require a scene ray, so smallest state-preserving alternative is a safe typed action-only cue when actual projected point/ring are hidden. [classic_session.zig:180-193; src/app/core/session.rs:24-36]

## Revised options and exact guarantee

Proposed primary rule: keep source scene order/initial-step/Union/finite clip/w>0/inviewport point candidate. Label180x54 nearest-clamps source label min into safe Rect inset1 logical pixel on all edges. Inset is explicit new guard because source opaque consumption includes exact panel boundaries; touching left/right safe boundary cannot be called fully pointer-selectable. Formula xmin=safe.minx+1,xmax=safe.maxx-1-180; analogous y with54. If xmin>xmax or ymin>ymax, no label. clamp component-wise deterministic, then reject any positive-area intersection with source pre-marker control/tray/help/guidance/tooltip zones. No retries/search for alternate nearby label location; candidate skips to nextUnion. Recompute connector source endpoint labelmin+(90,54). At723 table clamp final x becomes173.8/199.72/225.64, same y310.429;960 final115x199.72/130x225.64; remaining values unaffected. No control/count/text/cap growth.

Parent choice A strict scene cue: point/ring inside strict safe and same accepted ray pick plus safe clamped label; connector may remain materialization-clipped.723 expected absent allscales, explicitly accepted availability loss if chosen. Choice B recommended Proposed smallest presentation-only alternative: if clamped label clear/admitted but point/ring unsafe or ray occluded, emit `action_only` marker with same typed world/name/label, hide ring AND connector, source label remains selectable via accepted direct action; if point/ring safe and pick sameworld emit `scene_linked` original ring+recomputedconnector. Preserve point in numeric diagnostic output, not dispatch scene tap there. Tooltip/action text must explicitly say Select your Union world to begin the tutorial (same source text), no claim world itself visible/selectable by ray in action_only. No camera movement or new session transaction, no replacement simulation. Parent B ruling pending.

Default factory/nohelp/pre-marker source geometry analysis: clamped label clears tray/guidance at these12 configurations, field/adjust/advisory controls occupy right-side panels, bottom controls stay below label. Analytical expected B result action_only at723 allscales, scene_linked at960/1440 provided accepted seven-world ray picks same target and ring-zone test passes. This is conditional prediction, not proven f32 ray/materialization execution. Full ray default fixture must be independently literalized before availability acceptance; source point and zone geometry margins support targeted testing. Never promise availability for all viewport/camera/zone states: too-small safe area, blocking overlays, outside-viewport world, noUnion or noninitial step can yield none. One-pixel inset with exact finite checked rect arithmetic must pass no-zone point/label hit; touching control zone geometric boundary has no positive-area overlap but source inclusive hit boundary precedence may consume event, so label safe activation additionally requires not sharing enabled-control/tray/help boundaries along its area. Concrete future policy must enforce closed-boundary exclusion for consuming zones (use overlaps-or-touches), positive-area exclusion suffices only nonconsuming guidance/tooltip; not fabricate hit safety from strict intersection alone.

## Coherent snapshot trust contract

Pure `project(Input)` validates finite camera components, positive viewport, valid IDs/names/radii/Rect ranges and bounded zones; cannot prove arbitrary matrices mutually inverse, scene derived from campaign or caller zones match HUD. No tolerance-based inverse/coherence heuristic added. Replace ambiguous pure SourceMismatch: `InvalidCamera` means any required matrix component nonfinite; `InvalidWorldSet` means duplicate/missing IDs/ranges; `InvalidBounds` invalid policy zones/Rect; finite matrix disagreement is accepted numeric input in synthetic fixtures and may cause absent, not diagnosed provenance fraud. Depth z no new policy; finite clip w/x/y tested as source.

Production adapter Proposed `projectFromCore(core:*const AppCore,step:?Step,policy:BorrowedHudSnapshot,current:CurrentStamp) AdapterError!Result`: synchronous single UI-thread call obtains core.viewport(),core.camera(),core.campaign() values and core.sceneFrame() from SAME unchanged Core/source revision, uses scene.camera current matrices; HUD snapshot carries copied revision/resident owner/full context from pre-marker accepted HUD preparation. Before and after extraction require Current revision/owner matches trusted caller state and HUD snapshot viewport equals core viewport; changed identity returns `SourceMismatch` at adapter seam only. No pointer into local SceneFrame/WorldName escapes. Caller prevents core mutation for full call; pure function cannot enforce that trust from numbers. Camera viewport must be obtained from actual CameraState.logical_viewport and checked with core.viewport at adapter; source APIs core.camera():CameraState and sceneFrame() exist. Caller-source provenance root3G3 manifest rebind mandatory; do not use arbitrary fabricated `Input` in production path. Synthetic Input remains explicit supported pure test seam; it carries no production acceptance/provenance assertion. No existing CurrentStamp/HUDSnapshot API invented as callable; bind acceptedH3a after product acceptance.

If viewport normalized differences exist (Core camera min1 while raw viewport maybe0), adapter uses accepted HUD/core normalization policy and documents exact admitted supported viewport; reject mismatch instead of swapping arbitrary matrix. This is structural/provenance contract, not numerical proof inverse(view_projection). Full camera matrix finite validation necessary, not sufficient for coherent source; trusted extraction establishes coherence. `SourceMismatch` is exact trusted snapshot identity/viewport mismatch or revision changed, never claimed detector of all finite bad matrices.

## Explicit synthetic seven-sphere fixtures

Identity camera/inverse,viewport1440x900: sceneID0 position(0,0,0),radius.1,ownerUnion,name ASTER VALE. IDs1..6 positions respectively(10,0,0),(20,0,0),(30,0,0),(40,0,0),(50,0,0),(60,0,0),all radius.1/nonUnion, admitted names B,C,D,E,F,G. Center ray near(0,0,0),far(0,0,1),positive far sphere intersection.1 selects0; point720,450,label630,358,ring22 safe. Source scene order input explicit0..6. One occlusion fixture ID1 position(0,0,.2),radius.05 nonUnion would NOT occlude ID0 near/far .1; use ID0 at(0,0,.8),r.1,ID1 at(0,0,.3),r.1 with same projected center, then pickID1 at.2 vsID0 .7; expected B action_only/strictA no0, never reorder marker bypick. Subsequent candidate fixture ownerUnion ID2 at(.4,0,.8),r.1 and move ID1 off its ray; label/policy independence determines later candidate underA, source scene order preserved. Independent exact ray-distance/matrix tables in future tests, not production-computed expected values. Nonfinite/generic finite mismatch,revision/viewport mismatch adapter reject,disabledControl boundary rejection,inset1/tiny180x54 fit boundaries,12 default factory configurations all require future f32 literal tests.

## Frozen observations and limits

- zig/src/classic_camera.zig: `ac5931069c0f65c44eaad0e093dc668ef61756a5ff19cdf5f69b02dfcbd0c268`
- zig/src/classic_math.zig: `e0b5188b08ab78a4c32a348ba0398150ab2b8ac0541a8fe4984b9e4d60913186`
- zig/src/classic_picking.zig: `e9dc79171f9f3906549642df0d53c3076a52b4427eb3a9ba375aab2541b0c219`
- zig/src/classic_scene.zig: `e022c8961168b072f36cd26d0a84f9cb88bd17d904689df9d2614982c6f4ce11`
- zig/src/classic_app.zig: `2d418ba49cdfb85d2ffd8d172cefc563b1b4cf87bb601729eb4045f791e2f1be`
- zig/src/classic_session.zig: `747b27b98b15030931f214507568a9ced462121f47591b59db674c9383fd9568`
- zig/src/generator.zig: `96ed3c2490d3e2b8c69c9b7921fda0a5d6e962a8679773bc63cb971b146daf39`
- src/ui/start_marker.rs: `db5a7611b7acce6e873a76a219ac8a49c2532e86864ffc1041373f4fa90c5923`
- src/presentation/ui.rs: `b38b0b4f0c78c5de6e807116239018351d55538e2c5f2faba355828744517ae6`

Projector still no heap/token/event allocation, at most7 candidates/26zones/one marker. New enum scene_linked/action_only numeric storage requires target Marker<=128 assert, no cap growth or Header changes without actualH3a rebind. Parent A/B/inset/consuming-zone-boundary ruling pending, coherent adapter/source manifests/acceptedH3a API remain pending. No all-window/tutorial/host/rendering/game-completion claim.
