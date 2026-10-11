# Proposed I3 concrete Classic focus, owner and coordinator addendum

2026-10-09. Additive preparation only. Original router-guide binding is immutable. No product/compiler/gate/host/dependency authority. Parent's proposed rulings below require fresh review: preserve fixed-clock residue; reject stale-base Open with StaleProjection, allow current-guide F1/Escape Close/retry; synchronous complete raw-key/IME borrowing; semantic QueueFull/EventLimit/ContactLimit return typed failure with complete producer input retained. No drop/coalescing/DPR/host scheduling acceptance follows.

## Finite focus vocabulary and complete snapshot

Rust's FocusManager owns base_order, focused and optional modal{order,restore}; source guide saves the WHOLE manager, not merely its active focused control. Modal uses stable_unique order, independently preserves restore and refuses nested modal installation. [src/ui/accessibility.rs:307; src/ui/accessibility.rs:403; src/app.rs:1637]

Actual accepted Zig editor has109 identities:90 field IDs (13 globals+7*11 world fields),3 section actions,7 world selectors,7 footer actions and2 confirmation actions. Field tick_hz is noneditable but remains a well-formed historical identity. Keys are typed FocusKey(field FieldId/action Action), not arbitrary strings. [zig/src/classic_editor.zig:5; zig/src/classic_editor.zig:19; zig/src/classic_editor_ui.zig:6; zig/src/classic_editor_layout.zig:11]

Independent finite simultaneous order derivation:

|Editor section|Base eligible identities|Maximum|
|---|---|---:|
|Worlds|3 sections+7 selected-world controls+11 fields+7 footer|28|
|Rules|3 sections+12 editable globals+7 footer|22|
|Validation|3 sections+0 editable issue/status rows+7 footer|10|
|Confirmation modal|Dismiss then Confirm, no background eligible actions|2|

Rust independently enumerates3 tabs/7 world selectors,11 world fields/13 globals with tick rate readonly,7 footer and2 confirmation buttons; confirmation disables all underlay widgets. Zig generates the same populations and marks only editable fields eligible. Visibility does not increase counts and layout.next uses enabled/focusable, not visibility, for eligibility. [src/editor/layout.rs:23; src/editor/layout.rs:85; src/editor/layout.rs:165; src/editor/layout.rs:216; src/editor/layout.rs:385; src/editor/layout.rs:481; zig/src/classic_editor_layout.zig:85; zig/src/classic_editor_layout.zig:115; zig/src/classic_editor_layout.zig:135; zig/src/classic_editor_layout.zig:190]

The accepted editor explicitly rejects saved.order.len>28, duplicate base keys or confirmation IDs in saved base. It permits a separately valid historical restore identity, including one not in saved order; do not discard it or force restore membership. Actual source focus.current while confirmation must be index107/108 or null. [zig/src/classic_editor_ui.zig:258; zig/src/classic_editor_ui.zig:266; zig/src/classic_editor_ui.zig:769]

Proposed exact pointer-free snapshot representation:

```zig
const FocusDomain = enum(u8) { hud,settings,editor,guide };
const none:u8=255;
const FocusSnapshot = extern struct {
    domain:u8, base_len:u8, base:[28]u8,
    modal_present:u8, modal_len:u8, modal:[2]u8,
    focused:u8, restore:u8,
}; // 36 bytes exactly: all fields alignment1; assert sizeof==36.
```

Each byte indexes its DOMAIN's static catalog;255 alone means null. Domain validates before lookup. Modal absent differs from present len0; present zero-order is rejected under the actual modal contract. For editor,0..89 field catalog,90..92 section,93..99 world select,100..106 footer,107 dismiss,108 confirm. Base excludes>=107 and duplicates; modal can contain107/108 only and unique. Full snapshot including28 historical base IDs,2 modal IDs,focused and restore is copied by value. Null remains255 through capture and restoration; no first-control fallback is inserted by guide close. Ordinary modal CLOSE retains its actual existing H1 fallback rules as a separate operation. [zig/src/classic_editor_ui.zig:6; zig/src/retained_semantics.zig:287; src/app.rs:1649]

HUD static catalog24 entries (vocabulary, NOT simultaneous bound18):0..8 speed_down,pause,speed_up,launch,clear_selection,reset_camera,settings,help,scenario;9 skip;10..12 field/atmosphere,field/hydrosphere,field/topology;13 field/decrease;14 field/increase;15 advisory_ack;16 reset_guidance;17..23 marker/0..6. Exact ID bytes are a/hud/ + listed suffix. Full typed payload mapping includes marker world ID and field kind. H3a builds at most18 simultaneous controls, including at most ONE marker; retain hidden current HUD identity in focused even if absent from base, consistent with Session.focusIndex. [zig/src/classic_ui.zig:283; zig/src/classic_ui.zig:303; zig/src/classic_ui.zig:309; zig/src/classic_ui.zig:313; zig/src/classic_ui.zig:540; zig/src/classic_session.zig:266; zig/src/classic_session.zig:293]

Settings catalog11 indexes in source order scale/85,scale/100,scale/115,scale/130,motion,contrast,quality/auto,quality/low,quality/high,reset_guidance,close. ID prefix a/settings/. These are identity codes; a settings motion/contrast typed action's changing bool payload is recomputed from current preferences only after fresh capability validation, never cached as action authority. Guide catalog4 indexes Previous/Next/Menu/Close, exact guide.previous/next/menu/close. Guide base<=4, modal absent, focused optional. [zig/src/classic_ui.zig:325; zig/src/classic_ui.zig:350; zig/src/classic_guide.zig:389]

Coordinator owns exactly TWO FocusSnapshot values: current intended layer and optional saved pre-guide whole layer (presence flag separate from null focused). Current editor confirmation uses base + modal within ONE snapshot. When guide opens, copy that complete editor snapshot to saved and set current guide snapshot. No third duplicated confirmation snapshot or109-sized saved array. Closing guide copies saved snapshot into current intended layer, but saved remains alive until successful destination publication, allowing retry after failure. Mutation lease prevents base replacement while a live guide obscures it; owner change invalidates restore instead of retargeting historical focus. Source editor focus and settings/HUD fields remain Session-owned, synchronized through actual public focus/transaction methods. [src/app.rs:1637; zig/src/classic_editor_ui.zig:25; zig/src/classic_session.zig:300]

Decode editor saved base into ONE coordinator-owned [28]layout.FocusKey scratch for prepareEditorFromSession; canonical ActionId strings come from static catalog. No borrowed frame arena pointer persists. Proposed compile assertion sizeof(FocusKey)<=24 follows current shape: Action carries largest usize selector, FieldId carries tiny enum/world payload, outer tagged union; this is a layout ceiling, not measured sizeof. Thus adapter requested ordinary storage<=672. A future target failing that assertion requires review, not truncation. Scratch borrow ends before any snapshot/Session/source mutation and before reuse. Neither scratch nor FocusSnapshot owns TokenRefs. [zig/src/classic_editor.zig:8; zig/src/classic_editor.zig:32; zig/src/classic_editor_ui.zig:663]

## Actual H1 internal candidate ownership and external tickets

OwnedFrame.build performs full source preflight, topology scratch, copied frame allocations and focus construction before resident retain at744 and optional draft retain at747. Retain flags are set immediately after each successful retain. errdefer deinit releases acquired draft/resident flags. FrameSlot.replace does not destroy old frame until candidate built, reclassified and assigned. Therefore during a guide/editor transition the real maximum is old-frame2 + Header2 + H1-internal candidate2, not a single 'candidate' stand-in shared with Header. Same TokenRef in resident+draft still yields TWO retains in each consumer. [zig/src/retained_semantics.zig:688; zig/src/retained_semantics.zig:742; zig/src/retained_semantics.zig:759; zig/src/retained_semantics.zig:783]

Reserve two external candidate tickets before invoking publishGuide/publishEditor; these represent H1 internal refs only, with NO extra TokenRef.retain. They remain reserved through synchronous replace. If resident or draft retain fails, H1 unwinds; router cancels both reservations and leaves displayed tickets unchanged. On success candidate tickets become owned then transfer into frame ticket pair, old displayed pair released once. No hidden externally observable callback is needed because no source mutation/queue dispatch can overlap this synchronous call. Budget allocator probes can observe requested allocations, but registry entries for not-yet-acquired candidate refs remain reservations, not fake owners. [zig/src/classic_guide.zig:376; zig/src/retained_semantics.zig:744; zig/src/retained_semantics.zig:783]

Concrete ledger for this minimal Classic coordinator:

|Real ownership/reservation|Peak entries|
|---|---:|
|Root resident+optional detached draft|2|
|Old displayed actual H1 frame|2|
|One prepared guide/editor Header|2|
|One actual H1 internal replacement candidate|2|
|Saved guide base owner retained pair (explicit real retains)|2|
|Eight queue events each resident/draft|16|
|At most one serialized root replacement creation pair|2|
|Unused registry headroom|4|
|Profile|32|

Peak owned/reserved bound28, including independently retained saved owner pair; reserve32 is unchanged. No additional confirmation TokenRefs: base/modal snapshots are plain finite identity values and borrow the root domain until guide saved pair explicitly retains it. Creation pair and candidate coexistence is conservatively included; caller never begins a second preparation, candidate or creation while first lease active. If adding other owners consumes spare4, that requires fresh explicit enumeration, not an assumption every caller is covered. Root draft creation occurs on actual detached editor lifetime creation/replacement; content edits retain same token but change revision. Resident replacement occurs only on true campaign/session identity replacement, not every viewport or guide action. [task-3i-execution-slices.md:86; zig/src/retained_semantics.zig:107]

Guide Open saved-owner pair preflights tickets then actual retain resident/draft with exact first/second unwind. Numeric hold is not a separately allocated token and owns no refs. Root and saved pair keep origin domain valid after Close source effects until destination publishes; old frame independently retains it. Destroy saved pair only after successful leave or terminal teardown; never release copied borrowed identities. Full current match still requires BOTH token identities, hold, context and revision. [zig/src/retained_semantics.zig:84; zig/src/retained_semantics.zig:107]

## Complete proposed Classic context and hold taxonomy

Rust fallback semantic_input_owner formats screen/guide/library/modal/removal context; it explicitly excludes text values, rekeys a resident Rc on context change and uses hold=None,draft=None. Zig's stronger draft/revision protocol is accepted retained migration policy, NOT identical Rust fallback token semantics. Do not claim guide already creates source hold tokens in Rust. [src/app.rs:1208; task-3h3-consumer-budget-proposal.md:43]

Proposed closed Classic-only OwnerContext by value:

```zig
const ContextKey = struct {
    surface: enum(u8){classic,leaving_classic},
    base: enum(u8){playing,settings,editor},
    section: enum(u8){none,worlds,rules,validation},
    world:u8, // 0..6 only worlds, otherwise255
    confirmation:u8, // 0..6 actual ModalKind order, otherwise255
    guide:bool,
};
```

Validation is exhaustive: Playing/Settings require section none,world255,confirmation255. Editor requires current section; world current0..6 only Worlds,255 otherwise; confirmation actual seven-kind discriminant or255. Guide can overlay any validated Classic context, including confirmation. LeavingClassic forbids guide and all gameplay/editor/semantic activation; captures origin base only as inert retained ownership until teardown. No Library/Workshop/Living contexts are accepted by this API.

Context byte encoding is canonical fixed13 ASCII bytes `C|B|S|W|M|G|L`: B=P/S/E; S=-/W/R/V; W=-/0..6; M=-/0..6; G=0/1; L=0/1. The representation has seven one-byte components and six separators. Own [13]u8, len13; no format allocation or [128] dynamic context guess. Store same complete13 bytes in every current stamp; H1 Header/frame/queue copy as documented. Match exact bytes; decode rejects any inconsistent combinations. The context does not encode buffer values/cursor/preferences/pagination: EVERY effective change increments revision, including same token edits. [zig/src/retained_semantics.zig:107; task-3i-execution-slices.md:78]

Hold protocol: no guide/confirmation=>null. Confirmation opening=>hold=current revision allocated by checked next BEFORE mutation, persisted as confirmation_hold for that modal lifetime. Changing modal kind while already open creates fresh checked revision/hold. Guide opening=>guide_hold=Open committed revision; while guide active top owner hold is guide_hold, underlying confirmation_hold preserved in saved base owner/snapshot. Show/focus/resize retain guide_hold but change revision. Close restores current underlying confirmation_hold if it still matches saved origin, otherwise null only where source is genuinely no longer modal. Actual hold identities are unique within one monotonic coordinator revision domain, never reused or reset. A same-revision Open retry does not allocate a new hold or ref pair. No separate hold-counter exhausts independently. [task-3i-execution-slices-lifetime-fix.md:27; zig/src/classic_editor.zig:46]

Root owner proposal: one resident TokenRef for active Classic campaign identity; optional draft TokenRef only if detached editor exists. Playing/Settings projection owner MUST draft=null because actual UI rejects draft!=null. Editor owner requires its actual detached draft. Guide inherits underlying optional draft even while displayed as full-shell guide. Root may retain detached draft independently when hidden only if transaction source genuinely remains alive; do not conflate root owner set with projected OwnerSpec draft requirement. Record source ancestry in typed ContextKey; no external token or arbitrary context supplied by event. [zig/src/classic_ui.zig:140; zig/src/classic_editor_ui.zig:291; zig/src/classic_guide.zig:73]

## Smallest proposed complete headless API

In this pseudocode Session/Core/Editor/Guide/H1/Store are actual module aliases and Allocator is std.mem.Allocator; Session.Session is the concrete session type. The following is proposed exact PUBLIC surface. Input.KeyEvent, Input.EventSource/CurrentView, Interaction begin/update/end and error names must be rebound to frozen final modules before compilation; no intermediate source read is used here. Imported module type aliases remain accepted contracts, not invented existing exported spellings.

```zig
pub const ShellEffect = enum { none, return_to_main_menu };
pub const RouteResult = struct {
    handled:bool=false, shell:ShellEffect=.none,
    session:?Session.Result=null, projection:enum{unchanged,current,stale}=.unchanged,
};
pub const ImeEvent = union(enum) { preedit:[]const u8,commit:[]const u8 };
pub const GuideRequest = union(enum) { open,show:Guide.Location,close,main_menu };
pub const Recovery = union(enum) {
    close_current_guide, retry_projection,
    backspace_field:Editor.FieldId,
    replace_field:struct{field:Editor.FieldId,text:[]const u8},
    cancel_preedit:Editor.FieldId,
    dismiss_confirmation, cancel_editor,
};
pub const RecoveryStamp = struct{revision:u64,owner:H1.OwnerSpec}; // BORROWED call-only
pub const Redraw = struct {
    held:bool, stale:bool, outcome:?Core.FrameOutcome,
    permit_advisory_poll_submit:bool,
};
pub fn init(self:*Coordinator,allocator:Allocator,session:*Session.Session,
    workspace:*EditorLayout.Workspace) InitError!void;
pub fn deinit(self:*Coordinator) void;
pub fn key(self:*Coordinator,event:Input.KeyEvent,raw:Store.RawStore) RouterError!RouteResult;
pub fn ime(self:*Coordinator,event:ImeEvent) RouterError!RouteResult;
pub fn guide(self:*Coordinator,request:GuideRequest) RouterError!RouteResult;
pub fn enqueue(self:*Coordinator,event:Input.EventSource) Input.QueueError!void;
pub fn drain(self:*Coordinator,raw:Store.RawStore) RouterError!RouteResult;
pub fn pointer(self:*Coordinator,event:PointerEvent,raw:Store.RawStore) RouterError!RouteResult;
pub fn recover(self:*Coordinator,stamp:RecoveryStamp,operation:Recovery,
    raw:Store.RawStore) RouterError!RouteResult;
pub fn viewport(self:*Coordinator,value:Math.Vec2) RouterError!void;
pub fn focused(self:*Coordinator,value:bool) RouterError!void;
pub fn suspended(self:*Coordinator,value:bool) RouterError!void;
pub fn redraw(self:*Coordinator,now_ns:u128) RouterError!Redraw;
pub fn view(self:*const Coordinator) ?H1.FrameView; // expires before ANY mutation/deinit
pub fn current(self:*const Coordinator) Input.CurrentView; // borrowed full owner lease
```

PointerEvent is an exhaustive local normalized union: begin{id:u64,position:Vec2,source:Interaction.PointerSource,button:Interaction.PointerButton},update{id,position},end{id,position},cancel_all,wheel{logical_delta:f32},hover{position:Vec2}. It contains no trusted hit/world/action capability. Router computes current hit/opaque occlusion/picking after queue drain and current-source validation. Nonfinite source/derived coordinates reject typed NonFiniteGesture before camera mutation; accepting this source divergence needs parent review. ContactLimit pure-state rollback does NOT by itself rollback caller focus; propose preflight/admit contact before committing router focus effects so typed start rejection leaves complete router+interaction state unchanged. [task-3i2-interaction-dispatch-binding.md:38; task-3i2-interaction-dispatch-binding.md:72]

RouterError is union of actual underlying typed H1/Guide/UI/Editor/transaction/Input/Interaction errors plus explicit StaleProjection,SourceChanged,RevisionExhausted,InvalidContext,InvalidFocusSnapshot,OwnerLimit,UnsupportedSurface,NonFiniteGesture,CoordinatorClosed. Freeze exact aliases after accepted dependencies. Avoid anyerror diagnostic storage: persist a closed failure tag and requested bound numbers only. Session.Result's current error payload is surfaced unchanged as existing typed result, not serializable host protocol. [zig/src/classic_session.zig:18]

Initialization writes stable storage, creates only actual root resident and optional actual detached-draft token through SAME token32/requested1280 budget, reserves/marks actual tickets, constructs queue and initial full projection. Init failure unwinds all acquired refs/queue/preparation; caller retains Session/Workspace, not copied ownership. Result Coordinator remains inert/closed on failed init; reopening that partially admitted registry is forbidden, so caller destroys whole unused storage/lifetime rather than assigning init again. Runtime registration never moves coordinator; all terminal external borrows/ticket copies destroyed before address reuse. [task-3i-execution-slices-lifetime-fix.md:19]

Session/Workspace are borrowed stable owners provided once, not extra inline game/descriptor copy. RawStore is borrowed per call. Sole coordinator mutation lease also owns all projection-affecting Session mutation; caller must not mutate Session behind it. Viewports, redraw simulation/presentation, semantic/input/transactions and recovery all create effective revisions before any changed source. A no-op result consumes no revision. If source mutator can fail after partial effect, preserve its accepted state/error and commit revision for effective change, then Stale projection as needed; do not blanket rollback successful preferences because store save failed. [task-3h3-consumer-budget-proposal.md:43; zig/src/classic_session.zig:17]

Ordinary OWNED storage is explicitly bounded, separated from heap/preparation/Session:

|Storage|Bound/disposition|
|---|---|
|Two extern FocusSnapshot|72 exact bytes|
|One native editor saved-order scratch|28*sizeof(FocusKey)<=672, target assertion pending|
|Three full context byte arrays current/published/saved origin|39 exact bytes; no additional dynamic context buffer|
|Three ContextKey records|actual target sizeof pending, assert canonical semantic field bounds; fields enumerated above|
|Token/ticket owner pairs root/frame/prepared/candidate/saved/creation|12 token/ticket optionals maximum, registry already covers each; exact target padded size pending foundation freeze|
|Pointer controller|accepted8 specialization <=1088 target ceiling, final API/layout pending|
|InputState|two bitsets15 actions, final actual sizeof pending foundation freeze|
|One prepared projection discriminated handle|single active handle8 plus enum/optional padding, no concurrent Headers|
|One GuidePublication|32 accepted measured|
|Registry32/clock/live/preparation budgets, queue handle, FrameSlot|actual static sizes must use frozen foundation/H1 @sizeOf, no invented totals|
|Baselines/revisions/hold values/current-state flags/errors|explicit scalar records only; no unbounded text, ticket lists or callbacks|

Known focus+context owned byte lanes are783=72+672+39 maximum BEFORE alignment/other scalar fields; not claimed total Coordinator sizeof. Require per-field static assertions and actual sizeof total report during implementation. A whole-coordinator cap cannot honestly be exact before final foundation layout; this document fixes every population/field rather than guessing an overall number. QueueHeader heap slotstorage<=32768 plus actual metadata remains separate. Supplied Workspace descriptor storage remains caller-owned actual accepted layout, not hidden second array. [zig/src/classic_editor_layout.zig:14; task-3i-execution-slices.md:94]

## MainMenu scope correction: no headless acknowledgement protocol

Read actual source: apply_guide_action removes guide/restores focus BEFORE calling runtime.return_to_main_menu. For ActiveSession::Classic, that method has no fallible Classic step: it sets screen MainMenu, credits_visible=false and returns Ok; only resident Workshop prepare_for_durable_transition is fallible. Classic battle does not autosave, reset or drop in this method. [src/app.rs:1648; src/app/client_runtime.rs:4358]

Therefore this Classic-only headless coordinator needs only returned ShellEffect.return_to_main_menu. At emission it enters LeavingClassic, guide absent, baseline reset, no future Classic advance/advisory/activation. It retains old frame as explicitly stale/inert until caller tears down or future shell replaces it; no shell destination frame is invented. Caller can consume effect synchronously to move its own screen. No pending acknowledgement token, polling, error acknowledgement, shell failure state machine or duplicated host API is needed NOW. Original proposal's generic headless shell acknowledgement prerequisite is superseded for this Classic-only slice; actual multi-product shell owner/destination publication remains future host integration. An explicit later return-to-Classic transition would need reviewed ownership/source reset contract and is intentionally absent, not silently implemented through key events.

## Fail-closed recovery and proposed source rulings

- Current frame-derived actions require exact current/published/event revision+full owner and Current state. Guide replaces all Classic underlay dispatch; top guide capability admission dominates existing editor confirmation. Modal Confirm requires fresh emitted confirmation control and Current; it is NEVER a recovery operation. [zig/src/classic_guide.zig:389; task-3h3-consumer-budget-proposal.md:43]
- Stale base Open rejects StaleProjection before ref/focus/input/contact/source changes. Current-guide nonrepeat F1 or Escape maps directly to current-source CloseCurrentGuide recovery (not old-frame guide.close ID). Repeated F1/release does not close; stale guide Tab/Enter/button activation rejects. Explicit retry uses same current revision after new lease; failed retry never loops or increments revision. These are parent PROPOSED rulings pending review.
- Current-guide Close recovery validates RecoveryStamp against actual current origin tokens/context/hold/revision; removes intended guide at new checked revision, preserves complete saved focus/owner pair for failed destination publication, clears whole GuidePublication ONLY after actual successful leave frame. While failed Close is guide-absent Stale, stale guide pixels remain inert and retry can prepare base. No ContactLimit/queue event resolves it implicitly. [task-3h3-consumer-budget-proposal.md:45]
- Editor recovery only if current intended editor exists, no guide, and typed FieldId equals actual currently focused editable field. Reject tick_hz, wrong world/section/field, old stamp, absent editor or active confirmation. backspace_field calls actual backspaceFocused; replace_field calls actual replaceFocusedText with full UTF8 after guard; cancel_preedit calls setPreedit(""). No DeleteSelection/ReplaceSelection is advertised: current accepted Zig editor exposes no selection/cursor replacement API, and source parity here cannot invent one. An expanded cursor recovery vocabulary from earlier proposal remains unbound future scope. [zig/src/classic_editor.zig:134; zig/src/classic_editor.zig:156; zig/src/classic_editor.zig:167; zig/src/classic_editor.zig:181]
- While confirmation exists, only dismiss_confirmation recovery dispatches accepted .dismiss_confirmation; field recovery and Confirm reject. CancelEditor follows actual clean-close or CancelDirty request; dirty cancel may produce new modal, never bypasses confirmation. Ordinary replaceFocusedText would dismiss a modal without replacing under its own method; recovery rejects BEFORE invoking it so it cannot become an unreviewed bypass. [zig/src/classic_editor.zig:181; task-3h3-consumer-budget-proposal.md:45]
- Raw keyboard/IME arrives synchronously as complete borrowed UTF8, never enqueued into4096 semantic slot. Guide captures all IME; current editor forwards complete preedit/commit via actual API. On accepted editor allocation failure source buffers follow actual all-or-nothing content epoch contract and producer still owns full supplied buffer. Keyboard text precedence remains I1 source rule. This is proposed delivery ruling supported by actual synchronous Rust callers. [src/app.rs:2238; task-3i1-key-edges-dispatch-binding.md:84]
- Semantic enqueue queue full/oversize, pointer ContactLimit and malformed payload return typed failure without input truncation/drop/coalescing. Producer retains complete input and chooses external scheduling later. No accepted host backpressure or native contact bound. Duplicate/nonfinite contact starts reject before focus effects under proposed admit-first router ordering; finite-derived-overflow camera handling is deliberately fail-closed pending review. [task-3i-execution-slices.md:94; task-3i2-interaction-dispatch-binding.md:38]
- Guide redraw skips WHOLE Core.advance and advisory permits false; baseline resets entry/exit and first resumed framedelta0, speed/resumeSpeed unchanged. Preserve existing fractional Core.clockAlpha across guide, unlike actual onSuspend/onResume which clear it. Focus(false) clears Classic input+contacts; suspension clears input/timing but preserves contacts as corrected I2 source; occlusion does not imply cancel. [src/app.rs:1865; src/app.rs:1903; zig/src/classic_app.zig:283; zig/src/classic_app.zig:311; task-3i2-interaction-lifecycle-fix.md:9]

## Independent literal fixtures before implementation acceptance

1. Independently enumerate109 editor mappings and28/22/10 base maxima/2 modal; readonly tick_hz catalog presence yet no eligibility;7 world marker IDs and24 HUD catalog/18 simultaneous/11 settings/4 guide. Encode/decode roundtrip complete typed payloads, invalid domain/code255 lookup rejection, duplicates, wrong modal domain. Expected corpus authored from Rust/Zig source tables, not Coordinator outputs.
2. Capture28 historical editor base codes with absent/present focused and restore absent/frombase/not-in-base valid code; open confirmation2; guide save and Close reconstruct full exact snapshot. Guide-first nullable saved focus must remain null; no auto-first focus. Changing section/selected world during underlying invalid source is rejected by lease, never rebinds snapshot. Poison old frame bytes after capture and show owned/static snapshot survives.
3. Actual allocator/ref observation around Header retains, H1 resident-first/draft-second candidate retains and old frame destruction; inject actual second-retain failure with accepted test-only ABI method, prove Header and old frame remain, candidate0, reservations balanced. Same token resident/draft accounts two refs per consumer. Queue8 stale distinct pairs plus root/frame/Header/candidate/saved/creation footprint<=28 and registry cap32 verified without fake duplicate retain.
4. Enumerate every valid ContextKey combination and every invalid section/world/modal/surface tuple. Literal13-byte encoding comparison; hold absent/confirmation/guide overlay lifetime and closure. Same context/token with changed buffer or page revision rejects old event. Highest checked revision blocks mutation before source change.
5. Stale base F1 Open preadmission unchanged; stale current guide F1/Escape Close source mutation allowed and old controls rejected; retry same revision single attempt; failed Close retains saved pair and whole publication until success. No speculative confirm recovery or missing cursor APIs.
6. Full raw key/IME larger than semantic remainder and multibyte boundary unchanged; semantic QueueFull/EventLimit and ContactLimit preserve producer buffers and complete router start state. No source/device claim about contact8; DPR scheduling future.
7. Existing nonzero clockAlpha through guide with camera/visual_time/fingerprint fixed, advisory permit false, selected/resume speed all values preserved; exitfirstdelta0 then fresh interval only. Independent current read-only getters suffice (camera,clockAlpha,canonicalFingerprint,speed,resumeSpeed), so no Core private-field access or new observability production methods required. [zig/src/classic_app.zig:173; zig/src/classic_app.zig:235; zig/src/classic_app.zig:244; zig/src/classic_app.zig:283]
8. MainMenu emits one typed effect, guide removed, unchanged Classic campaign and speed, LeavingClassic blocks redraw/inputs while retaining old stale frame ownership; caller consumes effect, teardown zero. No fictitious shell acknowledgement witness.

All fixtures Proposed/unexecuted. Exact foundation/interaction final API/layout rebind, fresh review, real RED/GREEN, focused all-mode qualification and coordinated full gate remain required. No product dependencies/additional Core/Session/guide module changes justified beyond router/tests plus accepted narrow export/README registration; any missing current public focus/source bridge must be named and reviewed rather than private-field mutation.

## Frozen read-only identities

Relative paths are within canonical NYON. Original proposal identity is immutable anchor. Newly examined Rust/accepted Zig sources are current reads; no evolving foundation product files inspected.

|Source/document|SHA256|
|---|---|
| `.superpowers/sdd/2026-10-09-nyon-zig-migration/task-3i3-router-guide-integration-binding.md` | `08c2084bec26f4fd76d97cafdc6115c65ae9ad5defb6a26206dcc5d0ce4503f2` |
| `src/ui/accessibility.rs` | `526c2caa9beaf587a5afe3f680ad649436759899e4534fe6bf680c6b88849b54` |
| `src/editor/layout.rs` | `568d1ca1bffe48a98b08c928263b7e0198aa197363d39945240d5df8fe3125a7` |
| `src/editor.rs` | `069c2325e7db1c92529045740885b143892f4217ae26063e62ecae8ec7775439` |
| `src/app.rs` | `67b489723ec3886b38873f12ac6a21e763d0f6fc6d21c709b85fd27730eaaea2` |
| `src/app/input_router.rs` | `966a9618f3dc44dc25feae1e09f79ad5ee4f83b24189df2601b477e3a49cc68a` |
| `src/app/client_runtime.rs` | `10424db2220194b7fc20541ea0f95f32e7f95b9a2fd9f5d45747d04b8e0d9b30` |
| `zig/src/classic_editor.zig` | `aea58f15ff8e5a4291e9e8b3692ca6068813fb333525fe29e157f073ac2de26a` |
| `zig/src/classic_editor_layout.zig` | `cbe7c31dd70aedc9bf97d78abd718642779272b08c02512ad6e3ebb7febf323f` |
| `zig/src/classic_editor_ui.zig` | `829f33d96d78e30e27d34fcce64e59e08096ea2b9aa0dbdd1a4bb43980cf794b` |
| `zig/src/classic_ui.zig` | `bc97e432bee3cf3c560404823fb53ad9464b5086304867b06d29ee07bca4515d` |
| `zig/src/classic_guide.zig` | `f0bd20166c672821e875ee2b7f35fd698679b060ff9ee6237b6df7bff359742d` |
| `zig/src/retained_semantics.zig` | `7e68e9b83d5b5da7e145a02bb8d2c04968558e22c307535b637cade597a735cb` |
| `zig/src/classic_session.zig` | `747b27b98b15030931f214507568a9ced462121f47591b59db674c9383fd9568` |
| `zig/src/classic_app.zig` | `2d418ba49cdfb85d2ffd8d172cefc563b1b4cf87bb601729eb4045f791e2f1be` |
