# Proposed Task3I2 pure interaction dispatch binding

2026-10-09. PREPARED, NOT DISPATCHED. Scope: new future `classic_interaction.zig` and paired tests, after accepted input foundation/edges. Full Rust gesture source and relevant pointer/touch callers were read. Parent fresh review and explicit numeric capacity disposition precede implementation. Final3H/guide/router rebinding remains later. No product, compiler, gate, host delivery, Session/Core/camera mutation or transient guide/UI/root/README adoption.

## Concrete proposed pure surface and capacity

Use `classic_math.Vec2` (x/y f32, isFinite); imports limited to std and classic_math. Proposed generic `InteractionController(comptime HudAction:type, comptime max_contacts:usize) type` avoids importing Session or an evolving UI module. Parent later instantiates with the accepted typed immutable HUD action value. HudAction must be a self-contained value with exhaustive equality (`std.meta.eql`), no pointers/slices/callbacks/owning resources; no control index or guessed action enum is introduced. Source's typed UiAction equality becomes full value equality, including payload. Production specialization freezes that concrete type and size before dispatch.

```zig
pub const PointerSource = enum { mouse, touch };
pub const PointerButton = enum { primary, secondary, middle };
// Within generic specialization:
pub const Hit = union(enum) { hud: HudAction, world: void, empty_scene: void };
pub const Gesture = union(enum) {
    hud: HudAction,
    scene_tap: struct { position: Vec2, source: PointerSource },
    immediate_launch: Vec2,
    orbit: Vec2,
    zoom: f32,
};
pub const BeginError = error{ NonFinitePosition, DuplicateContact, ContactLimit };
pub fn init() Self;
pub fn begin(self:*Self,id:u64,position:Vec2,source:PointerSource,
    button:PointerButton,hit:Hit) BeginError!void;
pub fn update(self:*Self,id:u64,position:Vec2) ?Gesture;
pub fn end(self:*Self,id:u64,position:Vec2,hud_hit:?HudAction) ?Gesture;
pub fn cancel(self:*Self) void;
pub fn hasContact(self:*const Self,id:u64) bool;
pub fn contactCount(self:*const Self) usize;
```

Normalized phases are direct calls: begin/update/end; cancellation resets the whole controller. There is no per-contact cancel API: actual touch Cancelled resets every contact (`src/app.rs:2369–2376`). u64 IDs use one shared namespace across mouse/touch. Actual mouse caller uses0/1/2 for primary/secondary/middle and ignores other mouse buttons (`src/app.rs:2433`). Touch passes its native ID unchanged and ALWAYS primary. Cross-source numeric collision therefore rejects duplicate begin; do not silently add source tagging or host ID remapping. Pure begin supports every declared source/button pair exactly as source, including synthetic touch-secondary, while actual touch caller never supplies that pair.

Actual Rust `BTreeMap<u64,Contact>` has NO explicit contact cap; `begin_pinch_if_ready` also allocates a temporary Vec of scene-touch IDs (`src/presentation/interaction.rs:70`, `:201`). Proposed bounded implementation uses max_contacts occupied-or-empty inline entries plus one optional f32 pinch baseline, no allocator or temporary Vec. Capacity is separate from semantic queue8. Proposed parent decision: max_contacts=8, covering any mix of admitted mouse/touch contacts, NOT an assertion of source/device maximum or eight-touch host policy. No default is provided. Parent may revise before dispatch; implementation cannot silently select another number.

Proposed target storage caps: HudAction<=64 bytes, each actual optional contact entry<=128 bytes, actual controller<=max_contacts*128+64 bytes (at proposed8, <=1088). Measure/assert actual target sizes, padding and baseline/count overhead; no compile/layout result exists here. If actual accepted action type fails caps, return for explicit profile revision instead of compressing/truncating action payload. Controller ordinary caller storage is outside token/PreparationBudget populations and queue allocation. All eight occupied entries may be distinct HUD actions or arbitrary contacts, not a special two-touch allocation assumption.

begin preflight order: nonfinite position, duplicate ID, capacity; typed failure leaves every entry/count/baseline unchanged. Duplicate succeeds in neither full nor partial registry. ContactLimit returns rejected start to caller without evicting, coalescing, canceling existing gestures or silently claiming delivery. Rust's false failure becomes explicit typed rejection for existing nonfinite/duplicate cases; ContactLimit is a NEW Proposed admission outcome requiring parent disposition. Caller retry/report/drop/backpressure after rejection remains host/router policy, not resolved by this package.

## Literal ownership and movement protocol

Rust `src/presentation/interaction.rs:84–113` chooses owner in this exact order:

|Hit/source/button|Owner|
|---|---|
|Any HUD action, any source/button|HUD(full action value)|
|World, mouse primary|TapOnly|
|EmptyScene, any primary|OrbitOrTap|
|World or EmptyScene, any secondary|ImmediateLaunch|
|Remaining World/EmptyScene (middle or touch primary world)|OrbitOrTap|

begin stores source,owner,start=last=position,moved=false; emits no Gesture. Any touch begin then invokes exactly-two nonHUD-touch pinch detection. World is a classification only, not a stored WorldId or permission to command a target. The router repicks on release as source does.

update rejects nonfinite position or unknown ID with null and unchanged state. Otherwise delta=position-last, update last, set moved=true when Euclidean distance(start,position)>=6.0 logical pixels. moved is sticky, including a return below threshold. TapOnly/HUD/ImmediateLaunch never emit on update. OrbitOrTap emits delta only once moved, using displacement from LAST update, not start and not total subthreshold displacement. Returning toward start after crossing still emits orbit; a zero delta after movement remains an Orbit value.

end removes contact first. If final position finite, set last and OR threshold movement; nonfinite final position keeps prior last/moved and still ends. Unknown ID=>null. Source can emit an unmoved scene tap or launch at last finite position after nonfinite end; HUD emission depends on supplied hud_hit equality. Do not claim all nonfinite end events reject or silently change this fallback. Unmoved HUD only emits when hud_hit exactly equals original action; no rebind to another control. Unmoved TapOnly/OrbitOrTap => SceneTap(last,source). Unmoved ImmediateLaunch=>ImmediateLaunch(last), ONLY on end and once. Moved owners emit nothing on end. End-only threshold crossing suppresses tap/launch but emits no final orbit delta. Repeated end=>null.

Use f32 staged vector difference, length sqrt(dx*dx+dy*dy), threshold and logs consistent with source; no squared-threshold replacement or f64 redesign. Math cross-libm bit parity is not promised (`zig/src/classic_math.zig:1`). Finite inputs can still produce infinite subtraction/distance at extreme magnitudes: source Orbit may carry nonfinite delta, while pinch has explicit finite guards. This binding preserves that source distinction and claims no stronger finite-output guarantee. Before actual router/camera admission, finite derived-value handling must be explicitly qualified/reviewed; no hidden clamp, extra coordinate ceiling or accepted new policy is supplied here.

## Exact pinch and multipointer state

On touch BEGIN, count all touch contacts whose owner is not HUD. ONLY count==2 starts/restarts pinch, sets both owners Pinch and moved=true, and computes baseline from their last positions. Mouse and HUD touch are excluded from count. Source BTreeMap is ID-ordered; inline implementation scans matching contacts in ascending u64 ID to retain deterministic order (including future fixture inspection). Two fixed IDs suffice for count/distance calculation; no temporary allocation.

When updated owner is Pinch, find exactly TWO current Pinch owners (not simply first two touches). Replace baseline with new distance before deciding emission. If previous baseline absent, or previous/current distance nonfinite or <=f32 EPSILON, return null after replacing baseline. Otherwise emit Zoom=(ln(current)-ln(previous))*600. Positive=>closer when consumer applies source zoom; negative=>farther. Coincident100=>0 then0=>100 both emit none;100=>120 resumes from finite baseline. No averaging, pan, centroid, zoom-at-anchor or unrelated Living camera gesture is added.

Third nonHUD touch does NOT rewrite existing pair: count3 prevents begin_pinch_if_ready, existing two stay Pinch and third retains its initially selected owner. Its update may orbit; its unmoved primary end may SceneTap while original pair still pinches. Ending that third contact does not rerun pinch detection. A HUD third touch leaves two nonHUD touches and therefore DOES rerun pair assignment/baseline (source calls detection after every touch begin). Do not substitute a nearest-pair, ignore-third, all-touch cancellation or new touch policy.

Ending a Pinch contact clears baseline and converts EVERY remaining Pinch owner to TapOnly with moved=true, suppressing survivor tap/orbit until it ends. It does not touch other contacts or automatically form a new pair. A later touch begin can form a new exactly-two nonHUD pair using survivors as source allows. cancel clears all entries and baseline; repeat cancel is harmless. Caller guide-open/reset/focus-loss/suspend clears controller under source rules; no clock/camera changes happen here.

## Caller ownership boundary

`src/app/input_router.rs:32` drains semantic actions and rejects pointer start if owner changed first. Guide/nonClassic primary and Classic editor primary dispatch directly through their own UI paths; they do not begin this gesture model. Classic callers focus first enabled HUD hit, suppress opaque HUD gaps, otherwise classify picked world/empty and clear HUD focus (`:146–180`). Capacity admission cannot imply those caller effects rolled back; pure begin guarantees its own state only. A parent router protocol must decide whether to admit before committing caller focus effects, without claiming source already has ContactLimit handling.

Release caller computes CURRENT HUD hit and passes it to end (`:244`); gesture action then goes to trusted consumer mapping. Touch SceneTap calls primaryTap(...,true), which selects/targets and never implicitly launches; secondary ImmediateLaunch calls explicit pointer launch; Orbit consumer applies(-dx*0.005,dy*0.005); Zoom consumer applies logical_delta (`:251–278`). Pure module never picks world, calls Session/Core, applies camera motion or trusts an event-supplied gameplay target. HUD action current/enabled/context validation and exact accepted3H types are I3 obligations. No host/DPI/wheel callback or guide content imports.

## Independent literal fixtures for later implementation

- All ownership rows, all source/button combinations; primary mouse world TapOnly drag never orbits; touch-primary world drag orbits after threshold but unmoved tap selects only; synthetic secondary any source emits only end. HUD ownership overrides buttons; changed/null end action does not activate; full typed action payload inequality rejects.
- begin duplicate ID, cross-source ID collision, NaN/+Inf/-Inf components and full capacity typed rejection leave entries/count/baseline unchanged. N exactly admitted, N+1 ContactLimit; ending one allows one fresh admission without moving other IDs/owners. Empty/one/max/cancel cycles, no allocations. Measure storage caps on actual target.
- Origin0, update(5.9,0)=>none, update(6,0)=>Orbit for OrbitOrTap, delta(0.1,0); update(0,0) remains moved and Orbit(-6,0); end(0,0)=>none. Secondary release5.9=>one launch; release6=>none; crossed6 then return0=>none. End-only6 crossing no orbit. Mouse-world TapOnly crossed6 never orbit/tap. Unknown/repeated end null; nonfinite update unchanged, nonfinite end removes and uses last finite unmoved position.
- Two primary scene touches IDs1/2 at0/100: update2=>200 gives600*ln2 (~415.8883), back100 gives negative equal magnitude within documented f32 tolerance.100=>0 and0=>100 no zoom;100=>120 then600*ln1.2 (~109.3929). EPSILON/zero/nonfinite computed distance baseline replacement explicitly witnessed; no claimed cross-libm bitwise equality.
- Pair ending suppresses survivor even at original coordinate; survivor TapOnly moved produces neither orbit nor tap. Cancel pair resets all/baseline and permits fresh pinch. Mouse+one touch not pinch; HUD touch+one scene touch not pinch; pair plus third scene touch preserves pair and third independent orbit/tap; adding HUD touch to pair resets current baseline; ending nonPinch third does not recompute pair. Start permutations and ascending ID scans deterministic.
- Extreme finite f32 positions reproduce source derived overflow distinction; no finite-output or camera acceptance claimed. Later router fixture qualifies its handling explicitly. Actual consumer touch-selection and HUD occlusion/current capability validation stay independent integration tests, not pure gesture evidence.

Source contact capacity is unbounded; proposed8 and target storage ceilings require fresh parent review and acceptance before bounded dispatch. Host handling of ContactLimit and derived nonfinite output remain explicit integration prerequisites. Queue8×4096 remains separately Proposed. No literal fixture has been executed.

## Frozen evidence

|Evidence|SHA256|
|---|---|
|task-3i-routing-intake.md|d67931577d8a9d0a3174748e8f844a4aec26649e79642e027d2de42a5319be5e|
|task-3i-routing-brief.md|d2a5702b16fbbc3bf8145d94dbb011ab770ade972c9376483222f6ef99481e7e|
|task-3i-routing-brief-fix1-review.md|2dab0d66b560a5adf8440c8fe2ca9522eb2916485fffcae11961ca16b6e13816|
|src/presentation/interaction.rs|2ef2451d458e7e4057aca99783c41816b2b28518b8fad9b7834ce46a299cce6a|
|src/app/input_router.rs|966a9618f3dc44dc25feae1e09f79ad5ee4f83b24189df2601b477e3a49cc68a|
|src/app.rs|67b489723ec3886b38873f12ac6a21e763d0f6fc6d21c709b85fd27730eaaea2|
|zig/src/classic_math.zig|e0b5188b08ab78a4c32a348ba0398150ab2b8ac0541a8fe4984b9e4d60913186|
|zig/src/classic_session.zig|747b27b98b15030931f214507568a9ced462121f47591b59db674c9383fd9568|
