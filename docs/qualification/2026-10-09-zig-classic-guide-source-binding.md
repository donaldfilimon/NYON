# H3d guide source binding

2026-10-09. Proposed preparation only. Sole new documents; existing packages immutable. No implementation or host dispatch acceptance. H3a transient product is not a dependency qualified here.

## Source content and closed vocabulary

The seven topics and every paragraph must be copied verbatim into proposed `classic_guide.zig` static literals, with a source paragraph index, never replaced by a summary. This includes Workshop/Living instructions as informational prose without implementing those modes. Source titles and counts follow (`src/ui/guide.rs:15-86`). Counts are UTF-8 paragraph bytes excluding titles and separators.

|Topic|Title|Paragraphs|Bytes|Words|Conservative wrap lines|
|---:|---|---:|---:|---:|---:|
|0|Choose your game|3|787|131|134|
|1|Classic // First fleet|5|1229|203|208|
|2|Workshop // First energy|5|1204|204|209|
|3|Workshop // Ore to alloy|4|1138|186|190|
|4|Living Galaxy // First simulation|9|3548|521|530|
|5|Controls // Mouse and keyboard|4|1225|205|209|
|6|Save, exit and graphics|4|1460|236|240|

Total: 34 paragraphs, 10591 bytes, 1686 words, at most1720 lines across all topics. Per active topic maxima are9 paragraphs,3548 bytes,521 words,530 lines. These are maximum source bounds, not simultaneously attained wrapped line counts. Each paragraph contributes at most one line per word plus its trailing blank. Wrapping uses byte lengths, whitespace splitting, single-space word joining and no word splitting (`guide.rs:279-300`). Keep all original ASCII literal bytes in static storage; selected semantic page normalizes whitespace as source. Static topic/paragraph tables are not runtime heap allocations. Do not generalize this API to arbitrary caller content.

## Exact geometry, navigation and priority

Finite source viewport is componentwise at least320; nonfinite source viewport becomes640x480. Proposed additionally reject finite overflow in subsequent arithmetic with `InvalidGeometry`, rather than unspecified float-to-integer saturation. columns=floor((w-36)/9.6), rows=max(1,floor((h-222)/22)); topic clamps6 and sheet clampsceil(lines/rows)-1. Previous crosses to preceding topic's last sheet, Next crosses to following topic's first; endpoints omit their controls. Slots remain indices0 Previous,1 Next,2 Main menu,3 Close, even when omitted. Bounds x=8+i*((w-40)/4+8), y=h-62, width=(w-40)/4,height48. Body line positions18,134+i*22,scale1.6. Source sighted body is ASCII uppercase; semantic description is original-case `lines.join(" ")` (`guide.rs:126-277`). Never use uppercase text as the semantic source.

Source IDs: guide.application Application name NYON player guide; guide.dialog Dialog name topic title; guide.body Text name Instructions and joined description. Button IDs guide.previous.node,guide.next.node,guide.menu.node,guide.close.node; action IDs remove `.node`, roles Button, names/descriptions Previous,Next,Main menu,Close, enabled=true,selected=false. Focus order is present controls in slot order, not topic order. Announcements empty. Platform title NYON // Player guide; status `<title> (<sheet+1>/<sheets>)` and `Time held // Tab + Enter // Escape closes` (`guide.rs:170-274`). Proposed adds explicit static title/status semantic Text nodes and value children for body/buttons, capped19 nodes,18 child indices; source topology originally at most7 nodes. New IDs guide.title,guide.location,guide.held,guide.body.value and `<button>.value`, `<button>.description` are closed static identities. Source-derived values have no caller arbitrary text.

Guide replaces the displayed frame: it does not leave an interactive HUD/editor beneath a dialog. App opens only if absent, saves complete previous FocusManager, clears interaction/input and camera touches/drags; Show changes location only while open; Close restores saved focus; Main menu restores focus then requests runtime transition and clears relevant creator/removal state. All actions invalidate frame timing and rebuild (`src/app.rs:1626-1667`). Initial Classic topic1, Workshop2,Living4,other0 (`guide.rs:87-105`). Source guide priority is full shell replacement; proposed integration must resolve existing editor confirmation ownership before entry, never activate stale underlay through guide pixels. No queue mechanism is established here.

## Concrete Proposed pure API and storage

```zig
pub const Location = struct { topic: u8, sheet: u16 };
pub const Action = union(enum) { show: Location, close, main_menu };
pub const Input = struct {
    location: Location, viewport: Vec2, high_contrast: bool,
    focused: ?ActionId, current: CurrentStamp,
};
// CurrentStamp is a call-borrowed revision plus accepted H1 OwnerSpec.
pub const PrepareError = error{ InvalidGeometry, ArithmeticOverflow,
    PreparationLimit, OutOfMemory, RetainOverflow, InvalidOwner };
pub fn prepareGuide(allocator: Allocator, budget: *PreparationBudget,
    input: Input) PrepareError!PreparedGuide;
pub fn publishGuide(slot: *FrameSlot, published_revision: *?u64,
    prepared: *const PreparedGuide, current: CurrentStamp) PublishError!void;
pub fn validateGuideAction(view: FrameView, published_revision: u64,
    current: CurrentStamp, id: ActionId) ActionError!Action;
```

All names above except H1 types are Proposed, not existing exports. `PreparedGuide` is a move-only heap-header handle; methods `deinit`, borrowed `frameInput`, borrowed `bodyLines` stay valid only while its stable header lives. Publish checks captured/current complete owner identity and revision; synchronous replace updates scalar only after success. Validation uses actual FrameView.owner plus trusted stored revision, actual node.action/enabled/focusable and guide modal owner; null publication admits no action. No copied owning stamp or generation reader is invented.

Header contains `[521]WordSpan` and `[530]LineSpan`, each future target asserted at most16 bytes. WordSpan stores static paragraph index,u16 byte start,length; LineSpan stores first_word,count (zero means paragraph blank). Wrap into header without dynamic strings; preceding-topic last-sheet calculation reuses spans before active topic wrap. Static original paragraph literals never escape temporary storage because they are module lifetime. Materialize selected-page original-case joined semantic text and uppercase sighted text into one8192-byte Header arena. Each <=3548+9=3557; remaining1078 admits title/status/control copies bounded512 total. No complete-topic second buffer, no stack-owned ProjectionSource escaping. Header also owns24 NodeSpec descriptors,23 child indices,4 focus IDs,128 owner-context bytes,8 optional announcement descriptors, and metadata. Future assert actual Header<=32768, counts within caps and text arena use<=8192; assertion failure blocks dispatch, does not enlarge caps. Context beyond128 returns typed limit preserving source. Static action/location mapping stores at most4 entries; include it in Header metadata<=1024. Descriptor fields are borrowed offsets materialized from the stable header, never handle-relative addresses captured before a move.

One actual Header allocation charged `.frame_scratch`: reserve exact @sizeOf(Header) before alloc, cancel on OOM, commit actual requested charge; copy then checked retain resident and optional draft once each. Every unwind releases obtained references, frees allocation before charge.release using copied charge metadata. Stable budget outlives Header/frame. Caller live reference-slot allowance includes resident/draft input owners, prepared owners, slot owners and candidate construction, without a invented global token registry. Existing checked TokenRef retain overflow remains mandatory.

## Numeric admission and accounting

Proposed limits: nodes24,IDbytes4096,textbytes16384,focusactions4,announcements0,childindices23,ownercontext128,max_owned32768. Source guide announcements remain empty; Header optional announcement storage above is conservative descriptor allowance only, no emission or allocation. All retained copies counted separately: body semantic<=3557, sighted body<=3557 if copied into retained value, repeated button names/descriptions/values <=4*3*9=108, title/dialog/location/held/application <=256. Thus text total<=7478<16384. IDs<=24*64+4*64+4*64=2048<4096, each literal independently <=64; full UTF8 identity retained, no normalization. Focus raworder4, unique4; no modal/base duplicated raw lists. Owner context128 independent from ID arena.

At accepted target record sizes NodeSpec160,ActionId16,usize8, no announcement payload: R=24*160+23*8+4096+16384+128+(4+4)*16=24760. Conservative bound uses24872 allowing80 announcement-record slack and extra32 focus slack, despite announcement cap0. Candidate local<=32768; old resident+candidate<=49744 using conservative R. Header<=32768 plus DFS/focus scratch24*8+4*24=288 => named requested peak82800. Proposed shared budget98304 has15504 slack. This is enumerated payload, not whole-process memory: actual optional revision metadata layout, movable handle, ordinary call stack, allocator bookkeeping and transitive materializer stack remain future measurements. No hidden stack text buffers. Entire-source static10591 storage is separate immutable binary data. Actual target assertions and measured all-ordinal failures must establish this profile before dispatch. Global consumer budget policy is not automatically this slice's98304 authorization (`task-3h3-consumer-budget-proposal.md`, Limits/accounting sections).

Typed overflow/limit/OOM leaves old frame and trusted revision intact, source location/input preserved; no truncation/fallback overwrite. Caller explicitly retries with admissible geometry/context/resources. Navigation source enums contain bounded u16 sheet (530 maximum), statuses require at most3 decimal digits per number; topic title max33 plus space/parens/slash and6 digits gives43-byte location status. No integer/f32 text formatter other than these decimal sheet numbers. Geometry floats do not enter text.

## Pause boundary and missing ownership

Source suppresses entire Classic advance and advisory poll/submit while guide open; redraw also uses delta0 while held and across held-state changes (`src/app.rs:1857-1878,1903-1907`). Zig AppCore.advance updates camera and visual time before its mode/speed/hold checks (`zig/src/classic_app.zig:325-340`), so calling advance(0) or setting paused speed is insufficient. Future caller skips whole advance while guide open and resets timing baseline across entry/exit, preserving selected speed. No mutation of Core to add a guide hold is authorized by this preparation.

Actual Session stores core/editor/onboarding/settings/hud_focus/recovery; presentation(),controls(),focusIndex(),focusNext(),focus(),activateFocused(),clearUiFocus(),cancel() are public, but there is no guide location, saved complete focus, guide modal owner, held-state API or immutable guide lease (`zig/src/classic_session.zig:35-64,266-338`). Proposed new `classic_guide.zig` owns pure static content/projection only; guide state and remembered focus belong later root/session integration under explicit ownership ruling, not private-field guessing. A synchronous caller provides admitted CurrentStamp and frozen input; no Session wrapper may return borrowed locals. Existing3I routing brief remains Proposed and cannot establish actual queues or atomic host publication. H2 materialization rebind and accepted H3a cross-consumer ownership await exact accepted artifacts; guide source wraps6x1.6 glyph-width geometry rather than substituting H2 metrics.

## Independent future fixtures and dispatch prerequisites

Literal expectations: title Classic // First fleet, first paragraph prefix GOAL: You are Union.; complete final graphics paragraph and controller disclaimer must survive. Independently compare all34 original paragraph literal byte spans against frozen Rust source, with explicit topic boundaries; expected corpus must not be generated by the Zig implementation. Hand fixture paragraphs `one two`, `three` at columns3 -> `one`,`two`,blank,`three`,blank; long word `abcdef` columns3 stays whole. At320x320 columns29,rows4; buttons widths70 and x8,86,164,242/y258. At640x480 columns62,rows11,width150 and x8,166,324,482/y418. Topic0/sheet0 omits Previous but Next retains slot1. Last topic last sheet omits Next, keeps Main menu/Close slots2/3. Cross-topic Previous must use preceding topic's own wrap count. Resize re-clamps sheet and computes new focus from present actions.

Future tests: independent literal content/counts; navigation all topic boundaries; source normalized semantic versus uppercase sighted lanes; finite/NaN/infinity/overflow geometry; H1 node/action focus invariants; stale owner/revision/retained-token no ABA; all allocation/retain-failure ordinals with old-frame rollback; repeated publish exact backing cleanup; actual Header and requested-population ledger before/after; no advance/camera/visual/advisory while held and no accumulated timing debt on close. Host tests later separately qualify keys, pointer, accessibility and focus restoration. Neither compilation nor tests ran here. Parent fresh review, profile ruling, exact H2/H3a rebind, root guide state/lease/focus ownership, and later3I dispatch integration remain prerequisites. No new implementation authority inferred.
