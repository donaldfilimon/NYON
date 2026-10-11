# NYON in Zig

The game migration is in progress on Zig `0.18.0-dev.120+9fe22a29b`.
This directory currently contains the Classic RulesV1 authority and terminal
interaction. Full game scope and outstanding acceptance are tracked in
[`2026-10-09-zig-migration.md`](../docs/qualification/2026-10-09-zig-migration.md).

The headless `classic_clock` and `classic_app` modules provide the Task3A
Classic session lifecycle. `FixedClock.init` explicitly rejects a zero tick
rate. Its rational nanosecond clock clamps each pass to 250 ms and drops whole
work beyond capacity while retaining fractional residue. `AppCore.advance`
pushes and drains at most 15 steps separately for each 0/1/2/4 speed pass.
At 60 Hz, 100 ms yields 0/6/12/24 steps; a long frame yields 0/15/30/60.

`AppCore.init` accepts a checked scenario and explicit preferences value;
preference loading remains the caller's responsibility. Default preferences
start paused under an onboarding hold. The Task3E session below drives the explicit hold seam
from its six-step onboarding owner. Mode changes and suspension/resumption clear clock residue;
Settings/Editor holds retain selected/resume speed. Preferences remain live
values across restart and replacement.

Each session owns one simulation and must be deinitialized exactly once.
Campaign projections copy worlds and borrow a read-only fleet slice until the
next session mutation; scenario projections borrow the session. Frame outcomes
contain successful-step counts and inline tick/material summaries. Every owned
tick report is released inside `advance`. Tick allocation failure retains prior
successful ticks, preserves the failing tick's authority/queue, clears the
clock and enters a sticky recoverable failure hold. Speed changes and later
frames cannot retry; explicit `recover` permits new clock work, while a successful
restart clears the failure and resets session state. Finished authority reports
zero further transitioned steps.

Restart/replacement prepare checked inline scenario/simulation values before
releasing resident allocations; invalid or stale candidates preserve the live
session. This preparation currently allocates nothing, so exhaustion cannot
exercise a preparation allocation failure. Camera/advisory/focus integration,
graphical onboarding, graphical hosts and player journeys remain later slices.
The terminal currently does not use this session module.

The headless Task3B `classic_commands` module routes explicit Classic intent
using resident world IDs. Selection, hover and mouse/touch transitions never
queue gameplay; unknown picking IDs return `InvalidWorld` after their mode/HUD
gates. Keyboard launch uses hover only; contextual launch uses hover before a
latched touch target. The standalone functions accept a session pointer for
the call and retain no state or pointers back into movable `AppCore`.

Tray and field controls are immutable, allocation-free projections. Names are
nullable inline `WorldName` copies, launch strength is a nullable number, and
status/tooltip text is static. Later UI displays missing names as `NONE` and
strength as `-`, formatting numbers in caller-owned storage. Enabled controls
reflect authority preview; direct command dispatch still permits invalid
intent for canonical tick rejection.

`AppCore.enqueueGameCommand` is the single Playing-only queue seam. It returns
consumed sequence and preview validity; actual enqueue errors and mode refusal
remain typed. Sequence wraps only after success, with no collision skipping or
automatic retry. Pause, onboarding hold and tick failure hold do not block queue
admission. Controller dispatch returns value effects for focus clearing and
Union selection, preview, fleet-launch or field-tune observations. Launch/tune
observations mean preview-valid successful enqueue, not later tick acceptance.
Right-click updates hover and clears its touch latch before enqueue, preserving
those changes on failure. Task3E below consumes these effects for headless
focus/onboarding; picking geometry remains a later owner.

The headless Task3C `classic_editor` owns independent ORIGINAL and current drafts,
90 exact field buffers (89 editable), parse flags, focus/section/world values,
visual IME preedit and one modal capture capsule. It accepts semantic-invalid
UTF-8 drafts; editable numeric grammar matches Rust plus/leading-zero behavior,
while candidate extraction checks invalid text before semantic validation.
Names remain arbitrary valid UTF-8 until canonical scenario admission. Successful
edits normalize all non-error fields; invalid spelling retains its last typed value.
Backspace removes one Unicode scalar; IME commit appends and clears visual preedit.

Editors and modal save captures are move-only owners by convention: move them,
then invalidate the source; deinit exactly once. Their base allocator context must
remain stable. Each content edit stages a fresh current arena before publishing
and releasing the previous epoch. Append additionally owns a bounded temporary
concatenated string, freed on success or failure. ORIGINAL name slices live until
deinit; current names, buffers and preedit expire after a successful content edit
or replacement. Draft projections exclude allocator/arena handles. `copyOriginal`
and `copyCurrent` return independently owned drafts for caller preparation.

`replaceCurrent` is an atomic low-level publication seam for later transactions:
it rebuilds buffers and clears parse flags while preserving ORIGINAL, focus,
section/world, scroll, preedit and modal captures; it does not validate gameplay
semantics. The modal owns independently allocated save bytes and inline scenario
captures. Dismissal frees/clears both. Modal guards block edits and selection;
focused replacement dismisses a modal without consuming or applying its text.
Real widgets/action admission, tab traversal and layout remain later slices.
Store requests, confirmation outcomes and active authority installation are composed
by Task3D below. This editor never stores an AppCore, Simulation or store pointer,
nor queues gameplay.

The headless Task3D `classic_editor_transactions.Transactions` owns an optional
editor and typed status, using the editor's single modal. Call `open(core)` and
`dispatch(core, raw_store, action)` with per-call borrowed owners. No core/store
pointer or second mode flag is retained. The concrete store and allocator context
must outlive their calls and allocations; rebuild `store.raw()` after moving the
concrete store. Move each transaction once and invalidate its old variable, then
`deinit` exactly once. Status borrows remain valid until the next status change
or deinit; storage error messages are owned by the transaction allocator.

Outcomes distinguish unchanged, opened, modal opened, absence, loaded, saved,
replaced, closed, applied and failed. A failed outcome uses two distinct channels:
a non-null `action_failure` owns
the latest action error (invalid text, semantic Save codec validation, Apply
validation, confirmation or admission); otherwise `status.failure` owns the latest
load/probe/write error. Store status survives unrelated action errors. New store
failures clear any previous action error, so an older error cannot hide the new
outcome. Action errors are inline/static values without diagnostic allocations;
status owns storage messages and inline detailed codec reports. Successful action
outcomes clear `action_failure`, while reset/regenerate retain store status.
`OutOfMemory` remains an error and
`ModeSessionMismatch` rejects inconsistent core mode/session presence. Success
and absence status are allocation-free tags; display formatting belongs to callers.

Dirty load requests open a modal without reading. Confirmation reads the then
current raw slot, decodes with detailed errors, validates semantics and stages all
current buffers before replacement. Save rejects invalid text before semantic
validation/encoding and probes occupancy only after encoding. A returned payload
means occupied, including empty or malformed MemoryStore bytes. Native oversized,
invalid UTF-8 and I/O failures block overwrite. Confirmation writes independently
captured bytes without reencoding, rereading or compare-and-swap. Loading/saving
never change ORIGINAL or active authority and do not establish editor cleanliness.

Every confirmation takes its capsule before fallible work. Failure, including OOM,
consumes it and requires a new request. Request OOM preserves content, both
diagnostic channels and modal. This intentionally differs from standalone editor edits, whose OOM retains
the capsule. Revert/default/regenerate always confirm, stage a complete replacement
and keep ORIGINAL, presentation, preedit and prior store status. Regeneration uses
last typed seed/rules even when raw spelling or rules are invalid, so semantic
errors can remain after canonical buffers replace invalid spelling. Direct Cancel
can replace an active modal, releasing displaced captures immediately.

Open stages an editor from the active initial scenario before entering Editing.
Close releases it, enters Playing and clears recoverable failure while preserving
campaign/queue/selections/speed/preferences. Apply freshly validates and compares
all meaningful inline scenario values with a safely bounded capture before
allocation-free `AppCore.replaceScenario`. It drops the editor only after successful
installation. Lifecycle `effects` are values for future camera/focus/Restart-advisory
owners; read them with opened/closed/applied outcomes. No graphical integration is
claimed. Effects remain stored until the next successful lifecycle transition;
callers consume them only with the matching returned outcome. Open/close focus
handoff must preserve the stored HUD action identity; successful apply and
ordinary restart require a HUD reset. Task3E below owns that HUD reset and returns
camera/context-focus/advisory handoff values to later owners. Ordinary restart
uses the newly active scenario.

The headless Task3E `classic_session.Session` owns one AppCore, Transactions,
six-step `classic_onboarding.State`, bounded `classic_settings.State`, typed HUD
action identity and independent preference/general/operation error channels.
It retains no store or interior owner pointer: construct `raw()` at the concrete
store's current address for each call. Move the session once, invalidate its
old variable and deinitialize it exactly once; allocator contexts must remain
stable. Stored preference and editor storage messages have separate owners.
Borrow recovery pointers and editor status/action-failure fields only until the
owning channel's next mutation or session deinit; never copy and free them twice.

Startup loads/defaults preferences without saving or repairing. Incomplete
guidance holds Paused with resume Normal. Matching immediate tap/preview/valid
enqueue intents advance its first four steps before any authority tick; advisory
and Scenario actions supply its last two observations. Intermediate progress is
not durable. Scenario completion updates live preferences/releases the hold,
attempts persistence, then prepares the editor. Direct `openEditor()` does not
observe guidance. Completion, skip and reset preserve campaign and queued intent.
Public reset clears help; Settings reset retains help and mode. Restart/apply
clear HUD only after successful installation and retain guidance/preferences.
Editor open/close preserve HUD identity and forward context-focus effects.

`Result.handled` reports input consumption, not persistence/editor success.
Check `persistence`, `editor_outcome`, `editor_error` and `command_error` together.
Preference persistence OOM still attempts editor opening; completion/live changes
remain visible if one or both operations fail. Earlier owned preference failure
survives OOM until a replacement is ready. Failed save's default preference
outcome never replaces changed live values. Every accepted non-Close Settings
action saves immediately, including same-value choices; Close causes no save.
Successful saves clear only preference failure. General recovery takes display
precedence without erasing preference failure; resource errors remain separate.
Inert editor dispatch preserves a prior resource error until an actual retry.
No allocation for diagnostic formatting occurs after a durable write.

`controls(marker_id)` returns at most 18 ordered Playing controls or exactly 11
Settings controls, without geometry. The later scene/widget owner supplies any
marker ID. `Controls.items()` borrows the returned value; retain that value while
using its slice. HUD identity includes full field/world/settings payloads;
lookup/rebuild is pure, navigation includes disabled controls and hidden identity
survives until explicit navigation/failed activation. Settings keeps its clamped,
wrapping index as toggle payloads change. Direct UI clear-selection is ungated;
the standalone focus-clear helper and keyboard Cancel are Playing-only.

At the historical Task 3E boundary, presentation and camera effects were
handoff values. Task 3G3 now installs the session's effective presentation in
AppCore, uses its motion preference for visual-time advancement, and implements
AppCore camera motion/lifecycle and Session camera reset. Context-focus and
advisory routing, raw host input, actual widgets/accessibility, guide routing and
graphical/native/browser acceptance remain Pending; see Task 3G3 below.

```sh
./tools/check.sh
zig build
zig build run -- play
zig build run -- trace 4947573120260902 12000 scripted
```

Terminal commands are `state`, `step [TICKS]`, `launch SOURCE TARGET`,
`tune WORLD atmosphere|hydrosphere|topology increase|decrease`, and `quit`.
Launch/tune validate and queue for the current tick; `step` executes commands
in sequence order. Each state reports the canonical fingerprint. This is an
authority interaction slice; full visual game/runtime and durable game storage
are still being ported.

The gate enforces the exact compiler and uses disposable caches to avoid the
host's missing shared Zig archive. It runs Debug/ReleaseSafe tests and compares
per-tick digests, ordered command results and event payloads with the existing
Rust implementation, including frozen genesis digest `67D96E983D6C9330`.
No third-party Zig production dependencies are introduced. Rust remains the
comparison oracle during migration.

`Simulation` owns its campaign fleet and pending-command allocations;
`deinit()` releases both. A returned `TickReport` must be released with the same
allocator. A failed tick allocation preserves authority and queued commands;
all report/launch capacity is reserved before simulation mutation. Generated
world labels are owned values, and integer rates retain exact remainders.

Classic native persistence is available through four explicit migration probes:

```sh
zig build run -- scenario-store-load /absolute/owned/profile
zig build run -- scenario-store-save /absolute/owned/profile "$SCENARIO_JSON"
zig build run -- preferences-store-load /absolute/owned/profile
zig build run -- preferences-store-save /absolute/owned/profile "$PREFERENCES_JSON"
```

The profile argument is mandatory, nonempty and absolute. Probes use only that
profile's `scenario-v1.json` and `preferences-v1.json`, disable legacy lookup,
and perform no application-data environment lookup. Save is an explicit
replacement request. These probes do not load or restart terminal play; editor
draft loading and overwrite confirmation remain separate runtime work.

Output is tab-delimited. Scenario save and validated load emit the existing
`OK<TAB>SCENARIO_FINGERPRINT_HEX<TAB>CANONICAL_JSON` codec form. Scenario absence
prints `ABSENT` and creates nothing. Preference success and absent load emit
`OK<TAB>CANONICAL_JSON`; an absent load returns defaults without writing them.
Recoverable preference load failure emits exactly one `DEFAULT<TAB>STORE` or
`DEFAULT<TAB>CODEC` diagnostic followed by the default preferences' `OK` record,
returns zero and never saves/copies defaults. That `OK` describes the resulting
preferences, while `DEFAULT` records the failed stored input.

Operational saves and scenario-load failures emit one `ERR` diagnostic, flush
stdout and exit nonzero. Invalid arguments/profile use `ARGUMENT`/`PROFILE`;
resource exhaustion uses `RESOURCE<TAB>OutOfMemory`. `STORE` retains failure
kind, record, cause, byte limit, secondary cleanup cause and available message.
`CODEC` retains class plus parser cause, canonical-number field/kind, validation
report entries, byte limit or unsupported UI-scale value as applicable.
`ADMISSION` retains the checked authority error. Diagnostics do not dump invalid
record bytes. Invalid saves are rejected before mutation and retain both record
namespaces. Success output is encoded before saving; output I/O failure after a
successful durable write may still prevent delivery and cannot undo that write.
Existing trace, play and codec-only output forms remain unchanged.

`classic_store_load.load` returns an owned detached candidate or typed
absence/failure, without touching a resident simulation. Its scenario is freshly
decoded and its simulation passes checked admission. Raw adapters borrow their
concrete store at a stable address for the call. Release loader outcomes with
the operation allocator; a moved candidate simulation must be released exactly
once with its own `deinit`. Concrete stores release their own paths with their
constructor allocator. This candidate loader does not define editor draft policy.

The gate includes stdlib-only `tools/check-classic-store-cli.py`, which builds a
fresh pinned ReleaseSafe binary and qualifies save/load/reopen, invalid input,
exact byte caps, profile rejection, typed defaults, durable-value preservation
and real persist failure cleanup in owned temporary profiles. Native process
readback proves neither parent-directory crash durability nor browser storage,
GUI apply, or Windows/Linux filesystem runtime acceptance.

The headless Task3F1 `classic_advisory_types`, `classic_advisory_features` and
`classic_advisory_model` modules provide synchronous, allocation-free Classic
advisory extraction and CPU scoring. `Observation` copies next tick, seven worlds
and active hazard; its complete const fleet slice borrows authority only for the
call. Construct it from one current campaign projection and never retain it across
authority mutation, move, restart or deinit. All features, packed weights and
scores return owned fixed arrays; no temporary-array slice escapes.

The twelve features and 4-hidden-unit model preserve the Rust source's literal
normalization, saturating all-fleet inbound sums, widened squared distances and
strict sequential f32 arithmetic. Priority retains the lowest numeric ID on ties.
`scoresValid` checks finite inclusive 0..1 scores; `parityMatches` separately checks
finite pairwise tolerance and permits a different priority in a near tie.
Local Debug, ReleaseSafe and ReleaseFast checks reproduce the seven existing
Rust factory score bit witnesses. Historical witness target/compiler provenance
and cross-target behavior remain unqualified. Task3F2 now supplies the pure controller lifecycle below. Per-step session
integration, GPU and graphical acceptance remain later slices.


`classic_advisory_controller.Controller` owns fixed request/snapshot copies, one
flight and one overwriteable newest queue. `requestIfDue` publishes synchronous
CPU scores; Tick admission uses cadence 30 and latest requested tick. Matching
GPU faults precede newest-request checks and disable that epoch with latest CPU
fallback. Same-epoch availability alone retains flight/queue and permits an
already matching latest valid completion; a different epoch retires flight/queue
without resetting IDs or historical publication. `published` borrows the owner
until mutation, move or destruction; request/completion outcomes are owned values.
Device errors are typed host inputs, not real device execution evidence.

Task3F3 integrates the by-value Classic advisory controller into `AppCore`.
Initialization publishes CPU scores synchronously at unavailable epoch 0. Each
successful authority step schedules from its actual post-state and canonical
fingerprint: accepted commands/events request material refresh; other steps use
the 30-tick cadence. Failed steps publish nothing, retaining earlier publications
and the existing sticky failure hold. Successful scenario installation requests
Restart once while retaining controller request history; Session consumes the
advisory lifecycle effect rather than forwarding a duplicate restart.

`advisorySnapshot` borrows until Core mutation/move/destruction; host requests
returned by `takeGpuRequest` are owned fixed-array values. `setGpuEpoch`,
`completeGpu` and `failDeviceEpoch` bind the pure correlation/fallback controller,
with one pending host dispatch. `classic_advisory_projection.project` copies
score/priority/name/source metadata and source backend/state labels. Name lookup
searches World.id and owns inline WorldName storage; missing IDs use UNKNOWN.
Numeric scores remain typed; percent formatting/layout belong to Task3H.
These are headless CPU/controller/projection contracts. Actual GPU adapters,
renderer/panel readability, host/manual/a11y and full-game qualification remain
pending.


Task 3G1 adds allocation-free Classic presentation values in `classic_math`,
`classic_camera`, and `classic_picking`: column-stored scalar f32 math, camera
controls/smoothing in logical pixels using the existing motion preference, and
geometry-only ray/world picking with HUD precedence. The reviewed unchecked
cofactor inverse preserves exceptional arithmetic; finite output alone is not
proof of invertibility. Analytic fixtures qualify named numerical tolerances,
not native SIMD or cross-target transcendental bit identity. Scene composition,
AppCore/Session camera lifecycle, layout/input routing, GPU uniform packing,
DPR/browser/native rendering and manual acceptance remain outside this slice.


Task 3G2 adds synchronous allocation-free `classic_scene.buildScene`, producing
seven copied worlds and populated const prefixes of fixed 512-fleet/256-route
storage. `SceneCampaign` field-copies `CampaignView.worlds`, `fleets`, and
`active_hazard`; its fleet borrow expires at authority mutation/restart/move/deinit.
Finish extraction synchronously. Frames retain no input pointers; borrow output
prefixes only from a live stable frame and never inspect undefined storage tails.
Stable bounded selection scans all input fleets before endpoint filtering, keeps
the lowest 512 IDs with input-order ties, and preserves first-encounter route
control. Seven endpoint indexes and three factions admit at most 147 route keys,
so the preserved 256-route capacity branch cannot saturate under these inputs.
`SceneAdvisory.fromSnapshot` copies accepted F3 `priority` and `scores[priority]`;
F3 retains publication/freshness admission. Scene seed uses the scenario visual
fingerprint, independently from advisory canonical source metadata. Current camera
matrices are copied using accepted G1 scalar math. This headless slice does not
qualify AppCore scene/camera lifecycle, render/upload/ABI, layout/input/DPR,
GPU/native/browser/manual/accessibility or performance; those remain later work.

### Classic camera and scene composition (Task3G3)

`AppCore` owns the camera, logical viewport and effective presentation value.
`Session.presentation()` reads that same value; transient changes apply
immediately, onboarding completion retains them, and applied settings replace
them before persistence, including failed persistence. Durable `setPreferences`
is separate from effective `setPresentationPreferences`. Frames update camera
and uncapped raw visual time once before holds, failure or speed passes. Camera
smoothing alone caps elapsed time at 250 ms; Reduced preserves raw time and
emits zero scene time. `sceneFrame` returns owned populated values extracted
synchronously from authority and the published advisory; scene seed derives
from the active scenario fingerprint, while advisory metadata retains campaign
fingerprint provenance. `pickSceneWorld` uses current camera matrices in logical
pixels and honors the headless HUD-consumed flag.

Camera reset has one producer owner: `AppCore.replaceScenario`/`restart` consumes
the installation reset, and `Session.handleUiAction(.reset_camera)` consumes UI
reset. Session returned `Effects.reset_camera` is informational about completed
work and must not be replayed by a host. Applied editor reset/advisory flags are
cleared after the installed core consumes them. Open/close editor does not reset
camera. Reset restores only yaw/pitch/distance targets, preserving target
position, viewport, effective motion and Full current state; installation also
clears raw time. Source call sites establish invocation ownership; idempotent
final-state assertions alone are not invocation-count proof.

Headless tests cover owned frames and moved owners, opposing motion overrides,
holds and actual tick-allocation failures, direct authority parity and queued
command isolation. Retained HUD geometry/default-camera safe area, rendered
native/browser/DPR behavior, accessibility and manual acceptance remain later
slices.

### Retained geometry foundation (Task 3H1a)

`retained_geometry.Rect` provides finite validated logical-pixel XYWH/bounds
construction, inclusive point/rectangle containment, positive-area intersection
and overlap strictly greater than f32(0.01) on both axes. Negative XYWH extents
clamp to zero; nonfinite inputs, derived-max overflow and inverted bounds have
explicit errors. Direct struct construction requires finite ordered bounds;
external frame intake must validate with `fromBounds`. This pure allocation-free
foundation supplies no scale/DPR policy, semantic ownership, text, layout or host
rendering. Focused geometry verification and parent integration acceptance are
separate qualifications.

### Retained tokens and requested-byte ledger (Task 3H1b1)

`retained_semantics.TokenRef` owns a UI-thread allocation identity with explicit
retain/release; ordinary value copies borrow. `LiveTokenBudget` bounds actual
live token objects and their requested allocation bytes. `PreparationBudget`
separately admits pending reservations and reports actual requested bytes across
six frame/focus/scratch populations, with historical actual high-water. Reserve
before allocation, cancel on OOM, commit on success, free before charge release.
Zero-byte receipts still carry lifetime obligations. Reclassification changes
population without allocating or charging twice. Budget addresses and allocator
contexts must remain stable until every associated owner has retired.

`OwnerSpec` compares borrowed token identities, optional hold/draft and exact
context bytes; it does not own or validate context. Receipt ownership is move-only
by convention. Zig does not enforce private struct fields or prevent deliberately
reconstructed receipts; callers must use methods and invalidate moved variables.
Admission overflow returns `LimitExceeded` unchanged. Inactive consumption,
lifecycle underflow/overflow and nonquiescent deinit use unconditional contract
panic in all optimization modes. Opaque token storage has
no public refcount setter. This slice supplies no focus/frame ownership, queues,
UTF-8 intake, host accessibility or numeric consumer admission policy. Focused
three-mode evidence and parent integration acceptance remain separate.

### Retained candidate focus (Task 3H1b2)

`retained_semantics.OwnedFocus` copies exact validated UTF-8 IDs, preserves stable
first-occurrence order and imports complete `FocusSpec` state, including null
focus and historical modal restore. `fresh` selects the first base action.
Requests, traversal and modal close allocate nothing; fallible scope replacements
stage complete owned storage before publication and preserve old views on failure.
Modal close retains charged retired storage until the next compact rebuild.

Each helper stores eight explicit `Limits` fields and a stable caller-owned
`PreparationBudget`. Actual scratch and payload requests share that ledger;
helper admission includes old/new/scratch overlap and simultaneous copied ID
bytes. No numeric consumer budgets are supplied. Ownership moves by convention,
with stable allocator and budget contexts. Focused fixtures cover exact byte
ownership, source transitions, aliasing, compaction, literal allocation schedules,
real unrelated baseline storage, limits and every measured allocation OOM ordinal.
Independent review and parent integration acceptance remain separate. Frame
ownership/atomic publication, `cloneFromFrame`, events, consumers and host
accessibility remain later work.

### Atomic retained semantic frames (Task 3H1b3)

`retained_semantics` now provides `NodeId`, all 18 `NodeRole` values,
`NodeSpec`, ordered `AnnouncementSpec`, `FrameInput`/`FrameView`, `OwnedFrame`
and `FrameSlot`. Input is a flat preorder tree with separate ordered direct
child indices. Import validates UTF-8, finite ordered bounds, unique identities,
action focusability, complete reciprocal topology and iterative preorder.
Hidden, disabled and offscreen nodes remain available in the immutable snapshot;
`visibleNodes` selects positive clip intersections in preorder and preserves the
entire output buffer on insufficient capacity.

`OwnedFrame.init(allocator, budget, input, limits)` copies every byte and array,
retains each occupied owner token slot, and imports complete focus including
null focus and historical modal restoration. `FrameSlot.replace(input, limits)`
prepares one candidate before publishing nodes, announcements, owner and focus
in one swap. Failure preserves resident backing storage and existing borrowed
views. Replacement input may borrow the resident frame or an independently
owned helper, which must remain alive until the call returns.
`OwnedFocus.cloneFromFrame(allocator, view, limits, budget)` copies complete
focus into an independent helper and returns the narrower `FocusError`.

Mandatory local limits cover all copied identity/text/context bytes, counts and
simultaneously live candidate storage plus scratch. The stable preparation
budget separately covers resident frame, helper, candidate and scratch together.
Embedded focus uses private frame C/S charging; standalone focus keeps H/T and
H+J+T behavior. Requested allocation bytes exclude allocator overhead and inline
owner headers; inline token reference slots incur no fictional heap request.
Token objects retain their separately explicit live budget. No production
consumer capacity defaults are introduced.

Ownership is by move convention and UI-thread serialization. Zig fields are
accessible; public methods expose no mutable frame focus, but hostile field
mutation or duplicated owning copies are unsupported. Allocator/budget contexts
must outlive owners; views expire on successful replacement or deinit. This
pure storage qualification provides no host/event delivery, native/browser
accessibility, rendering, IME, consumer admission or game completion evidence.

### Retained atlas metadata loader (H2a)

`retained_atlas` provides value-owned frozen metadata decoding, explicit fixture
schema validation and atomic resident replacement. It binds the existing
53,288-byte JSON SHA-256, validates 205 canonical entries, and uses only the
first 1,048,576 bytes of caller-owned parser scratch. Oversized scratch still
participates in whole-buffer alias checks. No JSON/input/scratch strings survive
in the numeric table. Shared value diagnostics and record types prepare H2b.

Loader-focused tests cover schema admission, frozen identities, numeric field
transfer, scratch boundaries, ownership and replacement failure preservation.
Focused three-mode results and the parent aggregate gate require their own
receipts; this paragraph does not claim aggregate acceptance. No text
measurement/placement, retained consumer, renderer, host, Unicode fallback, font
or asset generation is qualified by this loader slice. Actual public parser
exhaustion remains a separate proof gap: bounded adversarial loads exercise
overcount admission without demonstrating `ScratchExhausted`.

### Existing-atlas staged text slice (H2b)

`retained_text` prepares pure scalar-order text over the existing owned atlas:
`measure` returns the unclipped advance, `replaceBoundaries` replaces scalar
start-byte/advance-before boundaries plus a terminal boundary, and
`appendPlacements` appends glyph rect/UV/color records. UTF-8, supported ASCII
mapping, finite strict-f32 arithmetic and the entire candidate are checked
before admission and staging. One commit preserves complete destination/count
bytes on failure and existing prefix/tail bytes on success. Spaces advance
without records; clipping preserves full advance. There is no allocation,
font fallback, shaping, renderer or consumer installation in these calls.

Explicit lowered limits, including zero, are enforced under the 65,536 input
byte, 65,537 boundary and 8,192 total placement ceilings. Logical staging bytes
charge only new candidate records. On the pinned native macOS aarch64 compiler,
Boundary is 16 bytes and GlyphPlacement is 48: full destination and staging
backings require 1,048,592 bytes each (2,097,184 together) for boundaries or
393,216 each (786,432 together) for placements. PlacementRequest is 72 bytes,
TextLimits 32, Measurement 4 and Outcome(Measurement) 80. Caller buffers are
separate from function frames; no worst transitive stack bound is claimed.
Whole supplied writable slices/count are disjoint from atlas/text and actual
addressable local copied records. By-value parameters expose no original caller
record address. Independent parent source review and aggregate qualification
remain required; host, IME, Unicode fallback and visual acceptance remain open.


### H3a pure HUD/settings semantic projection

`classic_ui.zig` prepares an owned retained command deck or settings screen and
provides a synchronous read-only Session adapter. Settings replaces the HUD
underlay. Publication checks shared budget, revision and actual owner, replaces
the complete FrameSlot, then assigns the caller's trusted publication revision.
Action validation checks that stored revision, actual frame owner and enabled
closed typed action. Caller serialization, reference-slot admission and later
host revision/event routing remain obligations. Help retains legacy ToggleHelp.

The slice retains full UTF-8 recovery input through 16,384 bytes. The Session
adapter deliberately displays bounded recovery categories while preserving raw
Session/Store messages, causes, cleanup errors and priority without clearing or
recovering. Explicit caller transition announcements use only ready and
preferences_changed; the Session adapter emits none. Pure hit-test and pointer
consumption helpers reproduce source controls and legacy 0.72 chrome geometry;
they feed no host events and do not select scene worlds. First marker, guide,
editor, tactical/advisory full content and sighted text materialization remain
outside this slice. No native/browser/accessibility runtime acceptance follows.

At the historical H3a acceptance, measured target Header was 33,232 bytes,
including Charge/budget metadata; the
movable handle is 8 bytes and complete optional publication revision is 16.
Header retains one resident reference and charges its actual one allocation to
frame_scratch until deinit frees storage before releasing its saved Charge.
The historical H3a slice qualification used 196,608 requested bytes. Current
H3c integration measures Header 33,312 bytes and uses the shared 1,179,648-byte
budget; the following figures describe the earlier H3a slice. Reviewed
conservative requested peak is 186,448; adding decimal20 + formatter256 +
handle8 + complete optional revision16 gives a narrow enumerated 186,748.
Using actual Header size instead of its 36,864 ceiling gives 182,816 requested
and 183,116 with those named records. Neither is a complete stack/process bound.
Source192, Controls80, Tray88, WorldName25, CurrentStamp64 and FrameInput184 are
additional measured type layouts; transitive stack and allocator overhead remain
UNKNOWN. Debug emitted local entry frames are characterized in qualification
receipts separately, not charged to the requested-allocation ledger.

Focused qualification includes independent source strings/actions/roles/order,
exact layouts, both actual publications, all recovery variants/numeric widths,
full input poisoning, purity, stale-owner/revision and budget mismatch, checked
retain overflow, all eight exact/one-less limits and every actual allocator OOM
ordinal. Two populated mode transitions observe requested peaks 123,762 and
123,506, with ten allocation ordinals each and transactional failure/retry/cleanup.
The fixed ceilings retain future slack: 72 nodes/71 edges, IDs12,288, text49,152,
focus18, announcements2, context128 and candidate-owned81,920; first HUD has at
most 17 controls/64 nodes. Whole repository qualification is a separate parent
gate; these focused figures do not qualify hosts or complete Classic gameplay.

## Classic editor pure layout (H3b first slice)

`classic_editor_layout` reproduces source section/world/field/footer/confirmation
geometry with scalar preflight and replay into one caller-owned 114-descriptor
workspace. It validates complete bounded reports, maps first source errors to
the exact field, and applies read-only per-field parse flags. Navigation, reverse
hits, reveal and scroll return values without mutating editor or Session state.

A successful layout borrows its workspace until `releaseView`; reuse returns
`WorkspaceBusy`, and double release panics in every optimization mode. Move the
workspace only while unborrowed. `IssueText.issue_index` belongs to its original
immutable report; `issueText` borrows that report until copied. Integration must
call `validateWorkspaceBorrowing` before layout writes with live reports/flags.

Layout allocates no heap and takes no allocator. Descriptor/Workspace target
limits are 96 bytes and 16 bytes overhead. Full local stack and transitive stack
are separate from this fixed storage bound. Retained editor Header, publishing,
text rendering and host dispatch remain later work. Future retained projection
must omit node actions for every nonfocusable source descriptor, including tiny
nonmodal controls, preserving their typed identity and source flags. Restore
actions only through fresh layout/publication after resizing; modal underlay
remains incapable. Disabled Apply is focusable but disabled and excluded from
traversal/activation. No retained tiny-frame publication is claimed here.


### Classic retained editor projection (H3b)

`classic_editor_ui` synchronously copies the current detached editor into a charged,
move-only `PreparedEditor`. The caller supplies an immutable source lease, complete
resident/draft owner, revision, and premodal focus history. Revisions must
advance for every presentation change. Prepared input and geometry borrow the
Header until deinit; `publishEditor` atomically replaces the H1 frame before
updating the trusted revision. Typed target validation returns a capability and
performs no editor, transaction, storage, or gameplay mutation.

The projection preserves complete active field text, IME, validation, independent
storage/action/recovery lanes, and absent remembered focus. It rejects malformed
reports, source/Workspace aliases, stale ownership, and exact text/snapshot budget
excess without truncation. Underlay actions are unavailable during confirmation;
closed source categories retain the original diagnostic objects. Host event
routing, persistent modal-history ownership, rendering, and whole-game runtime
acceptance remain separate integration work.

The pure `classic_start_marker` module projects the initial Union-world marker
from borrowed validated numeric inputs. It returns an inline name and canonical
strict f32 label endpoints, safe-clamps the first admitted source candidate, and
hides both ring and connector when point/ring policy or same-world picking fails.
`validateMarkerGeometry` exports the same endpoint/optional-field contract for
later attachment. Projection requests no heap allocation and changes no inputs.
Numeric camera inputs do not establish provenance. At this historical pure
projection slice, HUD snapshots, UI attachment and capability publication were
pending; the following H3c section qualifies their current headless implementation.
Host rendering and current-session action acceptance remain pending.

### Classic retained start marker integration (H3c)

`classic_start_marker_adapter.snapshotFromPrepared` copies the actual pre-marker
HUD geometry, owner context and control/ancillary zones under an immutable
prepared lease; `projectFromCore` extracts owned Core scene/campaign values under
a synchronous caller-coherent `CurrentLease`. End all snapshot/input borrows
before `classic_ui.attachStartMarker`, then reacquire geometry/input and publish
through the existing H1 path. Attachment preflights the entire insertion and
commits without allocation or token retention; seven static published marker
actions map to `select_guidance_world`. An absent result also consumes the once
attachment, and any successful publication permanently forbids later attachment.

Settings preserves actual guidance/tooltip zones and source help metadata,
without a help or tray zone. Its initial tutorial returns `not_playing`, while
noninitial steps return `not_initial` after full input validation. Settings uses
ordinary publication and rejects attachment with `WrongProjectionMode`.

Focused tests qualify twelve actual Session/Core default viewport/scale cases,
flat DFS/focus insertion, source lifetimes, canonical fractional endpoints,
113-byte maximum marker text copies, every real prepare/publication OOM ordinal
and exact measured shared-byte peaks. Target payloads are Header 33312 bytes,
inline optional visual 76, marker 72, snapshot 744 and attachment stage 616; these
are separate payloads, not whole process or transitive stack bounds. The shared
1179648-byte policy and existing candidate/descriptor limits remain unchanged.
Host dispatch must still revalidate current playing/initial/Union state; ring/
connector drawing, native/browser visuals, accessibility and whole-game
acceptance remain pending.

### Classic retained player guide (H3d)

`classic_guide` prepares a pure offline full-shell guide with all seven original
Rust topics and 34 verbatim paragraphs. The single charged 20,056-byte Header
owns pagination spans, selected preserved-case semantics, uppercase line text,
context, node descriptors and canonical focus. H1 owns copies after publication.
`GuidePublication` is a trusted pointer-free 32-byte revision/Previous/Next record
that survives Header destruction; replacement failure preserves the old record
and displayed frame. The caller owns active-guide routing and the owner lease.

Target Metadata is 280 bytes, WordSpan 6 and LineSpan 8. Dynamic text ceiling
7,173 and conservative H1 copy ceiling 7,440 include both full body lanes; actual
four-copy button labels total 104 bytes. The local Header and candidate caps are
32,768 each, within the same shared 1,179,648-byte preparation budget. Measured
fixture transition peaks are 42,512 guide->guide, 45,031 HUD->guide, 49,682
editor->guide, 58,839 guide->HUD and 308,890 guide->editor requested bytes. These
are fixture measurements, including live old frames and pending reservations
once, not worst-case populations or mapped/stack/process footprint estimates.

Body rendering/H2 integration, Session guide state, complete saved focus, Main
menu destination, suppression of the entire Core advance and advisory work,
entry/exit clock baseline, native/browser/accessibility and performance remain
later acceptance. The pure guide has no Session getter, fake pause field or host
effect. Focused evidence and proof gaps are in
`../.superpowers/sdd/2026-10-09-nyon-zig-migration/task-3h3d-guide-report.md`.

Task3I1 adds a pure Classic input foundation: one pinned 32-slot external
reference registry, a checked monotonic revision domain, and eight complete UTF-8
semantic event records. Each record has a 3840-byte shared context/action/text
payload (context 128 and action 64 maximum), with explicit overflow and no
truncation. Events retain full owner identities until FIFO drain; each dispatch
consumer must compare the entire owner and revision afresh. The live token
budget remains independently 32 objects/1280 requested bytes. Registry/clock
state must never be reset, copied as independent live state, moved or reused
while tickets/views survive. End all borrows and producers, drain queues, release
frame/hold/root ownership and tickets, then terminally close clock/registry/token
budget. Occupied registry shutdown panics in all optimization modes; hidden
views and arbitrary address reuse remain caller obligations. This foundation
does not implement normalized key edges, host delivery/backpressure, Session
routing or guide orchestration.

Task3I1 normalized keyboard helpers now provide synchronous borrowed whole-UTF8
key/text/IME carriers, exact Classic action/navigation mapping, Unicode Cc text
validation, and independent 15-action held/pending boolean sets in
`classic_input.zig`. InputState retains no event or slice and measures 30 bytes
on the pinned native target. Release preserves a pending edge; frame completion
clears pending edges while preserving held state. Named Space maps to launch;
character Space and named Enter/F1 have no gameplay mapping. Repeat/modifier
metadata remains caller-owned; only Shift changes Tab navigation.

These are helpers only. Guide/editor/Settings/Playing precedence, typed Session
routing, focus/suspend reset boundaries, editor IME effects and host key/IME
normalization/delivery remain Task3I3 or later qualification. Large borrowed IME
payloads are independent of semantic-queue storage admission.

Task3I2 adds `classic_interaction.zig`, a pure generic inline contact controller,
qualified with the actual `classic_session.UiAction` value type at capacity eight.
The ninth distinct contact returns `ContactLimit`; finite-position and shared raw
ID duplicate preflight reject without changing contacts or pinch state. Eight is
an accepted admission bound, not a native device maximum. Mouse/touch ownership,
f32 distance threshold, release-only secondary launch and exact-two-touch pinch
follow Rust source. Tests measure action 3, optional entry 40, controller 328 bytes
on the pinned macOS target; caller stack usage remains unknown. No allocator,
Session, camera or host effects occur. Derived orbit overflow is preserved;
finite consumer validation, HUD-first hit testing/current typed action validation,
world repicking, DPI conversion, lifecycle delivery and capacity backpressure
remain pending router/host work. Pure cancel clears all contacts; this alone does
not implement source callers. In source, absent-guide Open, focus loss and touch
cancellation clear Classic contacts; suspend and occlusion preserve them.

`classic_ui.PreparedProjection.setHudPublishedFocus` is a playing-only,
prepublication focus adapter. Call it after optional start-marker attachment
and after ending immutable projection borrows. It accepts null or an exact
current focus-order ID, validates the full supplied UTF-8 slice, and stores the
existing canonical ID. Caller bytes are never retained. Disabled controls may
remain focused; `validateAction` still rejects their activation. Repeated calls
before publication are allowed. Failed publication permits correction and
retry; successful publication permanently rejects further setter calls, even
when its frame has been released. The setter adds no allocation, token retain,
budget request, or Header storage. Router and host integration remain pending.
