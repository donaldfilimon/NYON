# Task 3 — Classic application, editor and presentation inventory

Date: 2026-10-09. Status: **Current source inventory; Proposed migration slices.**

This is a read-only inventory of the canonical NYON checkout. Only this report
was written. No source implementation, build, tests, native launch or browser
qualification was performed by this inventory task. Existing test assertions
below are acceptance oracles, not claims that they passed in this session.
Workshop/Living ports are out of scope; their names appear only where the shared
host exposes a boundary that Classic must preserve.

## Resolved source layout and authority boundary

The requested `src/app/mod.rs`, `src/app/clock.rs`, `tests/app_core.rs` and
`tests/editor.rs` do not exist in this checkout. Their actual counterparts are:

| Surface | Current owner and references |
| --- | --- |
| App module index and platform host | `src/app.rs:1-10`, host owner at `src/app.rs:225-243` |
| Classic core | `src/app/core.rs:147-178`; settings/onboarding/focus extension `src/app/core/session.rs:5-193` |
| Fixed clock | `src/engine/time.rs:3-55` |
| Editor index/state | `src/editor.rs:10-18`, `138-183`; layout `src/editor/layout.rs:14-224`; rendering `src/editor/render.rs:8-57` |
| Presentation index | `src/presentation/mod.rs:3-14`; Classic camera, picking, scene, interaction and UI are separate files; exclude the exported Living/Workshop modules |
| App/editor acceptance | `tests/app.rs:33-209`, `tests/scenario_editor.rs:17-405` |
| Usability/presentation/guide acceptance | `tests/usability.rs:61-543`, `tests/presentation.rs:54-220`, Classic portions of `tests/player_guide.rs:13-93`, `169-252` |

`AGENTS.md` assigns truth exclusively to `game::model`/`game::simulation`.
`AppCore` owns `active_scenario` and `Simulation`; every other core field is
session, lifecycle or presentation state (`src/app/core.rs:147-178`). The editor
owns a detached original/draft/text/error set; it receives no mutable simulation
reference (`src/editor.rs:138-156`). Immutable preview and scene extraction may
read campaign state, but only the application's gameplay dispatch calls enqueue.

Task 1 provides the Zig authority and codecs. Currently inspected interfaces are
`Simulation.fromScenario`, `enqueueNext`, `previewCommand`, `step`, `deinit` and
`canonicalFingerprint` (`zig/src/simulation.zig:78-126`, `390`);
`ScenarioDraft.clone`, `regenerateWorldsFromSeed`, `validated`, `deinit`
(`zig/src/scenario.zig:91-116`); `ScenarioV1.toDraft` (`215`);
`encodeDraft`, `decodeDraftDetailed` (`483`, `627`); and the preference value types
and `presentation()` (`zig/src/preferences.zig:9-62`). These are Current API
observations, not a Task 1 acceptance verdict. Task 2's store interface is a
dependency to agree before transaction implementation; do not invent a second
filesystem adapter inside Task 3.

## Binding behavioral contracts and traps

### Clock, selected speed and actual stepping

Classic speeds are Paused/Normal/Fast/VeryFast with multipliers 0/1/2/4
(`src/app/core.rs:47-82`). `game_speed` is the selected speed and `resume_speed`
remembers the last nonpaused speed. Onboarding rejects attempts to select a
nonpaused speed; changing speed clears clock residue (`271-282`). Settings and
editor retain selected speed while holding authority ticks; `advance` clears the
clock whenever mode is not Playing or selected speed is Paused (`790-802`).

There is no named requested/effective speed pair in Classic Rust. A Zig projection
may expose `requestedSpeed()` and `effectiveMultiplier()` if they faithfully map
to those fields and mode/host holds. Do not claim that this is a pre-existing Rust
API. Guide holds are host-level zero delta, not a replacement of selected speed
(`src/app.rs:1856-1878`); opening/closing guide resets `last_frame`
(`1626-1664`). Suspension/resumption clear residue (`src/app/core.rs:923-929`).
Do not import broader visibility-hold policies from other modes without a
Classic requirement.

Clock accumulation is integer nanoseconds multiplied by tick Hz, with a
1,000,000,000 denominator; accepted frame delta is capped at 250 ms. Whole steps
beyond available capacity are discarded, retaining only fractional residue
(`src/engine/time.rs:3-37`). **The 15-step limit is per multiplier pass**, because
Classic pushes and drains the clock once for each multiplier (`core.rs:799-802`).
At 60 Hz a two-second frame yields 15/30/60 steps at 1x/2x/4x, not a universal
15-step limit. This consequence is source-derived and needs a new explicit port
test. Existing speed oracle proves 0/6/12/24 ticks for 100 ms
(`tests/usability.rs:62-89`).

### Queue, preview and explicit launch

Gameplay actions use explicit source/destination world IDs
(`core.rs:108-138`; `tests/app.rs:34-63`). Preview is immutable and supplies
launch strength and rejection labels to the command tray (`core.rs:424-470`) and
field enabled states (`552-570`). It does not reserve energy or enqueue.

Queueing is legal only in Playing. A successful enqueue consumes the next
sequence with wrapping increment; failure does not (`767-787`). Crucially,
`preview_valid` controls onboarding observation, **not enqueue admission**:
commands whose gameplay preview rejects may still be queued and later rejected
by simulation. Preserve that distinction from disabled UI controls. Advisory
material dirtiness uses accepted tick reports or events, not queued intent
(`805-817`; `tests/app.rs:125-140`).

Mouse primary tap replaces selection. Secondary tap explicitly queues a launch.
Touch selects/replaces a Union source, stores a nonfriendly destination for
preview, and clears on empty scene; destination selection alone never queues
(`core.rs:698-760`; `tests/usability.rs:130-203`). Contextual UI launch uses
`hovered_world.or(touch_command_target)` (`325-326`, `649-656`), while direct
keyboard `Action::Launch` uses hover (`635-643`). Mouse hover leaving the scene
must not retain the destination (`tests/usability.rs:146-158`). The phrase
“launch confirmation” in Task 3 means explicit Launch/Space/right-click after a
tray preview; Classic source has no additional launch modal. Editor destructive
actions do have separate confirmation modals.

### Editor invalid drafts and transaction boundaries

Original and current draft are separate; saving/loading does not redefine the
original active scenario (`editor.rs:138-156`, `549-552`, `616-618`;
`tests/scenario_editor.rs:229-239`). Dirty is typed draft inequality or any parse
error (`222-237`). Failed numeric parsing retains the text and leaves the prior
typed value intact (`727-736`, `892-898`). Semantic validation and parse-error
tracking are separate; `validation_report()` alone omits parse errors.

All seven worlds expose name, x/y, owner, energy, defense, base output,
regeneration, atmosphere, hydrosphere and topology. Rules expose win threshold,
AI/hazard timings, maxima, launch minimum, field costs/refund and fleet speed.
Seed has a 16-digit hex editor field. Tick Hz is displayed but not editable
(`editor.rs:740-837`; `editor/layout.rs:385-452`). Editor parsing is distinct from
canonical wire parsing: Rust's seed editor uses `from_str_radix` and numeric
fields use `FromStr`. Do not automatically substitute strict codec parsing and
silently narrow editing acceptance; canonical encoding remains Task 1's boundary.

IME preedit is visual only; commit appends and clears preedit. During confirmation
IME/insert/backspace are blocked. `replace_focused_text` during confirmation
instead clears confirmation/pending payloads and returns without applying the
replacement (`245-259`, `410-455`). Modal widgets disable all background focus and
actions (`editor/layout.rs:216-222`).

| Operation | Required behavior and source |
| --- | --- |
| Load clean | Read slot; absence produces `NO SAVED SCENARIO`; decode and validate temporary draft before replacement. Store/codec failure preserves draft and emits status (`editor.rs:524-554`). |
| Load dirty | Request creates LoadDirty confirmation; confirmed load still validates before replacement (`556-565`, `652-654`). Original/active scenario stays unchanged. |
| Save empty | Reject parse errors, canonical-encode validated draft, check occupancy, then write; successful save stays detached (`589-618`; `scenario/codec.rs:35-36`). |
| Save occupied | Any present payload, including invalid bytes, requires OverwriteSave; retain encoded pending payload. Cancel leaves old slot and draft unchanged (`597-609`, `621-625`, `637-648`). |
| Save failure | Draft, original, active scenario and old durable payload survive; visible status. Confirmed failed save consumes that confirmation, requiring a fresh request (`637-646`; `tests/scenario_editor.rs:201-225`). |
| Apply | Request validates and captures candidate; confirm validates current candidate and compares it with pending. Result is an owned validated ScenarioV1, not an unguarded action (`578-586`, `661-676`). |
| Commit apply | App builds candidate Simulation before swapping active scenario/simulation; resets clock, selection, command sequence, field, camera/help/visual time and returns Playing (`core.rs:904-913`, `966-982`). Port must preserve old state on Zig admission/allocation errors. |
| Cancel | Clean closes immediately; dirty opens CancelDirty; dismiss keeps draft; confirmed Cancel alone closes (`editor.rs:568-575`, `655-659`; `core.rs:871-898`). |
| Revert/default/regenerate | Confirmation required; only detached draft changes. Revert restores original; factory defaults replaces draft; regeneration uses current seed/rules (`editor.rs:473-522`, `649-651`). |

Restart uses the already active scenario, not saved slot or editor draft
(`core.rs:916-920`). Preferences persist independently; preference load error
uses defaults and a recoverable message, while save failure preserves the usable
changed in-memory preference state (`core.rs:194-202`, `995-1002`;
`tests/usability.rs:498-527`). Preserve error visibility without mutating campaign
truth or rolling back a successful session-only setting. General recoverable
message takes precedence over preference message (`core.rs:581-584`).

### Onboarding, focus, presentation and host reality

Six ordered observed events teach Union selection, launch preview, explicit
enqueue, field tune, advisory read and editor open. Unexpected events do not
advance (`onboarding.rs:14-21`, `76-108`). Valid enqueued commands advance guidance
before simulation execution; onboarding is not a tick-report success verifier.
Completion/skip persists `onboarding_completed` and resumes saved speed; reset
pauses and clears help (`core/session.rs:148-191`; `core.rs:985-991`). Skip must
keep the same match and queued intent (`tests/player_guide.rs:226-252`).

Playing HUD focus stores action identity so layout changes do not retarget it;
settings owns an 11-control focus index with wrap/clamp
(`core/session.rs:79-145`; `settings.rs:23-39`). Scene taps clear HUD focus
(`core.rs:698-703`; `tests/usability.rs:206-228`). Enter/Space activates focused
HUD; Space without HUD focus launches. Text input, including spaces, wins before
editor control activation (`input_router.rs:625-683`). Raw key bindings live at
`app.rs:2519-2539`: 1/2/3, Q/E, P, brackets, C, S, R, H, Space, Escape, F4;
F1 opens/closes guide (`input_router.rs:524-540`). Gameplay pressed edges are
consumed once (`engine/input.rs:55-67`).

Presentation extraction is pure and bounded: seven worlds, fleets sorted by ID
and capped at 512, deduplicated routes capped at 256, visual seed derived from
scenario fingerprint, interpolation independent of authority (`scene.rs:19-22`,
`94-107`, `121-211`). Reduced motion disables particles/halo animation/parallax
and uses static background; Low/High particles are 96/512, while Auto requests the
high path; renderer decides capability fallback (`214-247`). Camera motion
clamps/smoothing and reduced-motion snapping are independent of game ticks
(`camera.rs:10-18`, `96-173`). Picking operates on logical pixels, rejects
nonfinite rays/zero viewports, and ties by nearest positive distance then WorldId
(`picking.rs:14-94`). UI takes HUD ownership first, including opaque gaps and
the entirety of Settings (`ui.rs:175-205`).

Classic widget models are headlessly testable. Native/browser accessibility
delivery, GPU matrices rendered correctly, physical/logical pixel conversion,
font/atlas/batching and actual input delivery require Task 6. Existing Classic
semantic host tree includes only application/guidance nodes
(`app.rs:834-842`); do not claim screen-reader coverage of every Classic control
from that source. Task 6's complete accessibility requirement remains open.

Guide pages are offline/read-only, start on Classic's first-fleet topic, share
readable lines with semantics, paginate on viewport and restore prior focus when
closed (`ui/guide.rs:89-156`; `app.rs:1626-1664`). Source teaching specifically
says launch spends half current source energy, keeps source defense unchanged,
and snapshots source Hydrosphere (`ui/guide.rs:26-32`). Classic restart loses
battle progress; guide Main menu does not save battle (`ui/guide.rs:73`).

**Durable exit limitation:** `DurableExitState` is a pure save-wait/cancel state
machine, but its Current save requirement is for the shared host's other session
mode (`app/durable_exit.rs:1-46`). Classic passes no save requirement and exits
immediately (`app.rs:1949-1959`). Classic scenario/preference writes are explicit
separate transactions, not a durable campaign autosave. Port that fact; a new
Classic battle autosave would be a design change. Shared durable close jobs and
platform quit wiring belong to Task 6, with the pure state contract available
from Task 3.

## Proposed sequential implementation slices

All paths below are **Proposed** new ownership under `zig/src/`. Each slice owns
its listed tests alongside its modules. Shared integration edits to `root.zig`,
`build.zig`, CLI and `tools/check.sh` are a separate parent integration dispatch
after slice review; workers must not edit them independently. Task 1 files stay
owned by Task 1 and store adapters by Task 2. No new production dependency is
assumed; camera matrix math/platform bindings need a concrete reviewed interface
before implementation.

### 3a — Fixed clock and Classic session skeleton

Own `classic_clock.zig`, `classic_app.zig`, `classic_clock_tests.zig`,
`classic_app_tests.zig`. Interface: allocator-owned AppCore containing active
ScenarioV1/Simulation, mode, selected/resume speed, sequence and recoverable
status; `init`, `deinit`, `setSpeed`, `advance(delta_ns)`, `onSuspend`, `onResume`,
`restart`, read-only campaign/replay identity. Clock exports
`pushFrame/takeStep/alpha/clear`. No window clock or platform import.

Acceptance:

- [ ] Rational carry at 16,666,666 ns then 1 ns matches `time.rs:76-83`.
- [ ] 250 ms clamp, dropped excess whole work and per-speed-pass 15 limit.
- [ ] 100 ms speed traces match direct canonical 0/6/12/24 steps.
- [ ] Speed/mode/suspend/resume transitions discard residue and never catch up.
- [ ] Restart builds temporary state before swapping, resets sequence/selection,
      and retains preferences; allocation/admission failure leaves live state.
- [ ] Tick report storage is deinitialized on every successful step; OOM is
      reported, not converted to success. Earlier successful ticks in a frame
      remain successful; failing step must not be silently retried after lost
      clock work. Define clear-and-hold/report policy for the Zig-only error path.

Review before 3b; authority errors and report ownership must be settled first.

### 3b — ID-based command controller and preview projection

Own `classic_commands.zig`, `classic_command_tests.zig`; extend AppCore only via
parent integration. Interface: explicit ID `select/hover/touchTarget/clear`,
`commandForAction`, immutable `previewLaunch/previewField`, tray/rejection model,
`queue(command)` returning typed enqueued/rejected outcome. No geometry yet;
later picking resolves a logical pointer to this ID interface.

Acceptance:

- [ ] Preview/tray/field enabled state leaves fingerprint, queue and sequence
      unchanged; reports every rejection class (`core.rs:1021-1034`).
- [ ] Enqueue failure, including allocator failure, does not consume sequence;
      successful enqueue wraps sequence exactly and takes effect only at a tick.
- [ ] Gameplay-invalid preview can still be enqueued by direct action and gets
      canonical tick rejection; disabled HUD control does not dispatch.
- [ ] Mouse hover target clears; touch source replacement/empty-clear and
      destination-only preview preserve source and never launch.
- [ ] Explicit Launch, Space and right-click produce the intended IDs;
      Editing/Settings reject gameplay dispatch; no extra launch modal invented.

### 3c — Detached editor fields, invalid text and IME

Own `classic_editor.zig`, `classic_editor_fields.zig`,
`classic_editor_fields_tests.zig`. Interface: Editor owns deep-cloned
original/draft, stable typed field IDs, raw text buffers, parse-error set,
focus/preedit and semantic validation. Export `canApply/isDirty`,
`replace/insert/backspace/commitIme/setPreedit`, validated candidate accessor.
Borrow no draft name slice from the active scenario or temporary decode arena.

Acceptance:

- [ ] Every world/rule field listed above, fixed tick-Hz display and hex seed
      round trip; warning-only report still applies.
- [ ] Invalid number/overflow/owner/name survives as raw text; old typed number
      stays intact; semantic-invalid typed draft cannot apply or save.
- [ ] Editor parse acceptance matches Rust separately from canonical wire
      acceptance, including lowercase hex editor input where Rust accepts it.
- [ ] UTF-8 backspace removes a character; printable space and IME preedit/commit
      do not dispatch gameplay or unintended action.
- [ ] Allocator failure preserves draft/buffers and original; teardown leaks no
      arena/buffer/widget storage. No mutable Simulation reference available.

### 3d — Confirmed editor load/save/apply transactions

Own `classic_editor_transactions.zig`, `classic_editor_transaction_tests.zig`.
Interface: `request(action, store)` produces completed/needs-confirmation/error;
`confirm(store)` returns none, cancel, or owned validated ScenarioV1;
`dismiss` destroys pending payloads. Use Task 2 typed slot load/save outcomes;
occupancy is presence, not successful decode. App installation consumes validated
candidate after temporary Simulation admission.

Acceptance:

- [ ] Matrix above for absent, valid, malformed, oversized and errored slot;
      failed dirty confirmed load keeps draft, original and active authority.
- [ ] Empty save needs no modal; occupied/corrupt save needs overwrite consent;
      denied write preserves old bytes and exposes status.
- [ ] Cancel/dismiss/revert/default/regenerate affect only authorized detached
      state; save/load never replace active scenario or reset original.
- [ ] Stale apply and attempted replace during modal cannot commit pending
      candidate; no confirmation returns typed error.
- [ ] Apply builds first then swaps, resets all session fields specified above;
      failed validation/admission/OOM preserves active state and retained editor.
- [ ] All failure paths free owned pending payloads exactly once; errors remain
      visible and a fresh request can recover.

### 3e — Preferences, onboarding and action-stable focus

Own `classic_session.zig`, `classic_onboarding.zig`, `classic_settings.zig`,
`classic_session_tests.zig`. Interface: preference load/save effects through
Task 2, six-event onboarding automaton, settings actions and 11-control focus,
stable HUD action ID focus. Session settings remain separate from ScenarioV1.

Acceptance:

- [ ] First run is paused; unexpected event cannot advance; observed preview,
      valid enqueue, tune, advisory read and editor open advance in order.
- [ ] Skip/completion resume remembered speed in the same match; reset pauses;
      none resets campaign or clears queued gameplay commands.
- [ ] All four UI scales, motion/contrast/quality and tutorial reset persist
      independently. Default-on-load-error and usable-state-on-save-error match
      `tests/usability.rs:498-527`.
- [ ] Settings holds ticks while retaining selected speed; closing avoids catch
      up; stable HUD action survives frame rebuild and scene input releases it.
- [ ] Focus wrap/clamp and disabled activation match current settings/HUD model.

### 3f — Classic advisory controller dependency

Own `classic_advisory.zig`, `classic_advisory_tests.zig` after a separate read-only
inventory of `src/advisory` establishes CPU score/freshness/parity contracts.
This report reads only the AppCore integration, not the advisory algorithm.
Interface: immutable campaign/fingerprint request, CPU published snapshot,
request generation/device epoch, pending GPU request and completion disposition.
GPU dispatch is Task 6. This is an explicitly identified dependency, not a
claim that advisory port inventory is finished.

Acceptance:

- [ ] Initialization and restart produce CPU reference output; accepted tick
      reports/events trigger material refresh; enqueue alone does not.
- [ ] Stale/device-failed/parity-failed completion cannot replace the fresh CPU
      snapshot or change campaign truth; actual score oracle derived from source.
- [ ] App reports published/updating/error state and never turns a score into a
      gameplay command. Advisory-read onboarding uses this visible output.

Review the deeper advisory inventory before execution; do not replace the
scoring contract with placeholder data merely to unblock UI.

### 3g — Camera, immutable scene and logical picking

Own `classic_camera.zig`, `classic_scene.zig`, `classic_picking.zig`,
`classic_presentation_tests.zig`. Interface: value math matrices/camera,
`buildScene(const campaign, scenario_fingerprint, alpha, view, advisory,
visual_time, camera, preferences)` and `pickAfterHud`. No GPU handle or platform
event enters these modules. Frame storage is owned by caller or explicitly
deinitialized; no references to transient Campaign snapshots escape.

Acceptance:

- [ ] Camera yaw/pitch/distance limits, finite viewport, smoothing and reduced
      snap match source; default camera frames every world in gameplay safe area.
- [ ] Projected world centers pick the same IDs; HUD wins; invalid rays reject;
      overlap tie is positive distance then ID (`tests/presentation.rs:108-150`).
- [ ] Seven worlds, sorted capped fleets, route dedupe/cap, visual seed,
      interpolation/hazard cues and low/high/reduced effects match source.
- [ ] Camera/quality/motion/scene build leave digest and queue unchanged;
      allocation failure cannot mutate campaign. Graphic pixel parity remains
      Task 6 even when these tests pass.

### 3h — Retained UI/editor layout and offline guide models

Own `classic_ui.zig`, `classic_editor_layout.zig`, `classic_guide.zig`,
`classic_start_marker.zig`, corresponding layout/guide tests. Interface: immutable
frames containing action IDs, labels, tooltips, enabled/selected/focused/error
state, logical rects, visibility and text runs; common frame supplies rendering
and hit/focus/semantics projection. Guide owns topic/sheet and returns readable
paginated content/Previous/Next/Menu/Close actions. Render batches are a Task 6
adapter, not imports back into these model modules.

Acceptance:

- [ ] 1100px wide threshold; wide editor nav/preview/form/footer, compact tabs;
      1440x900, 960x600 and 320x220 editor bounds/touch sizes match oracle.
- [ ] Four scales preserve 44 logical-pixel controls at supported windows,
      nonoverlap and visible focus; extreme 120x100 UI remains finite.
- [ ] One widget list drives focus/hit/culling. Forward/reverse focus reveals
      clipped fields; page/wheel/home/end clamps; confirmation traps focus.
- [ ] Invalid text/preedit/validation/store errors and all confirmation messages
      are visible. Opaque HUD gaps cannot leak hover/tap into the scene.
- [ ] Start marker targets a selectable Union world only at initial onboarding
      step and vanishes on selection; Skip preserves same match.
- [ ] Classic guide content matches energy-based launch mechanics, paginates and
      remains keyboard reachable; offline data has no platform dependency.

### 3i — Normalized input routing, guide hold and durable-exit contract

Own `classic_input.zig`, `classic_interaction.zig`, `classic_router.zig`, `durable_exit.zig`,
`classic_router_tests.zig`. Interface: normalized logical events
`key(edge, repeat, modifiers)/text/ime/pointer(id, source, button)/wheel` plus
`GuideOpen/Close/Quit/Suspend/Resume`; route according to active guide/modal/mode
before gameplay. InteractionController yields HUD/tap/immediate-launch/orbit/
zoom; pure DurableExit yields ExitNow/PrepareDurableSession/Wait and cancel.
The shared runtime provides save readiness; it owns job persistence, not this
state machine. Actual winit/browser/accessibility events remain Task 6.

Acceptance:

- [ ] Same typed action from pointer/keyboard; key repeat never duplicates
      gameplay pressed edges; release/repress works; Space text/HUD/launch
      ownership and Escape confirmation/editor/settings/selection priority hold.
- [ ] 6px drag threshold prevents accidental HUD activation/secondary launch;
      HUD ownership never becomes scene action. Two non-HUD touches pinch;
      coincident fingers stay finite; pinch end never falls through to a tap.
- [ ] F1/H/HELP open read-only guide, clear interaction edges, hold elapsed
      gameplay time, restore prior focus/speed and reset host-time baseline.
- [ ] Quit on Classic yields ExitNow and does not invent saved battle progress;
      pending durable-session exit waits/idempotently prepares/cancels through
      readiness input. Cancellation alone must not claim storage work undone.
- [ ] Any adapter-queued input belongs to current frame/session owner; stale
      input after mode/guide replacement cannot dispatch a former control.

### 3j — Classic vertical acceptance with Task 6 host

No duplicate platform module ownership. Own Classic journey specifications and
fixtures (`classic_journey_tests.zig` for normalized core journeys; later native/
browser scripts in a path agreed with Task 6). Task 6 consumes Task 3's AppCore,
scene/widget/guide frames, normalized input router and effect/readiness protocol.
Task 6 owns native/browser event loop, GPU/atlas/batches, store binding,
native/DOM semantic adapters and physical/logical coordinate conversion.

Acceptance:

- [ ] Headless journey: fresh preferences → skip/select → hover/touch preview →
      explicit launch → tick result; compare canonical Rust/Zig reports.
- [ ] Detached journey: invalid field → blocked apply/save → corrected field →
      dirty cancel/dismiss → save/overwrite/cancel → load → confirmed apply →
      restart. Inject store/codec/admission/allocation failures at each boundary.
- [ ] Settings/guide/scene/camera interaction fingerprints demonstrate state
      isolation and retained requested speed across holds.
- [ ] Native and browser journeys reproduce the above through real controls,
      actual profile-safe persistence and reopen, with 1x/Retina, compact/wide,
      touch/pinch, keyboard focus/text/IME and visible error recovery.
- [ ] Every required Classic control has working semantic action/focus feedback;
      record remaining Rust accessibility gaps separately from Zig acceptance.
- [ ] Render/backend failures retain usable authority/CPU advisory and recover
      as specified; physical/logical layout and screen-reader behavior are live
      evidence, not inferred from a pure model test.

Task 3 is **Partial** after headless slices alone. Mark it complete only when its
graphical journeys supplied by Task 6 are exercised; terminal interaction cannot
close the app/editor/presentation experience.

## Review and verification dispatch

Execute 3a → 3b → 3c → 3d → 3e → 3f → 3g → 3h → 3i → 3j sequentially, reviewing
each interface before dependent work. 3d blocks on Task 2's actual typed store
contract. 3f blocks on the focused advisory source inventory. Math/renderer
types and host normalized events are design dependencies to agree with Task 6,
not grounds to pull GPU/platform dependencies into authority.

Run each slice's Zig Debug/ReleaseSafe tests and the existing Zig check script
after integration; retain allocation-failure tests where owned dynamic state
changes. For Rust comparison witnesses use the repository-documented focused
commands: `cargo test --test app`, `--test scenario_editor`, `--test usability`,
`--test presentation`, `--test player_guide`, and focused clock/input/onboarding
library tests. `tests/player_guide` includes other modes: distinguish Classic
witnesses from broader suite results. Broader repository/native/wasm gates remain
the parent migration ledger's responsibility; this report does not run or mark
any gate green.

Ordering concerns: settle allocator ownership and error handling before AppCore
composition; distinguish queue admission from preview validation; preserve
per-multiplier clock clamping; do not conflate saving a draft with applying a
scenario; do not announce Classic battle autosave or complete screen-reader
support absent evidence; add the unassigned advisory dependency explicitly;
leave real graphics/event/storage delivery with Task 6 while preserving Task 3's
obligation to qualify the resulting player experience.
