# Proposed H3c first-selection marker source binding

2026-10-09. Read-only preparation. H3a actual product API/source rebind remains pending acceptance; no transient classic_ui inspected. Pure marker geometry consumes accepted math/camera/picking value inputs, adds at most one HUD control later. Parent must review the safe-area discrepancy/ruling below before dispatch. Source-only proof, no execution/visual claim.

## Exact Rust behavior and source priority

Marker only for SelectUnionWorld initial tutorial step, not merely onboarding active. Rust iterates scene.worlds in current scene order, filters campaign Union owner by world ID, chooses first projected candidate whose clip components finite, w>0, screen point lies inclusive[0,viewport]. clip=view_projection*position.extend(1); ndc=clip.xyz/w; screen=((ndcx+1)*width/2,(1-ndcy)*height/2). No z-near/far clipping test. Label minx=clamp(pointx-90,8,max(width-188,8)); miny=min(max(pointy-92,190),max(height-180,190)); action bounds180x54. No nearest-distance sort, visibility/collision/safe-area/ray-occlusion check. draw line starts labelmin+(90,54),ring center point radius22,width3,accent[1,.83,.22,1],line width3. [src/ui/start_marker.rs:20-73; src/app/core.rs:499-519]

Rust marker assumes campaign world-ID indexing valid. Zig proposed joins by actual model ID scan rather than indexing without validation; scene/campaign each fixed7 worlds. Duplicate/missing IDs typed InvalidWorldSet, preserving caller frame. Accepted scene source worlds value records include ID/position/radius; campaign worlds own admitted WorldName; no user names retained by borrow. First qualifying scene order remains stable until scene/source changes; no sticky remembered target added. Default factory seed0x4947573120260902, initial owner Union only ID0 ASTER VALE; generator frozen source position(1037,5027), world command plane(-3.963,0,.027) mathematically derived, not measured camera screen location. Do not claim a default camera screen literal without independent matrix evaluation fixture. [zig/src/generator.zig:5-25,80-103; classic_scene.zig:18,31-68; scenario.zig:79-90; src/scenario/mod.rs:56-68]

Selection direct action requires playing and current initial step plus current Union ownership. Existing Session select_guidance_world sets selected/hovered target, clears touch target and observes union_world_selected; then marker disappears because step advances. Skip retains same match and command state, observes completion/persistence path; projection never calls either action. [classic_session.zig:154-156,180-193; src/app/core/session.rs:24-36,172-183]

## Safe-area/selectability discrepancy and Proposed ruling

Source allows marker over opaque HUD or behind a nearer pick sphere. Inventory requires selectable Union target; prior intake explicitly notes source lacks HUD/safe-area filtering. This is a source-versus-target discrepancy, not already parity. Proposed policy: retain source first-qualified iteration, additionally require projected point inside positive-area logical safe Rect, outside all source opaque HUD consumption zones using pre-marker HUD state, AND accepted ray picking at point returns same world ID. Require full180x54 label Rect inside safe Rect and no positive-area intersection with explicit HUD control/tray/help/guidance exclusion Rects. If source label formula fails these requirements, skip that world and continue, no repositioning/smaller label/clamping-to-safe rewrite. At most7 candidates; none returns no marker typed reason. This deliberately narrows source availability; parent engineering ruling pending. [task-3h3-consumer-intake.md, Onboarding and geometry; task-3-app-inventory.md:384-385; src/presentation/ui.rs:175-209]

Safe Rect should bind source legacy GameLayout effective scale clamp(.72,1.50), panels left240*s/right280*s/top64*s/bottom102 compact or56*s wide, collapsed midpoint inverted axes. It is not sufficient alone: inclusive opaque edge consumption excludes exact panel boundary; tray/help and source controls also matter. Consume point using pre-marker source consumes_pointer: nonfinite consumed, Settings entirely consumed, first enabled control OR legacy panel gaps/tray/help. Guidance/tooltip occlusion not source consumes_pointer; Proposed label exclusions include guidance for readable marker and must be named as a new target policy. Source marker ring may extend beyond safe even when point passes: Proposed require ring Rect point±22 full contained safe and outside exclusion rects, same target narrowing; do not promise visual occlusion prevention absent materialization. Tooltip overlay exclusion similarly Proposed if included; final input explicitly supplies typed zones including tooltip, never assume hidden HUD. [src/game/view.rs:38-91; src/presentation/ui.rs:129-209]

## Concrete Proposed API and lifetimes

```zig
pub const MarkerError=error{InvalidWorldSet,InvalidViewport,InvalidBounds,
    ZoneLimit,SourceMismatch,ArithmeticOverflow};
pub const ZoneKind=enum{control,tray,help,guidance,tooltip};
pub const Zone=struct{kind:ZoneKind,bounds:Rect,enabled:bool};
pub const HudPolicy=struct{
    safe:Rect,opaque_edges:struct{left:f32,right:f32,top:f32,bottom:f32},
    settings:bool, zones:[]const Zone,
};
pub const Input=struct{
    step:?onboarding.Step, playing:bool, viewport:math.Vec2,
    camera:camera.CameraMatrices,
    scene_worlds:*const [7]scene.SceneWorld,
    worlds:*const [7]model.World, hud:HudPolicy,
};
pub const Marker=struct{world:model.WorldId,name:model.WorldName,
    point:math.Vec2,label:Rect,line_start:math.Vec2,ring_radius:f32=22};
pub const Absent=enum{not_initial,not_playing,no_selectable_candidate};
pub const Result=union(enum){marker:Marker,absent:Absent};
pub fn project(input:Input) MarkerError!Result;
```

opaque_edges fields are logical pointer thresholds: consumed x<=left or x>=right or y<=top or y>=bottom, with right=viewport.x-280*s and bottom=viewport.y-sourceBottom. Zones include only currently present overlays; control consumes pointer iff enabled, tray/help always consume, guidance/tooltip do not source-consume pointer. Every zone excludes Proposed full label/ring overlap regardless enabled, representing opaque rendered control even disabled. Settings is wholly consumed. Zone.enabled is ignored for noncontrol kinds. Input arrays/zones borrow only synchronous call; output owns fixed WorldName inline copy and geometry, no pointers/token/reference creation/heap. zones<=26 Proposed bound:18 controls+tray+help+guidance+tooltip=22,4 slack; charge if caller copies zones into H3a Header, not marker projector. Root must rebind actual accepted H3a policy source rather than arbitrary callers labelling consumed areas safe. World/scene IDs unique0..6 corresponding, admitted canonical names, finite positive viewport, safe/inverted geometry validated, zones full finite Rect, countchecked before loop. Source nonfinite clip/w<=0/outside viewport candidate skips; finite division/point overflow also skips, no fake finite point. Full invalid camera returns SourceMismatch before selecting rather than silently replacing matrices. camera.matrices includes inverse used by accepted Ray.fromLogicalPointer; do not rebuild camera or substitute alternate projection. Seven PickWorld fixed records built call-local from same scene values, radius validated via accepted picking. Pick tie follows accepted nearest distance then lowerID; this checks selectability, it does not reorder marker candidates. [classic_math.zig:101-122; classic_camera.zig:14-21; classic_picking.zig:5-46]

Pure projection is allocation-free, max7 candidates, max26 zones, no dynamic strings/events. Proposed target Marker<=128 bytes/Input/call temporaries and PickWorld[7] size measured separately, not invented allocator budget. Existing H3a provisional cap72/18 has3-node/one-control marker slack: first actual H3a excludes marker17 controls/64nodes; adding marker raises18/67. Full copied text label START HERE10 twice+tooltip Select your Union world to begin the tutorial44 once=64, plus optional name description24 if mapping chooses it must count complete extra24. IDs c/hud/marker/<0..6>/{primary,label,value},action a/hud/marker/<id> fit reviewed64/48. Name inline24 no new owner. Retained integration borrows Marker only while copying into same shared-budget H3a Header; if new fields exceed actual accepted Header cap reject review, no cap growth. Token owner/revision remains outer HUD owner, no new token for marker. validate marker action requires Current exact resident revision/context, present frame action/enabled, playing/initial source and Union ownership before accepted Session dispatch. Pure marker does not enforce actual host event revision advancement.

## Independent future fixtures

Identity view_projection, viewport1440x900,position(0,0,0) gives point(720,450),source label(630,358),line_start(720,412),ring bbox[698,428]..[742,472]; full label[630,358]..[810,412]. Safe legacy[240,64]..[1160,844]; no zones admitted, accepted pick fixture must independently ensure same target.960x600 identity center gives point480,300,label390,208,edge480,262,safe[172.8,46.08]..[758.4,498]. Tiny120x100 label sourcex8/y190 outside viewport and safe, Proposed absence; do not force smaller action. Source point exact edges inclusive, stricter opaque boundaries consumed. NaN clip/Inf,negative/zero w skipped,positive tinyw finiteoverflow skipped,valid z outside0..1 source projection not automatically rejected (pick may fail). Earlier candidate blocked by HUD/nearer sphere should skip to later qualifying Union in unchanged scene order; no nearest marker selection. Noninitial five steps/completed/Settings absent, worldID/ownership mismatch rejects and no source mutation.

Synthetic independent camera columns produce known clip, not product-generated expected points. Fixed default factory world coordinate/name/owner fixture separately verifies frozen source without importing generator to calculate expected. Test full label/ring containment, touching rect nonoverlap versus opaque inclusive boundary policy, all zone types,26/27 caps, safe collapse/no candidate, resize/scale85/100/115/130 and exact current-source selection hides marker. Snapshot poisoning proves fixed output owns name; retained OOM failure preserves old frame/stamp; no projector OOM allocation ordinal exists, report zero allocations honestly. Actual line/ring/text glyph clipping/color/accessibility cue rendering native/browser remains Task6, no font/shaping/new assets.

## Observed source freeze

- src/ui/start_marker.rs: `db5a7611b7acce6e873a76a219ac8a49c2532e86864ffc1041373f4fa90c5923`
- src/presentation/ui.rs: `b38b0b4f0c78c5de6e807116239018351d55538e2c5f2faba355828744517ae6`
- src/game/view.rs: `6b5f1180edaa8e8afd84d25d85a7cdaf7c9ae2d139bbd8b4b21fd4e2dc98fc35`
- src/app/core.rs: `2ac51de5a8110c76c96c6665483812ea7869d9a33f05a4731d2abeb7c910befa`
- src/app/core/session.rs: `19373aa262c0933073e44ce3fda97e2ed25fd0d6ab8470e1aa450aa4bdec3de1`
- src/scenario/mod.rs: `66aa18d9938b53bbfb6ddb23067a164b686f380ec6a6d9d282d548c4fbf6cc85`
- zig/src/classic_math.zig: `e0b5188b08ab78a4c32a348ba0398150ab2b8ac0541a8fe4984b9e4d60913186`
- zig/src/classic_camera.zig: `ac5931069c0f65c44eaad0e093dc668ef61756a5ff19cdf5f69b02dfcbd0c268`
- zig/src/classic_picking.zig: `e9dc79171f9f3906549642df0d53c3076a52b4427eb3a9ba375aab2541b0c219`
- zig/src/classic_scene.zig: `e022c8961168b072f36cd26d0a84f9cb88bd17d904689df9d2614982c6f4ce11`
- zig/src/classic_app.zig: `2d418ba49cdfb85d2ffd8d172cefc563b1b4cf87bb601729eb4045f791e2f1be`
- zig/src/classic_session.zig: `747b27b98b15030931f214507568a9ced462121f47591b59db674c9383fd9568`
- zig/src/generator.zig: `96ed3c2490d3e2b8c69c9b7921fda0a5d6e962a8679773bc63cb971b146daf39`
- zig/src/model.zig: `86db58f389d4b0b1e415a363b7e32c8694837f7bd23d4b3b9635cee66daed20e`

Gaps: parent safe/ring/label/zone selectability ruling; actual accepted H3a API/owner/policy/header rebind; accepted latest3G3 source manifests; actual target fixed sizes and independent fixtures; later host revision/dispatch and materialization. No product/test/gate success or whole-game acceptance.
