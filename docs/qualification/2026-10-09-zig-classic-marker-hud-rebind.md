# H3c accepted HUD/API rebind

2026-10-09. Proposed next dispatch preparation, no product change. Accepted H3a identity now grounds the construction seam previously Pending. Parent ChoiceB/immutable lease and local Marker<=128 policies remain accepted preparation. Pure layout worker owns current root/README changes; historical accepted root/README hashes are acceptance receipts, not live drift accusations.

## Actual accepted HUD surface and missing seam

Full classic_ui.zig read. Accepted prepareHudSettings(allocator,budget,source,current) and prepareHudSettingsFromSession(allocator,budget,session,pointer,current) synchronously return move-only PreparedProjection. input() returns Header-borrowed FrameInput; publishHudSettings(slot,published_revision,prepared,current) checks Header budget identity/captured owner+revision then replace/assign scalar; validateAction checks actual FrameView.owner and trusted scalar. Header viewport,scale,settings,help_visible,revision are private; there is no public geometry accessor. input().owner already exposes copied context/resident/hold and nodes expose actual bounds. Cannot construct revision/viewport/scale snapshot through those nodes alone (classic_ui.zig:42-115,422-447,534-581).

Proposed minimal future UI hunk:

```zig
pub const GeometryView = struct {
    revision: u64, viewport: Vec2, scale: f32,
    settings: bool, help_visible: bool,
};
pub fn geometry(self: *const PreparedProjection)
    error{EmptyProjection}!GeometryView;
```

Returns value metadata from existing Header only, no allocation/retain/hidden second Header and no private pointer export. Existing input() borrow must be taken only after geometry succeeds, under same immutable prepared lifetime. No fields added to Header for this accessor. Viewport is admitted normalized HUD viewport; scale is actual stored .60..1.50 scale, not recomputed from mutable preferences. Revision source is existing Header, never fabricated from current event. Actual prepared.input owner is the stable resident/context source. GeometryView exact target size is future measured caller payload; no stack-zero claim.

Construct snapshot ONLY from immutable pre-marker prepared HUD in synchronous adapter: geometry plus input; copy owner.context into128-byte snapshot, retain no token; borrowed resident remains alive because original prepared Header remains owner. Check draftnull,contextlen<=128, full currentowner match and capturedrevision. Original BorrowedHudSnapshot[26]Zone remains fixed local/caller storage, not stored inside UI Header. No published-only FrameView can reconstruct help viewport/scale/revision without a separately trusted geometry record; first slice uses prepared input, then integrates marker in SAME Header before publishing. This avoids a second projection Header or a source/current cycle. Snapshot never escapes prepared Header lifetime.

## Exact exclusions from actual nodes

Scan pre-marker input nodes in stable source order. For each primary node with action!=null use its bounds (missing -> InvalidHudGeometry), kind control, enabled=node.enabled. Never include label/value child duplicates as zones. Playing has17 maximum pre-marker controls including optional skip/reset_guidance; settings has11. Fixed[26]Zone allows17 control +tray +help +guidance +tooltip=21 populated records, five slack. Count before append; ZoneLimit typed rejection. Recovery Alert/selected-field/status without bounds adds no geometry zone; do not invent an opaque recovery panel (classic_ui.zig:197-234,282-329).

Exact region IDs c/hud/tray, c/guidance and text c/tooltip: take actual bounds into corresponding kinds when present, no geometric recomputation. Help has no full panel semantic region in accepted UI: explicit help_rect=Rect.fromXYWH(v.x*.5-190,v.y*.5-90,380,180) when help_visible, copied from consumesPointer policy; reset_guidance bounds are not the help panel. Settings consumes entire viewport and yields marker absent. Actual control enabled state stays independent of focusability; disabledcontrols do not consume source point, but parent ChoiceB excludes positive label/ring intersection (classic_ui.zig:555-581; src/presentation/ui.rs:175-209).

Edges use legacy s=max(geometry.scale,.72): left240*s,right280*s,top64*s,bottom102 when v.x<1100 else56*s. Actual safe bounds are left,top to v.x-right,v.y-bottom; inverted axis collapses at min+(max-min)*.5 exactly as GameLayout, finite arithmetic checked. This binds actual HUD consumption rather than deriving from mutable user scale. GameLayout's original minimum.72 and source consumed panel gaps are deliberate legacy policy (src/game/view.rs:39-84; classic_ui.zig:567-578). Safe-clamped label180x54 inset1; empty admissible interval yields absence. Enabled controls/tray/help exclude overlaps OR boundary touches; disabledcontrols/guidance/tooltip exclude positive-area overlap only. Source point consumption uses inclusive edge comparisons,enabledcontrol/tray/help; guidance/tooltip are new readability exclusions, not fabricated source consumption.

Zones derive from actual prepared nodes, which also supply published geometry after H1 copy. Future tests compare them to actual FrameView bounds after publication, while snapshot itself is not persisted into frame. Distinct recovery/announcement semantics and text policies unchanged. If HUD node naming/schema changes, adapter requires explicit matching accepted rebind, not broad role guessing.

## Acyclic future module split and callable seams

```zig
// NEW classic_start_marker.zig: pure; imports math/model/camera/scene/picking
pub fn project(input: Input) ProjectError!Result;
// NEW classic_start_marker_adapter.zig: imports pure marker + UI + Core
pub const SnapshotError = error{
    EmptyProjection, InvalidHudGeometry, ZoneLimit, SourceMismatch,
};
pub fn snapshotFromPrepared(prepared: *const ui.PreparedProjection,
    current: ui.CurrentStamp, out: *BorrowedHudSnapshot) SnapshotError!void;
pub fn projectFromCore(lease: *const CurrentLease,
    hud: *const BorrowedHudSnapshot) AdapterError!Result;
```

Input/Result/ProjectError/CurrentLease retain accepted marker adapter-binding exact types and semantics; SnapshotError is additive proposed construction union. projectFromCore does not import Session or UI in pure projector. Actual camera/Core typed values remain confined to adapter synchronous call. UI imports only pure marker result/types for later integration, never adapter. Adapter imports UI and pure marker; pure marker imports neither UI nor adapter. This prevents UI->adapter->UI cycle. No API exists until authorized module edits; proposed future ownership includes NEW pure module/tests and NEW adapter/tests, minimal UI geometry+integration hunk owned by parent-designated UI worker and minimal root/README. No protected Core/Session/camera/scene/picking edits. Sequential dispatch waits active layout work and fresh review.

Later integration signature in UI is proposed `attachStartMarker(prepared:*PreparedProjection,marker:marker.Result) PrepareError!void`, after snapshot/project and before publication. Caller forbids simultaneous source mutation; function admits zero or one marker, rejects repeated attach as InvalidSource, stores no output pointer, copies name/text/bounds and adds source typed guidance action canonical static ID + inlineWorldId. New ID mapping in validateAction must derive selected guidance ID from actual published immutable node value/closed decimal world0..6, or equivalent fully counted typed field; event cannot supply WorldId. Exact chosen published encoding remains fresh implementation binding/test obligation, not fulfilled by existing actionForId. Do not claim current accepted actionForId supports marker: it currently lists17 HUD actions with no guidance marker (`classic_ui.zig:449-461`). Source Session.UiAction.select_guidance_world and controls(guidance_world) already exist (`classic_session.zig:13,179,290`).

CurrentLease is explicit caller-held immutable Core/onboarding/stamp borrow. It is not a Core token/revision getter. Adapter compares captured snapshotrevision/currentowner/context/hold and normalizedviewport, cannot detect mutation while lease violated. Local camera=core.camera(),campaign=core.campaign(),scene=core.sceneFrame() remain live until project returns; SceneFrame owned arrays and CampaignView borrowed fleet slice never escape. Use scene.camera projection/picking and camera logicalviewport provenance check; exact accepted Core signatures verified at classic_app.zig:141-159,173,228. Numeric finite/range/ID checks do not prove matrix provenance. No host dispatch, locks or queue added.

## Count/layout/resource invariants

Accepted H3a Header33232 (review target evidence); cap36864 leaves3632 target bytes. Snapshot[26]Zone/context/metadata remains caller storage, not added Header. pure project outputMarker<=128 is futureassert, no heap/tokenretain. Proposed marker attach adds3nodes,1control,<=3 child entries at same tree, closed ID/action capability, static START HERE and source tooltip and at most24byte inline WorldName; extra typedworld/presence metadata must be explicitly counted in actual Header. Existing17->18controls,64->67 maximalPlayingnodes fit72/18/71; announcements2/recovery16384/49152text/12288ID/81920candidate/36864Header caps remain unchanged. Future actual Headerincluding every new field<=36864 mandatory; do not silently store snapshot or expand arena. Both actual requested Header and helper/node bounds enter stable budget ledger; shared1179648 policy unchanged. Target sizes and oldframe/OOM/retain rollback tests required, no source acceptance guessed from slack.

Source original marker text/tooltip/action fullcopies count separately; actual helper/header ownership and published worldencoding future fixture must literalize bytes. Immutable snapshot no retain, lease no retain, prepared Header owns one resident retain, H1 candidate owns its own successful retains. Caller slot ledger admits each reference independently. No redundant owningstamp/publicationrecord invented. UI existing publication revision scalar remains only after successful replace; marker validation requires trustedscalar+actualnodecap/currentowner and currentinitialUnion before later Sessiondispatch. No host routing accepted.

## Frozen accepted evidence and future fixtures

H3a independent reviewbb0b1e1790338ba26a04572295c79267c226ad1fde5d76ed618b36d0eb69eac7 and aggregate gateexit0/358Debug/65identities/no drift establish accepted artifact status. Rehashed stable UI/tests: both match accepted final JSON. Of61 H3a preservation entries,59 match; classic_editor.zig and classic_editor_tests.zig differ under the authorized active layout/getter worker. They are not treated as protected frozen inputs or an unauthorized drift finding here. Root/README historical final hashes are noted but intentionally not treated as liveprotected while layout owner edits. Protected G3 app/session identities taken from G3 finalmanifest and H3a preserved identities, not historic3E guess. Relevant exact live identities below:

|Source|SHA256|
|---|---|
|`zig/src/classic_ui.zig`|`26779e9ff238611fae6aaf1e786aa1b422bf6edcf823d963a6886fd4b66e4736`|
|`zig/src/classic_ui_tests.zig`|`12484dc907cc0930e205dbc8c226647eb21ca8838af6a029499d9e8245d951c2`|
|`zig/src/classic_app.zig`|`2d418ba49cdfb85d2ffd8d172cefc563b1b4cf87bb601729eb4045f791e2f1be`|
|`zig/src/classic_session.zig`|`747b27b98b15030931f214507568a9ced462121f47591b59db674c9383fd9568`|
|`zig/src/classic_camera.zig`|`ac5931069c0f65c44eaad0e093dc668ef61756a5ff19cdf5f69b02dfcbd0c268`|
|`zig/src/classic_scene.zig`|`e022c8961168b072f36cd26d0a84f9cb88bd17d904689df9d2614982c6f4ce11`|
|`zig/src/classic_picking.zig`|`e9dc79171f9f3906549642df0d53c3076a52b4427eb3a9ba375aab2541b0c219`|
|`zig/src/classic_math.zig`|`e0b5188b08ab78a4c32a348ba0398150ab2b8ac0541a8fe4984b9e4d60913186`|
|`src/presentation/ui.rs`|`b38b0b4f0c78c5de6e807116239018351d55538e2c5f2faba355828744517ae6`|
|`src/game/view.rs`|`6b5f1180edaa8e8afd84d25d85a7cdaf7c9ae2d139bbd8b4b21fd4e2dc98fc35`|

Future independent fixtures: literal control/tray/guidance/tooltip sourcebounds;helpvisible380x180 exclusion; disabledcontrol boundarytouch allowed but positiveintersection rejects;enabledtouch rejects;collapsedtiny safeabsent; all12default f32 camera/viewport/scale cases;firstsourcecandidate action_only priority;snapshot copiedcontext and no tokenretain;owner/revision/viewport mismatch;PreparedHeaderdeinit only after snapshot/project/attach borrow ends; actualHeadercap/no duplicated snapshot;two attachrejection;published worldencoding exact and forged eventworldcannotselect; all failureordinals preserve old frame/publication/source. Parent markerpolicy correction synthetic7sphere oracles retained. APIs/targettests and published worldencoding remain Pending, not rendering/host/game completion. No compiler/test/gate ran here.
