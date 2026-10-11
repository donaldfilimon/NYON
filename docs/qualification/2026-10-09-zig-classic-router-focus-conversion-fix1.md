# Proposed I3 FIX1 source/history to published focus conversion

2026-10-09. Additive closure of preparation review Important I1 ONLY. Originals immutable. No product edits/compiler/gates/agents. The review correctly distinguishes source historical focus from current H1 focus: focused must belong to active order; Settings source is nonnullable and its builder always publishes clamped index. [task-3i3-router-concrete-binding-review.md, § Important I1; zig/src/retained_semantics.zig:371; zig/src/classic_ui.zig:350]

## Exact override to prior snapshot meaning and storage

The earlier addendum's universal claim that every saved domain can publish null/history unchanged is superseded ONLY by this per-domain conversion. Two36-byte FocusSnapshot values still preserve complete historical order/modal/restore metadata. Their focused byte now means last successfully published H1 focused code, not an arbitrary source identity outside active order. Add TWO ordinary u8 source_focus bytes, current and saved-pre-guide, with255=null. Each uses its adjacent snapshot domain's finite catalog and validates independently. Existing snapshot.restore continues to represent historical modal restore; its membership in base is NOT required. No extra TokenRef or string copy, third snapshot or dynamic order array.

Known ordinary focus/context lanes become785 maximum=72 snapshots+2 source bytes+672 decoded native28-key scratch+39 context bytes, before other fields/alignment. This is an explicit2-byte correction, not a whole Coordinator sizeof measurement. The current active guide source_focus byte uses guide catalog; saved byte preserves underlying source focus even when it differs from saved.focused. Editor confirmation active focus stays snapshot.focused107/108/null while source_focus retains EditorState.focus's potentially historical base identity. [zig/src/classic_editor_ui.zig:128; zig/src/classic_editor_ui.zig:258]

Capture begins under Current source lease, reads actual source state via public read-only paths below and captures actual successful FrameSlot.view().focus separately. Do not reconstruct actual published.focused from source_focus. Reject unknown/domain-mismatched/out-of-range codes, incompatible modal structure or a source/frame mismatch beyond the explicitly supported hidden/readonly cases. No first-control default is chosen by capture. The saved manager/order is complete historical data; changed geometry is rebuilt by each actual producer rather than replaying stale bounds/order as current controls. [zig/src/retained_semantics.zig:756; src/app.rs:1637]

## Per-domain source and publication table

|Domain|Admissible source_focus|Actual H1 published focused|Guide Close source handling|
|---|---|---|---|
|HUD|null or any actual24 catalog typed action identity, including hidden marker/help/skip|matching current Header focus entry; absent source identity=>null; disabled-present controls may remain focused|Session.hud_focus untouched throughout guide; verify exact saved source identity; rebuild current HUD/marker order|
|Settings|EXACT0..10 index; null/255 or historical foreign identity IMPOSSIBLE and rejected|actual builder always clamped index; no null override|Session.settings index untouched; rebuild actual11 controls and publish same index|
|Editor without confirmation|null or well-formed actual109 catalog FocusKey from actual EditorState.focus, including historical missing/readonly identity permitted by producer|only enabled+focusable matching actual current descriptor; otherwise null|EditorState.focus untouched; FocusInput.current MUST equal actual sourceFocus; rebuild layout/order|
|Editor with confirmation|actual EditorState.focus retained as underlying source_focus; independent active modal focused107/108/null|FocusInput.current uses modal107/108/null; H1 base from saved historical28 keys, modal current actual2 controls; published.focused only matching modal capability|source EditorState.focus and modal remain untouched; restored modal snapshot.current and saved-base order/restore passed separately|
|Guide|null or one of4 exact closed IDs, including now-omitted endpoint history|prepareGuide canonical known present focus, else null|guide source removed, underlying domain resumes through above adapter; whole guide record cleared only successful leave|

This table is the accepted PRODUCER behavior plus a proposed narrow HUD canonical focus setter described below. It does not assert Rust automatically preserves null across every rebuild: Rust guide restores whole manager, then replace_active_order can select first on missing/null. Our previous blanket null/source-parity statement was too strong. The bounded Zig adapter retains actual accepted null behavior for HUD/editor/guide and actual nonnullable Settings behavior; that explicit migration policy needs parent disposition. [src/app.rs:1649; src/app.rs:1664; src/ui/accessibility.rs:346; src/ui/accessibility.rs:363]

HUD detail: capture Session.hud_focus directly (public field) with exact full typed equality against static24 catalog; Session.focusIndex is pure and returns null when hidden without clearing source. prepareHudSettingsFromSession projects only matching nonmarker controls and never mutates Session. Existing HUD builder adds EVERY control to focus order even if disabled, so 'eligible' here means generated focus entry, NOT editor enabled-filter semantics. Missing hidden source identity becomes published null while source_focus is retained. Disabled generated identity may be focused but validateAction rejects activation with DisabledAction. [zig/src/classic_session.zig:41; zig/src/classic_session.zig:293; zig/src/classic_ui.zig:198; zig/src/classic_ui.zig:468; zig/src/classic_ui.zig:384]

Settings detail: capture `session.settings.focusIndex()` only; ordinary public State.focus clamps0..10, next/previous wrap11. Source null cannot be restored or invented. Validate saved source code0..10 before guide Open; Settings saved.focused must equal same canonical index in actual displayed11-entry H1 order. If a synthetic snapshot claims null or absent index for Settings, reject InvalidFocusSnapshot before input clearing/guide mutation/ref acquisition. No new Settings nullable field/API or frame override. If current preferences change while guide open through a separately permitted current-source operation, index identity remains stable while action boolean payload must be freshly resolved from current capability. [zig/src/classic_settings.zig:13; zig/src/classic_ui.zig:325; zig/src/classic_ui.zig:350; zig/src/classic_ui.zig:399]

Editor detail: source is `session.editor.session` optional owned EditorState; read `.focus` or its accepted producer sourceFocus mapping, public fields without mutation. prepareEditorFromSession internally resolves actual sourceFocus; no-modal FocusInput.current must exactly equal it even when key absent/readonly. Producer preserves remembered source focus but sets H1 focused only when current eligible descriptor matches; do NOT replace source current with null merely to satisfy H1. Thus a synthetic well-formed readonly tick_hz source focus may be kept remembered/source but H1 focused null. Normal focusField cannot establish readonly focus; tests must label this as a valid producer historical/synthetic source observation, not normal user action. [zig/src/classic_editor_transactions.zig:14; zig/src/classic_editor_ui.zig:128; zig/src/classic_editor_ui.zig:258; zig/src/classic_editor_ui.zig:320; zig/src/classic_editor.zig:231]

Confirmation detail: on actual modal opening, capture the base order/current source BEFORE request into snapshot.base and snapshot.restore. After modal opens use current modal focus separately; never overwrite EditorState.focus with dismiss/confirm merely to represent active platform focus, since focusAction refuses changes under modal. Historical restore may be readonly, missing or even another valid catalog code per current producer; preserve it while base entries remain unique<=28 and<107. During guide, save the WHOLE already-modal snapshot and underlying source_focus. On Close after origin/full-owner match, construct one temporary FocusInput with `.current` decoded saved active modal focused and `.saved_base` referencing decoded saved base/restore. Producer emits same modal2, no guide action auto-dismisses it. H1 restore may lie outside base; H1 focused may NEVER lie outside actual modal. Current confirmation activation still requires current active modal capability and full revision/owner. [zig/src/classic_editor.zig:131; zig/src/classic_editor_ui.zig:263; zig/src/classic_editor_ui.zig:266; zig/src/classic_editor_ui.zig:309; zig/src/retained_semantics.zig:371]

Guide detail: capture canonical closed source ID or null separately from actual last published focus. prepareGuide accepts well-formed unknown/omitted focus and canonicalizes publication to null; router itself rejects unknown source catalog but preserves an omitted endpoint code as source history, with published null. Newly present endpoint can regain focused after geometry/location change ONLY through exact canonical present-match conversion at new revision, not from old pixels. Guide navigation starts from actual current published active-order focus, not hidden remembered source code; a source focus null event remains null unless a real navigation/focus operation changes it. [task-3h3d-guide-report.md:11; zig/src/classic_guide.zig:59; zig/src/classic_guide.zig:268]

## Narrow proposed HUD producer seam, needed for marker focus only

Actual prepareHudSettingsFromSession calls session.controls(null) and loses hidden marker source focus from ProjectionSource.hud_focus; attachStartMarker appends marker action to Header focus order but does not set Header.focused. Therefore publishing a newly present saved marker needs a reviewed tiny producer seam. Existing methods do NOT already provide it. [zig/src/classic_ui.zig:474; zig/src/classic_ui.zig:508; zig/src/classic_ui.zig:704]

Proposed ONLY additional product scope: `zig/src/classic_ui.zig` PreparedProjection method and paired UI tests, parent separately authorizes its edit after current sole mutator ownership release:

```zig
pub const SetHudFocusError = error{
    EmptyProjection,WrongProjectionMode,AlreadyPublished,InvalidUtf8,InvalidFocus,
};
pub fn setHudPublishedFocus(self:*PreparedProjection,
    focused:?retained.ActionId) SetHudFocusError!void;
```

Caller ends all immutable Header/input/view borrows before call. Order: require nonempty Header; require playing projection (`!settings`); reject already published; validate supplied optional full UTF8; if nonnull scan actual Header.focus, require exact full-byte match and recover canonical existing Header ActionId; otherwise InvalidFocus. Only AFTER all checks set h.focused to found canonical entry or null. No caller bytes escape, allocation, retained refs, node text/geometry mutation, focus order rewrite, speed change or extra budget request. Repeated successful call before publication permitted, failure leaves every Header field unchanged. It does not bypass disabled control activation, because Header focus includes disabled rows and validateAction still rejects disabled. Existing H1 membership/UTF8 validation remains authoritative second check. [zig/src/classic_ui.zig:100; zig/src/classic_ui.zig:198; zig/src/classic_ui.zig:373]

Router calls after prepareHudSettingsFromSession AND optional marker attachment, before publication: match saved/current source catalog identity against actual Header.focus; choose canonical present ID or null, then invoke setHudPublishedFocus. This covers marker presence and faithfully preserves ordinary hidden HUD behavior. It is NOT called for Settings. No generic setFocus-anywhere override, mutable Header escape, private field access, additional Session API or Settings null representation is proposed.

The method has zero extra heap/requested-byte budget; self retains existing Header charge, Header shape unchanged, stack uses one optional ActionId scalar. Existing publication Header snapshot/old frame/published revision rollback unchanged. Actual new method target size and synthetic allocator-zero-call witness still need implementation evidence; proposal is not proof.

## Exact capture, current and successful publication ordering

1. Capture source_focus from actual Session domain and actual current FrameView.focus metadata under immutable lease. Validate each per-domain constraint including Settingsnonnull and H1 membership. Copy complete historical FocusSnapshot order/modal/restore and nullable actual frame focused. Save origin owner/ref pair only after storage/UTF8/registry/nextrevision admission. Source state remains untouched by guide Open/Show/Close; InputState and contacts clearing remains Open-only.
2. While guide present, Session source HUD/settings/editor focus is frozen by mutation lease. Guide owns independent source_focus and published focus. Do not synchronize guide focus into Session.hud_focus, settings index or EditorState.focus. Attempts to mutate underlying source through unsupported caller paths fail SourceChanged/InvalidContext and invalidate restoration; do not repair by overwriting Session from old snapshot.
3. Close restores coordinator history by copying saved source_focus/snapshot into proposed base source state, but performs NO write to Session focus fields because they never changed. Verify actual source value equals saved source value and current base owner matches origin before any projection restore. If mismatch, reject SourceChanged and keep explicit stale/noninteractive state; do not force saved source into replacement Session. Source null for HUD/editor is preserved; Settingsnull is preadmission error.
4. Rebuild actual destination producer order using current geometry/source. HUD optional marker then proposed canonical setter; Settings existing producer unchanged; editor exact FocusInput mapping above. Never pass snapshot.focused historical raw code directly into H1 FrameInput. H1 obtains only producer's canonical current active focus. Historical base order is replayed only where actual editor confirmation producer explicitly expects saved_base; otherwise actual rebuilt order replaces it for current publication.
5. On successful actual publication, update live published snapshot from actual new FrameSlot.view().focus and retain source_focus separately. Retire saved guide snapshot/ref pair only then. If prepare/publication fails, old displayed backing/focus/publication unchanged; current intended source/history stays in Stale and retryable saved metadata remains. A null old frame-focused does not make source_focus null; an old nonnull frame-focused does not guarantee new capability membership.

Keyboard Activate and semantic Focus/Activate use actual CURRENT displayed FrameView.focus/nodes and exact fresh owner/revision, not historical source code alone. For HUD a disabled focused node does not authorize activation; for Settings use index only after canonical current row capability admission; for editor enforce modal and readonly checks; guide uses actual enabled present canonical controls. Existing Session.activateFocused can mutate source HUD focus on absent/disabled cases, so router must not call it as an unguarded restoration/projection helper. Actual source effects from a deliberately admitted user activation remain existing Session behavior, followed by appropriate revision. [zig/src/classic_session.zig:319; zig/src/classic_ui.zig:384; zig/src/classic_guide.zig:389]

## Independent FIX1 fixture matrix

|Domain witness|Source/history expected|Published H1 focus expected|
|---|---|---|
|HUD hidden reset_guidance or skip disappears|Saved source code and Session.hud_focus unchanged through guide|null if absent; H1 publication succeeds without InvalidFocus|
|HUD disabled-present launch|Source identity retained, base includes row|launch may be focused; activation DisabledAction, no command|
|HUD marker world3 absent/present after rebuild|Full world3 typed identity preserved|absent=>null; actual marker3 appended=>canonical marker3 via new seam; never marker4|
|HUD source null|source_focus255 stays255|null after guide close, no first-control fallback|
|Settings indexes0/5/10|nonnullable source index unchanged|same exact clamped canonical row, boolean payload freshly derived|
|Settings synthetic snapshot focused/null mismatch or source255|InvalidFocusSnapshot BEFORE Open mutation/refs|existing frame/source/input/contacts unchanged; no nullable Settings override|
|Editor valid missing old-world field / readonly tick_hz synthetic / disabled Apply|null not substituted for actual source_focus; FocusInput.current equals actual source|null if not eligible in rebuilt order; remembered identity intact|
|Editor source null|source null unchanged|null; no auto-first focus|
|Editor confirmation under guide with historical restore outside saved base|null/history retained in source/restore, active modal codes separately saved|modal107/108/null only; base historic<=28, modal2; no underlay focus or auto-confirm|
|Guide omitted Previous/Next remembered ID then page changed|closed source code preserved|omitted=>null, newly emitted matching ID=>canonical focused; unknown routercode rejects|
|HUD setter failure order|wrongmode/settings/already-published/invalidUTF8/absentID|Header unchanged, no allocation/refs; existing publication rollback unchanged|
|Every close prepare/OOM/publication failure then retry|saved source+full focus/ref pair remain retryable|old frame stays stale/noninteractive; successful retry canonicalizes actual order and retires saved pair once|

Expected orders/IDs authored independently from source tables; inspect actual published FocusSpec and source values separately. Required H1 hidden-id rejection fixture proves direct raw history reuse would fail at371, while producer conversion produces valid null. New HUD seam tests include canonical borrowed temporary poisoning, disabled-present focus accepted without activation, marker exact-payload recovery and all failure modes with Header byte/state preservation. No new allocation trace permitted. Existing all-mode source/view/owner lifetime matrices and budget profile stay unchanged except2 ordinary source-code bytes.

## Disposition and frozen anchors

Proposed I1 closure, fresh review required. Original universal focus-null wording is superseded by precise source/H1 separation, Settingsnonnull rejection, actual editor/guide canonicalization and explicit proposed HUD marker seam. No source implementation or fixture/gate receipt is claimed. Final accepted foundation/interaction API/layout rebind and wider parent policy/host acceptance remain unchanged prerequisites.

|Source/document|SHA256|
|---|---|
| `.superpowers/sdd/2026-10-09-nyon-zig-migration/task-3i3-router-concrete-binding-review.md` | `529fe0c7a4779e512b98b0b6666b23ea2b39b4d8e7769dbb79dcb691d3ad734d` |
| `.superpowers/sdd/2026-10-09-nyon-zig-migration/task-3i3-router-concrete-focus-owner-addendum.md` | `c472da7e73e70ec36de96eb4117d26869f3aeda24641d89327771429b48769b5` |
| `zig/src/retained_semantics.zig` | `7e68e9b83d5b5da7e145a02bb8d2c04968558e22c307535b637cade597a735cb` |
| `zig/src/classic_ui.zig` | `bc97e432bee3cf3c560404823fb53ad9464b5086304867b06d29ee07bca4515d` |
| `zig/src/classic_session.zig` | `747b27b98b15030931f214507568a9ced462121f47591b59db674c9383fd9568` |
| `zig/src/classic_settings.zig` | `f66701119b9fb9870a39aa3dea225d9b705c8711dceaf8aa2ee3437a003efd11` |
| `zig/src/classic_editor_ui.zig` | `829f33d96d78e30e27d34fcce64e59e08096ea2b9aa0dbdd1a4bb43980cf794b` |
| `zig/src/classic_editor.zig` | `aea58f15ff8e5a4291e9e8b3692ca6068813fb333525fe29e157f073ac2de26a` |
| `zig/src/classic_editor_transactions.zig` | `e4776d63c6f81c8d33371ecf5ee7db5ee59b85095226869f304ef2c0a20d3913` |
| `zig/src/classic_guide.zig` | `f0bd20166c672821e875ee2b7f35fd698679b060ff9ee6237b6df7bff359742d` |
| `src/app.rs` | `67b489723ec3886b38873f12ac6a21e763d0f6fc6d21c709b85fd27730eaaea2` |
| `src/ui/accessibility.rs` | `526c2caa9beaf587a5afe3f680ad649436759899e4534fe6bf680c6b88849b54` |
