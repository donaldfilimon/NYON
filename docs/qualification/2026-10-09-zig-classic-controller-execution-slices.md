# Proposed Task3I bounded execution slices

2026-10-09. Read-only additive preparation, not dispatch. Recover accepted intake/brief/FIX1 source rules without redrafting their full routing plan. Active marker integration classic_ui/tests/adapter/root/README were NOT read or adopted; future guide product does not exist and is not presumed. Actual final3H consumer APIs must rebind after accepted guide and current integration freezes, before router dispatch.

## Eight-file dependency and ownership map

The reviewed “eight modules” are four production modules plus four paired test modules, not eight new production abstractions. [task-3i-routing-brief.md, § Ownership and dispatch freeze]

|Slice|Owned future files|Producer → consumer|Qualification boundary|
|---|---|---|---|
|I1 owner/revision/semantic queue foundation, then normalized edges|classic_input.zig + classic_input_tests.zig|host/caller immutable owner+normalized input → queued borrowed lease/held-edge API → router|No Session effects, guide implementation or host delivery acceptance|
|I2 bounded gesture ownership|classic_interaction.zig + classic_interaction_tests.zig|logical pointer/contact + call-scoped typed HUD/scene hit → HUD tap/scene selection/orbit/zoom/release launch → router|No DPR/host event queue/camera mutation inside pure interaction|
|I3 consumer routing + guide orchestration|classic_router.zig + classic_router_tests.zig|I1+I2 and accepted consumer capabilities → existing Session/Core calls + explicit transition effects|Requires final actual3H rebind, caller GuideState/focus/fulladvance/baseline seams|
|I4 pure durable exit|durable_exit.zig + durable_exit_tests.zig|requires-save/ready facts → ExitNow/PrepareWorkshop/Wait/cancel → later host|No archive completion/host exit/battle autosave|

I1 precedes I2/I3; I2 precedes pointer routing I3. I4 is independent source-pure work but sequential ownership dispatch remains parent-controlled. Root/test exports/README minimal parent-owned hunks only after designated worker readiness; any new Session/Core seam remains separately coordinated, never implicit I1 ownership. No protected authority/camera/store/advisory/editor/layout/assets/dependencies changes. [task-3i-routing-brief.md, § Ownership and dispatch freeze]

## I1 concrete first bounded callable seam

NEW classic_input owns ONLY lifetime admission, monotonic revisions and semantic queue first. Its later key-edge state is a second bounded subchunk in same two files. It imports std/H1 only initially, no UI/guide/Session/Core. All following declarations are Proposed and compile/layout-qualify during authorized implementation.

```zig
pub const Revision = u64;
pub const Context = struct { bytes:[128]u8, len:u8 };
pub const CurrentView = struct { revision:Revision, owner:retained.OwnerSpec };
pub const RevisionClock = struct {
    value:Revision=0,
    pub fn next(self:*const RevisionClock) error{RevisionExhausted}!Revision;
    pub fn commit(self:*RevisionClock,expected:Revision,next_value:Revision)
        error{RevisionConflict}!void;
};
pub const SlotPurpose = enum {
    root_resident,root_draft,resident_frame,staged_frame,
    modal_hold,guide_hold,queued_event,transition,
};
pub const SlotTicket = struct { index:u8,generation:u64 };
const EntryState = enum { free,reserved,owned };
const RegistryEntry = struct {
    state:EntryState=.free,purpose:SlotPurpose=.transition,generation:u64=0,
};
pub const ReferenceRegistry = struct {
    entries:[32]RegistryEntry = [_]RegistryEntry{.{}} ** 32,
    pub fn init() ReferenceRegistry;
    pub fn reserve(self:*ReferenceRegistry,purpose:SlotPurpose)
        error{TokenReferenceLimit,TicketExhausted}!SlotTicket;
    pub fn markOwned(self:*ReferenceRegistry,ticket:SlotTicket)
        error{InvalidTicket}!void;
    pub fn cancel(self:*ReferenceRegistry,ticket:SlotTicket)
        error{InvalidTicket}!void;
    pub fn release(self:*ReferenceRegistry,ticket:SlotTicket)
        error{InvalidTicket}!void;
    pub fn deinit(self:*ReferenceRegistry) void;
};
pub const SemanticKind = enum { activate,focus,edit_utf8 };
pub const EventSource = struct {
    current:CurrentView,kind:SemanticKind,action:[]const u8,
    text:[]const u8="",
};
pub const QueueError = error {
    OutOfMemory,QueueFull,EventLimit,InvalidUtf8,InvalidPayload,
    TokenReferenceLimit,TicketExhausted,ReferenceOverflow,InvalidTicket,
};
pub const EventView = struct {
    current:CurrentView,kind:SemanticKind,action:[]const u8,text:[]const u8,
};
pub const SemanticQueue = struct { header:?*QueueHeader=null,
    pub fn init(allocator:std.mem.Allocator,registry:*ReferenceRegistry)
        error{OutOfMemory}!SemanticQueue;
    pub fn enqueue(self:*SemanticQueue,source:EventSource) QueueError!void;
    pub fn front(self:*const SemanticQueue) ?EventView;
    pub fn finishFront(self:*SemanticQueue) void;
    pub fn clear(self:*SemanticQueue) void;
    pub fn deinit(self:*SemanticQueue) void;
};
pub fn matchesCurrent(event:EventView,current:CurrentView) bool;
```

Context len128 does not fit u8? It DOES (u8 max255); preflight len<=128 before cast/slice. Context stores original complete UTF8, no normalization. `next` checked value+1; commit only accepts expected==value and next_value==checked expected+1, mutates once; no rollover/reused revision. Caller commits source revision for EVERY effective source/layout/input change affecting semantic display, including same draft edits, preferences/viewport/selection, and ownership/modal changes. Value-only clock cannot detect external mutation; caller mutation lease/protocol is required. Event match requires exact revision AND full OwnerSpec.matches (resident allocation identity/hold/fullcontext/optionaldraft); same draft token alone never proves current display. [zig/src/retained_semantics.zig:107; task-3h3-consumer-budget-proposal.md, § stale/current protocol]

Event activate/focus requires nonempty action and text empty; edit_utf8 requires nonempty exact field capability ID and complete UTF8 text. Generic queue does not decide EditorState field mapping or accept readonly edits; router later validates actual current frame/field/modal capability. This new payload seam replaces no source editor buffer. Guide destinations/confirmation reason/WorldId are NEVER event-supplied authority: queue activation contains immutable actionID, later trusted publication/capability chooses typed destination. No callback/platform object or raw pointer is persisted. [task-3h3d-guide-concrete-dispatch-binding.md, § GuidePublication; task-3i-routing-intake.md, § Source authority and exact routing]

## Reference ledger, requested bytes and noABA

Use actual stable LiveTokenBudget.init({max_live_objects=32,max_live_requested_bytes=1280}); TokenRef.create owns initial ref, retain checked, release frees before refund. Token40 is prior source-derived64-bit shape, not a new measurement: actual private Token @sizeOf must be pinned via H1 existing test/request observation, require32*actualTokenBytes<=1280. No private token storage export/edit. Every future creation must use SAME livebudget; no parallel unbounded token budget. Registry storage is separately measured ordinary coordinator storage, not counted as token object bytes. [zig/src/retained_semantics.zig:18; zig/src/retained_semantics.zig:82; task-3h3-consumer-budget-proposal.md:62]

32 caller reference/reservation slots conservatively cover frame resident/draft2+candidate resident/draft2+root resident/draft2+modal/guide holds4+eight event resident/draft16+transition temporaries6. Last6 include initial creation handles and pending creations/retains/transfers. Before create/retain reserve registry ticket; on failed allocation/retain cancel ticket, on successful ownership markOwned. H1 frame internal retains use reserved external tickets attached to caller lifecycle, not duplicate TokenRef retains. Transfer moves existing ticket; no owning copy from borrowed values. Queue events use up to2 tickets each. This is an explicit future caller seam, not proof all existing H1 consumers already register refs. I1 synthetic consumer tests enforce it; I3 integration must wire every actual owning slot. Livebudget independently enforces object+pending-byte limits even without ledger, but cannot enforce all caller reference-slot limits by itself.

Registry ticket generations checked increment, never wrap; exhausted ticket generation returns TicketExhausted before mutation. Private entry states enforce cancel reserved-only/release owned-only/markOwned reserved-only; stale ticket cannot free reused slot. Registry deinit requires allfree. Event TokenRefs stay retained until finish/rejection/clear/deinit, so old allocation cannot be reused while queued identity exists: noABA follows actual retained lifetime, not revision numeric equality alone. Eight events may own16 distinct stale resident/draft objects; no single-old-epoch assumption. [task-3h3-consumer-budget-proposal.md:62; zig/src/retained_semantics.zig:96]

## Eight4096-byte slots: explicit new Proposed storage choice

Earlier reviewed numeric proposal used8 events/4096 AGGREGATE and acknowledged actual payload unknown. Current parent task requests8*4096 storage. This package labels **8 fixed slots each<=4096 actual target bytes**, total slotstorage<=32768, as a new Proposed engineering choice for fresh review, not already accepted host policy. It supersedes aggregate4096 only IF parent accepts this package. Separate QueueHeader metadata allocation overhead is counted in actual request; no claim total allocation==32768. [task-3h3-consumer-budget-proposal.md:67; task-3h3-consumer-budget-proposal.md:69]

Concrete private EventSlot contains kind/revision,resident TokenRef,draft?TokenRef,hold?u64,twooptionalSlotTickets,three u16 byte lengths and initialized occupied state, plus bytes[3840]u8. Target metadata/padding<=256 and whole@sizeOf(EventSlot)<=4096 mandatory. Payload is disjoint context<=128 +action<=64 +text remainder, checked total<=3840; max full context/action leaves3648 text bytes. Offsets derived checked from lengths, no three extra dynamic buffers. UTF8 validate context/action/text before any retain or write. Fullsource payload too long returns EventLimit, producer retains complete buffers and event undispatched for explicit handling/retry, NEVER truncation/discard/coalescing. Host IME/preedit/rawkey/text delivery remains unbounded and outside queue admission; queue bounded edit event is not permission to cap underlying editor input.

QueueHeader contains allocator,registrypointer,slots[8],head/count and no extra pending event buffer. One actual allocator.create(QueueHeader) at init; actual sizeof/header request reported independently from PreparationBudget six populations/token1280. InitOOM leaves no refs/registry reservation. Stable heap Header avoids selfpointer/movablehandle payload slices escaping; move-only by convention. No per-enqueue allocation. enqueue checks full/strings/scalar bounds, reserves resident+optionaldraft tickets, retains with exact unwind, then infallible copy into free slot and count commit. All enqueue errors preserve queue bytes/count and input; temporary ticket generation bookkeeping may advance on canceled reservations, but occupancy/ref usage returns toprecall. Nofalse wholeRegistry-byte rollback claim. Existing queued refs/records are unchanged.

front returns borrowed EventView into stable occupiedslot; borrow ends BEFORE finishFront/clear/deinit/enqueue modifying same queue. Router later rechecks current fresh stamp per event, dispatches or rejects then finishFront releases both refs/tickets exactly once and FIFO advances. Pure foundation does not dispatch or mutate Session. Full queue typed QueueFull means producer receives explicit failure and owns retry decision; no host backpressure/drop policy accepted. Shutdown ordering: stop producers/endborrows,clearqueue,destroyframes/prepared/holds/root refs with tickets,then registry/livebudgetdeinit. Queue constructor caller allocator+registry address outlive Header; token livebudget/allocator outlive all refs.

## Subsequent consumer producer contracts and source decisions

I1 key subchunk defines normalized named/character key,press/release/repeat/modifiers plus borrowed text; InputState held/pressed edges,takePressedOnce/clear. Raw text/IME buffers not silently shoved into fixedsemanticqueue. I2 normalizedlogical contactIDs/button/hits emits sourcegesture decisions; threshold6 equality sticky moved, right ImmediateLaunch releaseonly, touchtargetno launch, pinchlogdistance*600/coincidentbaseline/survivormoved. Contact numeric count remains a separate intake blocker, not derived from event8. [task-3i-routing-intake.md, § Source authority and exact routing; task-3i-routing-brief-fix1-review.md, § Findings disposition]

I3 consumes event kinds and current actual FrameView, validates enabled/currentmodal/actioncap then calls actual Session.handleUiAction(action,raw),dispatchEditor(action,raw),primaryTap(hit,touch),cursorHover(hit,hud_consumed),keyboardAction(action),focusNext/focus/activateFocused with currentcontrols. Session currently publiclyowns core/editor/onboarding/settings/hud_focus; no guide state getter exists. These methods are actual stable source, not permission to invent privateunderscore access. [zig/src/classic_session.zig:35; zig/src/classic_session.zig:140; zig/src/classic_session.zig:210; zig/src/classic_session.zig:261; zig/src/classic_session.zig:300]

Priority remains F1nonrepeatguide then guidecapture,editor,Settings,Playing; HPlaying opensguide; enabledactivation and sourceHUDfocus includingdisabled remain distinct. Modal/tiny underlay node.action=null adaptation and historical focus identities/SavedBaseFocus remain accepted consumer contracts; no builder fallback invented. I3 coordinator owns persistent complete basefocus capture/restore, GuideState/location/trustedGuidePublication, source current/fullcontext/revision association. Guide publication record destinations never taken fromevent. Guide-held and held-transition frames discarddelta/resetbaseline; ENTIRECore.advance skipped (actualadvance updatescamera/visualtime BEFOREholds), selectedspeed unchanged. Concrete runtime baseline/GuideState seams still need parent-owned binding after actualguideacceptance, not this I1 fakecoordinator. [task-3i-routing-intake.md, § Guide correction and holds; zig/src/classic_app.zig:325; task-3h3d-guide-concrete-dispatch-binding.md]

I4 preserves request(false)=>ExitNow without clearingpending, request(true)prepare/Wait,observe-readyExitNowclear,cancelpriorpendingclear. No Classic autosave or hostclose/storecompletion claim. [task-3i-routing-brief-fix1-review.md, § Findings disposition]

## First slice independent fixtures and dispatch prerequisites

Literal fixtures:32reserved+ownedslots admit/33reject; every ticket lifecycle/stalegeneration/no-wrap;32distinctobject40bytes exact1280/one-less, actual request trace and allpendingcreation slots; eightdistinctstaleeventpairs and transition refs; tokenretainoverflow each unwind; actualQueueHeaderOOM; queue8/9FIFO,actual4096slotassert/metadata256payload3840/wholeheaderrequest;128context64action andremaining3648text/one-more typedfail with sourcefullUTF8 unchanged, multibyte exactbytes; malformedUTF8 and illegalkindpayload before refs; copiedsourcepoisoning;contextbytesdifferent/hold/resident/draft/revision separatelystale; event1sourceepochchange invalidateslater, fullowner same-token revisionchange rejects; frontleaseend beforedrain; clear/reject/shutdown/repeatedsteadyenqueue noleaks/ABA/oldrefsfreeonlyafterfinalrelease. Caller retains currentresident on every creationlimitfailure. Context/revisionexhaustion/sourcechanged-stale recovery are explicit error states, no silent retryactivation.

Actual size assertions and focused allocator failure receipts qualify during implementation, not impossible preimplementation measurement. No fake OOM for allocation-freeenqueue except real retain/resource failures. First I1 does not need transientUI or futureguide import; independent synthetic H1 owners suffice, but no hostacceptance follows. Before I3 dispatch: accepted final3H actualsource/APIs including guide and markerintegration rebind; complete producerdelivery/queuefull policy; callerrootowners/contexttaxonomy/revisioncommit/postsourcefailurestaledisplay protocol; contactbound; savedfocus/fulladvance/baseline/MainMenutransition ownership; actualtoken/queueprofile approval. Stale current source recovery must preserveinput/oldframe and restrictdispatch per acceptedconsumerprotocol, never activateoldpixels blindly. These are explicit blockers, not invented resolved seams. [task-3h3-consumer-budget-proposal.md; task-3i-routing-brief.md, § Ownership and dispatch freeze]

## Frozen documents and stable source observations

Hashes below are read-only source identities. Accepted source status of Core/Session/H1 comes from current parent acceptance; older intake evolving hashes remain historic receipts, not allcurrentAPI qualification. No activeUI/adapter/rootREADME identity was captured.

|Document|SHA256|
|---|---|
|`task-3i-routing-intake.md`|`d67931577d8a9d0a3174748e8f844a4aec26649e79642e027d2de42a5319be5e`|
|`task-3i-routing-brief.md`|`d2a5702b16fbbc3bf8145d94dbb011ab770ade972c9376483222f6ef99481e7e`|
|`task-3i-routing-brief-fix1-review.md`|`2dab0d66b560a5adf8440c8fe2ca9522eb2916485fffcae11961ca16b6e13816`|
|`task-3h3-consumer-budget-proposal.md`|`f4c2d9b3c5b84dbbaea33fd30ab1d9ee4ff7a7f48db4f6ee8ca0aec79400b202`|
|`task-3h3d-guide-concrete-dispatch-binding.md`|`3c509d380c77ef99a211fd9e6f87cd6259e415aa03283110ad0d62f963d3138c`|
|`task-3h3d-guide-text-inventory-fix.md`|`d95923652b90663020a2ef74b8ca2012926b3b19e63a35f5604b186191bcf1da`|
|`task-3h3c-marker-pure-dispatch-binding.md`|`63d071dbc644e5d25ad6dd9be835cbb9e26bb0d446a238ce7beb4391111ebfb6`|
|`task-3h3c-marker-canonical-geometry-binding.md`|`a3739143810e921195e37fb2ed245e3d96aa6baf4d44aeb08736db7957a58435`|

|Source|SHA256|
|---|---|
|`zig/src/classic_session.zig`|`747b27b98b15030931f214507568a9ced462121f47591b59db674c9383fd9568`|
|`zig/src/classic_app.zig`|`2d418ba49cdfb85d2ffd8d172cefc563b1b4cf87bb601729eb4045f791e2f1be`|
|`zig/src/retained_semantics.zig`|`7e68e9b83d5b5da7e145a02bb8d2c04968558e22c307535b637cade597a735cb`|
|`zig/src/classic_commands.zig`|`2b148fe0962af83308718ab20d4d216cdd04b30d086a210167591e3a1dae932b`|
|`zig/src/classic_editor.zig`|`aea58f15ff8e5a4291e9e8b3692ca6068813fb333525fe29e157f073ac2de26a`|
|`src/presentation/interaction.rs`|`2ef2451d458e7e4057aca99783c41816b2b28518b8fad9b7834ce46a299cce6a`|
|`src/app/durable_exit.rs`|`aa3d6156bf7400bbdf11f3a8b661ba7d977f9fcfd4a3758ea36b68465397f3b8`|
|`src/app/input_router.rs`|`966a9618f3dc44dc25feae1e09f79ad5ee4f83b24189df2601b477e3a49cc68a`|
