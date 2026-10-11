# Proposed H3c immutable-lease adapter dispatch binding

2026-10-09. Additive correction to review4bf17b67704be29754159d3e6564b5fa89268936396613e2469d885d050ceab0. Originals unchanged. Root engineering ruling selects Choice B: source180x54 safe-clamped label inset1, first admitted source candidate remains first even action_only; hide ring AND connector if unsafe/occluded; no camera mutation; closed overlaps-or-touches exclusion enabledcontrol/tray/help, positive-area overlap exclusion disabledcontrol/guidance/tooltip. No universal availability claim. Actual H3a accepted-source/API rebind pending; this is Proposed dispatch contract, no implementation/host acceptance.

## Concrete Proposed lease, snapshot and API

```zig
pub const CurrentStamp = struct { revision:u64, owner:OwnerSpec }; // borrow-only
pub const CurrentLease = struct {
    core:*const AppCore, step:?onboarding.Step, stamp:CurrentStamp,
};
pub const ZoneKind=enum { control,tray,help,guidance,tooltip };
pub const Zone=struct { kind:ZoneKind,bounds:Rect,enabled:bool };
pub const BorrowedHudSnapshot=struct {
    revision:u64,resident:TokenRef,hold:?u64,
    context:[128]u8,context_len:usize,
    viewport:math.Vec2,safe:Rect,
    edges:struct{left:f32,right:f32,top:f32,bottom:f32},
    settings:bool,zones:[26]Zone,zone_count:usize,
};
pub const Mode=enum { scene_linked,action_only };
pub const Marker=struct {
    world:model.WorldId,name:model.WorldName,point:math.Vec2,
    label:Rect,mode:Mode,
    connector_start:?math.Vec2,ring_radius:?f32,
};
pub const Absent=enum{not_initial,not_playing,no_label_candidate};
pub const Result=union(enum){marker:Marker,absent:Absent};
pub const ProjectError=error{InvalidWorldSet,InvalidViewport,
    InvalidCamera,InvalidBounds,ZoneLimit,ArithmeticOverflow};
pub const AdapterError=ProjectError||error{SourceMismatch};
pub const Input=struct{
    step:?onboarding.Step,playing:bool,viewport:math.Vec2,
    camera:camera.CameraMatrices,
    scene_worlds:*const [7]scene.SceneWorld,worlds:*const [7]model.World,
    safe:Rect,edges:OpaqueEdges,settings:bool,zones:[]const Zone,
};
pub const OpaqueEdges=struct{left:f32,right:f32,top:f32,bottom:f32};
pub fn project(input:Input) ProjectError!Result;
pub fn projectFromCore(lease:*const CurrentLease,
    hud:*const BorrowedHudSnapshot) AdapterError!Result;
```

CurrentLease is caller-held immutable borrow, no runtime lock/token/revision source hidden inside Core. Caller establishes it from current source coordinator, excludes all Core/Session/onboarding/HUD/owner mutation/publication/move/deinit for synchronous call and keeps resident/hold/context live. No callback or getter to reread trusted current exists; adapter cannot detect revision change while lease violated, and never claims before/after change detection. By-value/captured stamp supplies provenance only under this explicit precondition. Caller must log/qualify lease construction and complete call ordering in later integration; pure tests explicitly own immutable inputs. Step must be current Session.onboarding.step from the same locked-out source epoch, not arbitrary stale enum.

BorrowedHudSnapshot context and populated Zone records are copied value metadata, no source dynamic slice escapes; revision/context bytes represent exact pre-marker HUD preparation state. resident is borrowed capability, no retain when constructing/copying snapshot. Caller keeps source owning prepared Header/frame resident handle alive and stable throughout snapshot and lease use, forbids successful frame replacement/deinit. Snapshot owns no token or allocator object, needs no deinit/release; context_len<=128,zone_count<=26 checked BEFORE slice. Snapshot metadata storage and complete local stack measured separately, no claim it is zero bytes. Construction function must bind actual accepted H3a preparation/owner/viewport/zone exports later; not currently existing public API. No storing snapshot beyond underlying owner lifetime or transferring across publication. HUD preparation stamp/current source revision match exactly, full context bytes no normalization, draft mustnull for marker slice.

Adapter checks hud.revision==lease.stamp.revision; hud.resident.sameIdentity(stamp.owner.resident),hold equality, exact context bytes/length,draftnull; otherwise SourceMismatch. It does NOT read Core revision/token/context because none exists. It extracts local CameraState via core.camera(),CampaignView via core.campaign(),SceneFrame via core.sceneFrame() under lease; all local values remain live until project returns, borrowed scene/model arrays never escape. Normalized viewport=max(core.viewport(),1) components (HUD source nonfinite fallback1x1); reject raw nonfinite as InvalidViewport because accepted Core setter would never install it. Require camera.logical_viewport exactly equals normalized viewport and hud.viewport exactly equals same; mismatch SourceMismatch. Scene.camera is used for projection/picking, not independently substituted CameraState matrices; camera extract serves viewport provenance check. Finite scene matrix checks InvalidCamera only. Zero raw Core viewport normalizes1 for camera/HUD match; collapsed safe/label produces absence, no small-window promise. Supported12 test cases have raw==normalized dimensions. Source setup cannot mutate during extraction; if violated numeric equality checks do not prove coherence. [classic_app.zig:141-159,173,228; classic_camera.zig:35-50,105-120]

Scene/campaign world ID sets unique0..6/correspond, canonical names/finite positions/positive radii validated; projection matrix finite does not prove inverse consistent with view/projection. Arbitrary finite matrices remain valid synthetic numeric seam; production provenance comes solely from accepted scene/Core extraction lease. Latest3G3 accepted source manifest and H3a owned snapshot API must rebind before production adapter dispatch. No float tolerance inverse validation invented. SourceMismatch exact captured HUD/current/normalized viewport mismatch, no all-state mutation detection.

## Final Choice B geometric/selection policy

Iterate unchanged scene order. Noninitial/notPlaying absent before candidates. Candidate Union owner by ID,finite clip,w>0,x/y screen in inclusiveviewport; no additional ndcz filter. Source label formulas from original source binding first. Clamp full label180x54 nearest into safe inset1: component clamp(sourceMin,safeMin+1,safeMax-1-extent), empty interval yields no label for that candidate. Check label exclusion zones: enabledcontrol/tray/help closed rectangle overlap OR touches rejects; disabledcontrol/guidance/tooltip positive-area intersection rejects. Source point consumption still inclusive opaqueedges/settings and enabledcontrol/tray/help; guidance/tooltip do not consume point. Full label lies inset safe, preserving direct typed selectability without requiring scene point be pickable. Every candidate label admitted under these rules immediately wins, even if scene_linked false; never prefer later Union for ring visibility.

Scene_linked iff point and ring22 bounding Rect fully inside safe, no applicable zone overlap/touch, source point not consumed, accepted Ray.fromLogicalPointer and pickWorld over SAME7scene spheres selects sameID. Otherwise action_only:connector_start=null,ring_radius=null. scene_linked connector_start=label.min+(90,54),ring_radius=22. Renderer must not draw hidden geometry; line full visibility not otherwise guaranteed. Marker action remains source SelectGuidanceWorld, exact START HERE and source tooltip; world-name description retained full if mapped, count actual copies. No pan/focus/zoom/camera state mutation. Full safe label cannot fit or outside viewport/noUnion/blocked overlays yields no_label_candidate; no universal availability promise.

12 default723x802/960x600/1440x900×85/100/115/130 f32 product fixtures mandatory. Prior scalar analytic table is expected conditional guide, not actual target result.723 analytically action_only because pointx105.464 outside every safeleft; label inset x173.8/199.72/225.64 clears named default overlays analytically, still must qualify actual rectangles/matrices/pick.960/1440 may scene_linked only after ring/picking tests. Explicit synthetic7 sphere fixtures in correction remain independent oracles. Add firstUnion action_only versus laterUnion scene_linked priority literal, exacttouch versus positiveoverlap per zone enable kind, stable lease snapshot lifetime/mismatch, no tokenretain and complete output inline-name survival after source reuse.

## Resource and future ownership

Pure projector/adapter allocate0, create/retain/release0 tokens, queue0 events; output fixed Marker inline name. Proposed @sizeOf(Marker)<=128 includes optional enum/connector/ring fields, must assert target. Snapshot full Context128+Zone[26]+token/revision/geometry metadata and adapter local CameraState/CampaignView/SceneFrame/PickWorld[7] are real caller/local storage, target/stack measurement required; do not count only Marker128. No shared budget cap growth or permanent Header snapshot added without accepted H3a exact layout rebind. When future retained integration copies one marker into admitted H3a Header, existing17→18control/64→67node slack applies, text64 plus optionalname24 as already counted; actual Header target assertion still required. Owner/revision validation current frame action then actual currentPlaying/initialUnion check before Session dispatch belongs later host3I, not claimed solved by project.

Future NEW classic_start_marker.zig/tests owns pure project and synchronous Core adapter after root sequential dispatch; parent owns accepted H3a marker integration/test/export hunks only once actualH3a/H3c qualification freeze. No edits math/camera/scene/picking/session/authority/store/assets. Originals preserved. Fresh independent review, source manifest rebind, actual acceptedH3a construction seam,12default f32/picking geometry and target/local fixtures precede implementation. Materialization/native/browser/accessibility/game completion outside scope.

## Frozen prerequisite artifacts

- task-3h3c-marker-source-binding.md: `c639cd8f2bb7dfe7c9452893724460f2d5a4f781ee9b9dbd3149d52f21647700`
- task-3h3c-marker-policy-correction.md: `d82815aa9531aa98680b2c3de105b5db08ddf40ca08944e5e1c8ffae1dc60748`
- task-3h3c-marker-policy-correction-review.md: `4bf17b67704be29754159d3e6564b5fa89268936396613e2469d885d050ceab0`
