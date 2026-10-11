# Proposed H3c concrete HUD/marker integration dispatch

2026-10-09. Additive preparation only. Earlier source, Choice B, immutable lease, accepted HUD rebind, attachment,113-byte text and canonical endpoint contracts remain authoritative. The active pure worker's classic_start_marker files, root and README were neither read nor adopted. All marker names below bind the reviewed proposed API until its separate READY/review/gate rebind. This package grants no product authority.

## Modules and exact callable surfaces

Future ownership: designated UI worker owns only classic_ui.zig/accessor/flags/attachment/action mapping and corresponding existing UI tests; NEW classic_start_marker_adapter.zig and classic_start_marker_adapter_tests.zig own snapshot/Core adaptation. Minimal root/README exports follow sequential parent dispatch. No Core/Session/math/camera/scene/picking edits. Dependencies: UI imports pure marker; adapter imports UI+pure+Core; pure imports neither UI nor adapter. No cycle or private Header export. [task-3h3c-marker-hud-rebind.md, § Acyclic future module split and callable seams]

```zig
// Future additive classic_ui surface
pub const GeometryView = struct {
    revision:u64, viewport:Vec2, scale:f32,
    settings:bool, help_visible:bool,
};
// Method on PreparedProjection; metadata value, no Header pointer/slice.
pub fn geometry(self:*const PreparedProjection)
    error{EmptyProjection}!GeometryView;
pub const AttachError = error{
    EmptyProjection, WrongProjectionMode, AlreadyPublished, AlreadyAttached,
    InvalidMarker, InvalidWorldName, NonFinite, Overflow, InputLimit, InvalidSource,
};
pub fn attachStartMarker(prepared:*PreparedProjection,
    result:marker.Result) AttachError!void;
// Existing signatures remain; implementation mutates private published flag.
pub fn publishHudSettings(slot:*retained.FrameSlot, published_revision:*?u64,
    prepared:*const PreparedProjection,current:CurrentStamp) PublishError!void;
pub fn validateAction(view:retained.FrameView,published_revision:u64,
    current:CurrentStamp,id:retained.ActionId) ActionError!TypedAction;
```

Geometry reads existing revision/viewport/scale/settings/help_visible; empty handle rejects before input(), whose actual current implementation unwraps Header. No new metadata fields for this getter. Geometry may be reacquired after attach; only snapshot construction below requires an unmodified pre-marker tree. [zig/src/classic_ui.zig:41; zig/src/classic_ui.zig:84; zig/src/classic_ui.zig:359]

```zig
// NEW adapter exports; aliases use reviewed pure definitions, not replicas.
pub const CurrentLease = struct {
    core:*const app.AppCore, step:?onboarding.Step, stamp:ui.CurrentStamp,
};
pub const BorrowedHudSnapshot = struct {
    revision:u64,resident:retained.TokenRef,hold:?u64,
    context:[128]u8,context_len:usize,
    viewport:math.Vec2,safe:geometry.Rect,edges:marker.OpaqueEdges,
    settings:bool,zones:[26]marker.Zone,zone_count:usize,
};
pub const SnapshotError = error{
    EmptyProjection,InvalidHudGeometry,ZoneLimit,SourceMismatch,
};
pub const AdapterError = marker.InputError || error{SourceMismatch};
pub fn snapshotFromPrepared(prepared:*const ui.PreparedProjection,
    current:ui.CurrentStamp,out:*BorrowedHudSnapshot) SnapshotError!void;
pub fn projectFromCore(lease:*const CurrentLease,
    hud:*const BorrowedHudSnapshot) AdapterError!marker.Result;
```

Snapshot constructor scalar-validates all metadata, counters, schema and zone/geometry arithmetic before writing out. On error out is unchanged; infallible second-pass fill avoids a second snapshot temporary. Context and zones are copied fixed values; resident is a borrowed TokenRef with zero retains. Snapshot can neither outlive nor license mutation/deinit of its prepared owner. Exact initialized context prefix only is read; unused tails need not be initialized or compared. Source out must not alias prepared storage/current context or any immutable source input; public caller-owned struct and private Header make ordinary independent local storage the supported construction. Snapshot target size and constructor locals must be reported, not called free. [task-3h3c-marker-adapter-dispatch-binding.md, § Concrete Proposed lease, snapshot and API]

Current must match actual prepared.input().owner and captured geometry revision, with draft null, full UTF8 context exact and length<=128. Do not normalize context. Reject any preexisting canonical marker action/node triplet as SourceMismatch: construction is pre-marker only. Validate root kind/name/schema, action-node bounds and region uniqueness before indexing/copying. Settings schema yields settings=true and no marker, but still validates all supplied metadata; it is not a bypass for malformed input. Unknown unexpected action/region schema rejects InvalidHudGeometry rather than silently guessing a zone. No invented runtime borrow checker or prepared published flag getter is required: immutable pre-marker construction is caller precondition plus actual tree checks.

## Exact snapshot zones and source policy

Scan actual input nodes in flat source order, take each primary action node once, never its children. For Playing admit the accepted17 HUD static action IDs only and bounds, retaining enabled state. Settings admits11 accepted settings primary actions. Append actual c/hud/tray region, c/guidance region and c/tooltip text bounds when present, using marker kinds tray/guidance/tooltip. Help panel is absent from semantic regions: when GeometryView.help_visible append the source consumesPointer rectangle (v.x/2-190,v.y/2-90,380,180), kind help. Bounds are checked with accepted Rect before output writes; no independent recomputation of controls/tray/guidance/tooltip. Maximum Playing17+4=21 zones<=26; settings11 and no Playing regions/help. Unknown duplicates/missing required Playing tray/invalid bounds reject. [zig/src/classic_ui.zig:269; zig/src/classic_ui.zig:300; zig/src/classic_ui.zig:301; zig/src/classic_ui.zig:346; zig/src/classic_ui.zig:350; zig/src/classic_ui.zig:512]

Legacy s=max(stored scale,.72); edges left240s,right280s,top64s,bottom102 if viewport.x<1100 else56s. Safe min(left,top),max(v.x-right,v.y-bottom); inverted axes collapse at checked strictf32 min+(max-min)/2. Require finite arithmetic, ordered resulting Rect. Extreme finite overflow becomes InvalidHudGeometry; zero-area safe is allowed and later produces no_label_candidate. Store actual normalized HUD viewport and actual scale-derived zones, not mutable prefs. Enabled control/tray/help exclusion is closed overlap-or-touch; disabled control/guidance/tooltip is positive-area overlap, with no .01 threshold. These are accepted Choice B policy additions, not fabricated source parity. [zig/src/classic_ui.zig:100; zig/src/classic_ui.zig:506; task-3h3c-marker-adapter-dispatch-binding.md, § Final Choice B geometric/selection policy]

## Actual Core extraction and coherent caller lease

CurrentLease is a synchronous caller promise excluding Core/Session/onboarding/owner/HUD mutation, replacement, moves and deinit until return. Caller associates step and full stamp with the same source epoch; no Core revision/token/context getter exists or is proposed. Adapter cannot detect lease violation. Compare hud revision/resident.sameIdentity/hold/full context against stamp and reject draftnonnull or malformed lengths/counts before slice access. This SourceMismatch covers captured HUD/current and viewport disagreement only, not arbitrary matrix provenance. [task-3h3c-marker-adapter-dispatch-review.md, § Residual I2 disposition]

Actual calls: raw=core.viewport(); camera_state=core.camera(); campaign=core.campaign(); scene_frame=core.sceneFrame(); playing=core.mode()==.playing. All local values live through pure project. CampaignView.worlds is owned[7]World (fleet slice is not exposed by this adapter); SceneFrame.worlds/camera are owned values. Reject nonfinite raw viewport InvalidViewport; normalized=max(raw,1) per component. Require camera_state.logical_viewport==normalized and hud.viewport==normalized else SourceMismatch. Use scene_frame.camera, never substitute camera_state matrices; supply pointers to local scene_frame.worlds and campaign.worlds only during synchronous marker.project. Return inline Result only. [zig/src/classic_app.zig:84; zig/src/classic_app.zig:141; zig/src/classic_app.zig:154; zig/src/classic_app.zig:173; zig/src/classic_app.zig:228; zig/src/classic_app.zig:241]

The exact pure call Input is {step=lease.step,playing,viewport=normalized,camera=scene_frame.camera,scene_worlds=&scene_frame.worlds,worlds=&campaign.worlds,safe=hud.safe,edges=hud.edges,settings=hud.settings,zones=hud.zones[0..hud.zone_count]}. Pure validates7 ID sets/names/finite camera/radii/geometry as reviewed; numeric checks do not prove inverse consistency or scene authority. No Session fields, locks, callbacks, host queue or current reread is added. [task-3h3c-marker-pure-dispatch-binding.md, § Concrete proposed data-only API]

## Same Header attach transaction and capability mapping

Header allocated from start includes initialized marker_attached=false,published=false and optional inline marker visual,<=160 bytes including tag/padding. Whole actual Header<=36864 mandatory (accepted prior33232); Marker<=128. No second Header/snapshot/map allocation, arena expansion, token retain or attach budget charge. Preserve source recovery/announcement fields and17 existing action mappings. Header's one actual allocation remains charged .frame_scratch, reserve BEFORE allocator.create, cancel OOM, commit exact request, destroy before charge.release; stable shared budget outlives frames/Header. [zig/src/classic_ui.zig:92; task-3h3c-marker-attach-dispatch-binding.md, § Exact future UI contract]

State checks: nonempty Playing, not published, not attached. Successful absent marks attached once but adds nothing. All failures leave entire initialized Header, counters, token references and budget unchanged. Any successful publish sets published=true ONLY after slot.replace and trusted revision assignment; failed publish leaves flag/scalar/frame unchanged. Repeated publication is allowed; attach after any successful publication remains forbidden even if slot later emptied. Existing const PreparedProjection parameter can mutate heap Header flag without changing its handle. [task-3h3c-marker-attach-dispatch-binding.md, § Exact future UI contract]

For worldN0..6 exact immutable action a/hud/marker/N maps to typed .select_guidance_world=N after existing owner/revision/actual enabled+focusable published node capability checks. No event WorldId or node value parser. Nodes c/hud/marker/N/{primary,label,value}: Button START HERE/complete45-byte tooltip/valuecanonicalName, followed Text START HERE and Text canonicalName, children nonfocusable/noaction/samebounds; primary enabled/focusable,selectedfalse. Header unique name<=24 copied once, reused twice; H1 five copies10+45+n+10+n<=113, IDs actual90<=128. Names validate len1..24 BEFORE asSlice then WorldName.init. [task-3h3c-marker-attach-text-fix-review.md]

Attach calls shared pure validateMarkerGeometry: strict canonical max=min+(180,54), positive representable extents, no subtractionwidth test/epsilon/minrounding; scene_linked connector=min+(90,54),radius22,bothpresent; action_onlybothnull. Map NonFinite->NonFinite,ArithmeticOverflow->Overflow,InvalidMarkerGeometry->InvalidMarker. Safe/zone/pick provenance belongs projector/lease, not attach. [task-3h3c-marker-canonical-geometry-binding.md, § Shared public pure validation contract]

Complete nonmutating preflight: current arrays/counts/arena limits, exact root/toolbar uniqueness, contiguous toolbar subtree, parent and adjacency reciprocity/flatDFS, no duplicate marker identity, all focus references and changed indices; name/geometry/static text/ID totals; old+3nodes<=72/edges<=71/old+1focus<=18; used+n<=20480; all checked arithmetic. Build fixed triplet480 bytes plus all named index/visual/name metadata<=256; staging<=736 padded768 asserted target. Topology validation uses bounded scans rather than uncounted heap/second array. Future whole stack measurement separately unknown. After preflight ONLY infallible namecopy/reverse memmove/parentrenumber/tripletwrite/all adjacency rebuild/focusappend/visual/counters/attached-last. No fallible node/control/copy helper after mutation starts. [task-3h3c-marker-attach-dispatch-binding.md, § Exhaustive nonmutating preflight then infallible commit]

Insert triplet immediately after all toolbar descendants BEFORE tray/root guidance/tooltip/status/recovery. Renumber every shifted old parent>=insertion by3; rebuild child ranges/children in final flat source order. Append focus after existing toolbar controls, preserve current focusedID. Exact main order speed_down,pause,speed_up,launch,clear_selection,reset_camera,settings,help,scenario; optionalskip; three field choices;decrease/increase;advisory;optionalreset_guidance;marker. Maximum67nodes/66edges/18focus fits72/71/18. No early old-frame free. [zig/src/classic_ui.zig:269; zig/src/classic_ui.zig:287; zig/src/classic_ui.zig:300; task-3h3c-marker-attach-dispatch-review.md, § Original findings disposition]

## Required sequencing, resource and fixtures

prepareHUD -> geometry/input immutable borrow -> snapshot under trustedcurrent -> Corelease/project -> END all input/node/geometry/snapshot read leases -> attach mutable -> reacquire input/geometry -> H1 publish -> deinit prepared after borrows. Result owns inline data and survives snapshot destruction. Snapshot construction/project perform0 heap/token/event operations, but full fixed snapshot/CameraState/CampaignView/SceneFrame/geometry/staging/Result are real caller payloads to measure. No whole-process/zero-stack claim. Shared1179648 remains same FrameSlot budget; local HUD candidate81920,Header36864,node72/text49152/ID12288/focus18/ann2/context128/edges71 unchanged. Real allocation peaks include old resident+candidate+Header+H1 DFS/focus scratch and all pending charges; actualP/P-1 transition trace required, no fabricated fake OOM for allocation-free attach/adapter. [task-3h3c-marker-attach-dispatch-binding.md, § Borrow phase and resource/publication boundaries]

Independent12 default fixtures use actual accepted generator/Core camera/scene/pick, viewports723x802,960x600,1440x900 ×85/100/115/130. Prepare actual HUD from coherent Session immutable state, snapshot, pureproject, end borrows, attach, publish actual H1, then compare literal geometry/action/name and independently counted zones/DFS/text. Source default16controls/18zones; initialUnion0 ASTER VALE; two field adjust flagsfalse; sourcecameraFOV45/pitch55/distance15. Expected723 action_only;960/1440 scene_linked conditional on actual source picking and policy, coordinate tolerance.02px only, exactworld/mode/options. Fractional173.8/225.64 endpoints must pass shared predicate through attach/H1. Independent expected safe/zone/control formulas come from source, never generated by snapshot/project for golden values. Investigate source discrepancy rather than regenerate expected. [task-3h3c-marker-pure-dispatch-binding.md, § Independent source-derived fixtures; task-3h3c-marker-canonical-geometry-review.md]

Test closed seven mappings/marker7 forgery; enabledtouch vsdisabledtouch/positiveintersection;help/tray/guidance/tooltip;tiny collapsed safe;absentonce/repeatedattach;all invalidname/geometry/topology/cap errors preserve initialized wholeHeader;publishedban/repeatpublish/failedpublish retry;shifted descendants unchanged;fullname113 boundary through private measured B+113/B+112 preflight fixture without fake near49152source. Real allocator every prepare/H1publish ordinal, budgetreservation cancellation, retainoverflow unwind, oldframe/backing/focus/scalar/source unchanged on failure; repeated steady publications release allcharges/noABA/staletokens. Assert actualHeader/visual/Marker/snapshot/stage target sizes and record complete local/transitive stack unknown until measured. No compiler/test/gate evidence here. Protected Core/Session/math/camera/scene/pick identities must remain unchanged under future aggregate gate. [task-3h3c-marker-attach-text-fix-review.md; task-3h3c-marker-attach-dispatch-review.md]

Host dispatch must separately revalidate currentPlaying/currentinitialstep/currentUnion before Session action, materialize optional ring/connector accordingmode and qualify text/accessibility/focus. No host eventqueue or renderer accepted. Pure final exports/review/gate rebind is prerequisite before implementation; accepted HUD currently unchanged, rootREADME excluded from this freeze. No new root policy ambiguity identified; actualtarget/sourcequalification remains Pending.

## Read-only frozen source identities

|Source|SHA256|
|---|---|
|`zig/src/classic_ui.zig`|`26779e9ff238611fae6aaf1e786aa1b422bf6edcf823d963a6886fd4b66e4736`|
|`zig/src/classic_ui_tests.zig`|`12484dc907cc0930e205dbc8c226647eb21ca8838af6a029499d9e8245d951c2`|
|`zig/src/classic_app.zig`|`2d418ba49cdfb85d2ffd8d172cefc563b1b4cf87bb601729eb4045f791e2f1be`|
|`zig/src/classic_session.zig`|`747b27b98b15030931f214507568a9ced462121f47591b59db674c9383fd9568`|
|`zig/src/classic_camera.zig`|`ac5931069c0f65c44eaad0e093dc668ef61756a5ff19cdf5f69b02dfcbd0c268`|
|`zig/src/classic_scene.zig`|`e022c8961168b072f36cd26d0a84f9cb88bd17d904689df9d2614982c6f4ce11`|
|`zig/src/classic_picking.zig`|`e9dc79171f9f3906549642df0d53c3076a52b4427eb3a9ba375aab2541b0c219`|
|`zig/src/retained_semantics.zig`|`7e68e9b83d5b5da7e145a02bb8d2c04968558e22c307535b637cade597a735cb`|
