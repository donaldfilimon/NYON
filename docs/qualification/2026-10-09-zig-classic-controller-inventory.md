# Task 3B command-controller seams

Date: 2026-10-09. **Current:** source trace only. **Proposed:** the smallest Zig interface below. No product files, dependencies, builds, tests, commits or lifecycle-review verdict were produced. This resolves 3B interface questions, not 3A acceptance or the later onboarding/picking/UI slices. The actual Zig ID/command type file is `zig/src/model.zig`; there is no `types.zig` under `zig/`.

## ID transitions and admission

Let `S`, `H`, `T`, `F` mean selected world, hovered world, latched touch target and selected field. Store IDs/value enums, never world pointers. `commandTarget = H orelse T`. Initialization/restart use `(null,null,null,atmosphere)` (`src/app/core.rs:220-224,325-326,966-979`; `zig/src/classic_app.zig:101-105,271-275`). These transitions accept a picking result `?WorldId`; the controller does not own coordinates, rays or camera state.

| Operation | Exact source transition / result |
| --- | --- |
| Selection helper, equivalent to `select_scene_world` | Non-Playing or HUD-consumed: false, no change. Otherwise `S=hit; T=null`; leave `H` and `F` alone, clear HUD focus, return true even on empty. This helper emits no onboarding event (`src/app/core.rs:400-407`). |
| Hover, equivalent to `hover_scene_world` | Playing: `H=hit`, including null on HUD consumption/empty; preserve `S,T,F`. Other modes: no change. This helper emits no onboarding event (`src/app/core.rs:410-413`). |
| Explicit hover clear | Set `H=null` without a mode gate; preserve `S,T,F` (`src/app/core.rs:329-330`). |
| Cursor-hover variant | Playing: update `H` as above and observe LaunchPreviewed only if contextual preview succeeds. The coordinate layer rejects nonfinite input before this transition (`src/app/core.rs:678-691`). |
| Mouse primary tap | Playing only: `S=hit; H=hit; T=null`; clear HUD focus; return true even on empty. Any resident world can become selected, including enemy/neutral. Observe UnionWorldSelected only for a Union-owned hit (`src/app/core.rs:698-703,736-742`). |
| Touch empty | Playing only: clear `S,H,T` and HUD focus; return true (`src/app/core.rs:704-708,659-663`). |
| Touch Union hit | `S=hit; H=hit; T=null`; clear HUD focus; observe UnionWorldSelected. Tapping another Union world replaces the source; tapping the same source makes `S==H`, not a launch (`src/app/core.rs:710-715`; `tests/usability.rs:177-202`). |
| Touch non-Union hit with a currently Union-owned `S` | Preserve `S`; `H=hit; T=hit`; clear HUD focus; observe LaunchPreviewed only for valid preview. Never enqueue (`src/app/core.rs:717-729`; `tests/usability.rs:130-142`). |
| Touch non-Union hit without a currently Union-owned `S` | `S=null; H=hit; T=null`; clear HUD focus. This also covers a source whose ownership changed (`src/app/core.rs:731-734,666-671`). |
| Secondary/right-click launch | Non-Playing, no destination or no selected source: false, no ID change. Otherwise `H=destination; T=null` **before enqueue**; queue the explicit `Launch{S,destination}` even if gameplay preview rejects. Those ID changes survive an enqueue failure (`src/app/core.rs:745-760`). |
| Cancel/clear | Clear only `S,H,T` plus HUD focus; preserve `F`, sequence and queued intent. Action Cancel has Playing gate; direct UI ClearSelection dispatch has no mode gate (`src/app/core.rs:606-609,627-629,659-663`; `src/app/core/session.rs:39-41`). |
| Field selection | Playing only: set atmosphere/hydrosphere/topology; it does not enqueue (`src/app/core.rs:606-613`). |

Hover has priority even when it equals the source; do not prefer a different latched touch target to avoid a collision. Clearing hover preserves a touch target, whereas the mouse path has no latch and therefore loses its target (`src/app/core.rs:325-330`; `tests/usability.rs:146-158`). Contextual UI launch uses `H orelse T`; keyboard `Action::Launch` uses **H only**. Increase/decrease use `S,F` (`src/app/core.rs:108-138,635-656`). Neither selection nor target choice implicitly launches.

**Invalid IDs:** Rust's public pointer paths receive only IDs extracted from resident scene worlds; there is no arbitrary-ID selection setter (`src/app/core.rs:390-397`; `src/presentation/picking.rs:72-94`). Safe ownership checks return false for a missing world; tray names fall back to `NONE` (`src/app/core.rs:435-443,666-671`). Proposed new ID-only picking seam: after the mode/HUD gate, reject a nonnull nonresident ID with explicit `InvalidWorld` before changing IDs/focus; `null` remains a valid empty-scene result. This is an interface admission choice, not an existing Rust invalid-pointer transition. Keep raw `GameCommand` dispatch separate: unknown source/destination IDs must still reach enqueue and canonical tick rejection. Valid ID lookup precedes same-source/target, ownership and energy checks in preview (`zig/src/simulation.zig:490-518`). Guidance-world selection additionally requires Playing, the SelectUnionWorld onboarding step and current Union ownership; its step gate belongs to 3E (`src/app/core/session.rs:25-36`).

## Immutable preview and control policy

`previewContextualLaunch(*const AppCore)` is absent when either `S` or `H orelse T` is absent; otherwise return the exact authority preview/rejection. It has no mode gate. Read-only tray and field-control projection must not reserve energy, modify IDs, consume sequence, enqueue, clear targets, allocate through the authority allocator or emit onboarding observations (`src/app/core.rs:424-470,552-570`; `zig/src/simulation.zig:112-124`).

Tray source/destination are world names or `NONE`. Missing selection: strength `-`, status `SELECT SOURCE AND TARGET`, disabled. Valid preview: decimal strength, `COMMAND READY - EXPLICIT LAUNCH REQUIRED`, enabled. Rejection: strength `-`, exact rejection label, disabled (`src/app/core.rs:435-470`; `src/ui.rs:50`). Field controls with no selection are disabled with `Select a Union world`; valid decrease/increase use `Decrease selected field (Q)` / `Increase selected field (E)`; invalid preview uses its rejection label (`src/app/core.rs:552-570`; `tests/usability.rs:350-425`).

| Authority rejection | Exact label (`src/app/core.rs:1021-1034`) |
| --- | --- |
| WrongTick | `REJECTED - WRONG TICK` |
| DuplicateSequence | `REJECTED - DUPLICATE COMMAND` |
| CampaignFinished | `REJECTED - CAMPAIGN FINISHED` |
| InvalidWorld | `REJECTED - INVALID WORLD` |
| SameSourceAndTarget | `REJECTED - CHOOSE ANOTHER TARGET` |
| SourceNotOwnedByIssuer | `REJECTED - SOURCE IS NOT UNION` |
| InsufficientEnergy | `REJECTED - INSUFFICIENT ENERGY` |
| FieldAtLimit | `REJECTED - FIELD AT LIMIT` |

UI hit testing and focused activation enforce `enabled`; direct action/command dispatch does not reapply that check (`src/presentation/ui.rs:161-165,504-508`; `src/app/core/session.rs:118-145,38,67-68`). A disabled control is presentation policy, not a canonical enqueue predicate. Two individually valid previews can compete for the same energy once executed in sequence; preview never reserves their resources.

Proposed allocation-free tray: copied `WorldName` values (or nullable names), nullable numeric strength, static status/tooltip slices and enabled booleans. Later UI formats strength into caller-owned storage. Do not return `WorldName.asSlice()` pointing into a temporary copied `CampaignView`; `asSlice` borrows its receiver (`zig/src/model.zig:11-35`; `zig/src/classic_app.zig:73-85,130-132`).

## Queue, sequence and failure semantics

The sole queue path checks **Playing only**, computes immutable `preview_valid`, attempts `enqueueNext(current_sequence, command)`, then wraps the sequence on success. A gameplay-invalid preview is still enqueued; the tick records the rejection. Preview-valid successful Launch clears `T` and yields FleetLaunched observation; preview-valid successful TuneField yields FieldTuned. Invalid preview or failed enqueue yields neither event; invalid-preview successful Launch leaves `T` intact (`src/app/core.rs:767-787`). Paused/onboarding hold do not block queueing; no new gate based on those states or 3A's Zig failure hold follows from Rust source.

The Zig authority admits only unfinished campaign, current tick and unique pending sequence, then fallibly appends; it does not inspect gameplay validity (`zig/src/simulation.zig:99-109`). Propagate `OutOfMemory`, DuplicateSequence and other enqueue errors distinctly; consume no sequence and preserve the prior queue/authority on failure. Use `+%= 1` on successful enqueue. Do not skip colliding wrapped sequences or automatically retry an allocation failure. A wrap to a pending sequence may therefore fail until the queue drains; canonical execution sorts by unsigned sequence (`zig/src/simulation.zig:145-161`). Existing authority tests pin invalid-command canonical rejection, duplicate preservation and allocation preservation (`zig/src/simulation_tests.zig:415-467,565-602`); they were read, not run here.

## Smallest ownership seam

Use a standalone `classic_commands.zig` module/zero-state controller. Every mutation takes `*AppCore`; every projection takes `*const AppCore`. Retain 3A's existing ID/field storage and restart reset owner. Store no `*AppCore`, `*Simulation`, `*World` or callback context pointing back into the movable owning core. The current core is returned by value and explicitly move-only by convention (`zig/src/classic_app.zig:88-120`); an embedded back-pointer would not track a move.

Add only narrow AppCore methods for `previewCommand`, `pendingCommandCount` and mode-gated `enqueueGameCommand`, keeping `_simulation` and `_next_sequence` internal to AppCore. Queue result should distinguish mode refusal from actual enqueue error and carry the consumed sequence plus pre-enqueue `preview_valid` on success. Keep missing selection as a controller dispatch result rather than inventing an authority error. Do not expose mutable authority/queue slices. A field/launch action maps IDs into `model.GameCommand` then uses this one queue seam (`zig/src/model.zig:40-49,143-152`).

Return bounded value effects for later composition: `clear_ui_focus` plus optional UnionWorldSelected / LaunchPreviewed / FleetLaunched / FieldTuned observation. Preserve the operation-specific rules above, especially helper selection/hover versus observed tap/cursor paths. 3E consumes effects synchronously and owns onboarding step changes, persistence, speed release and focus identity; 3B implements none of those. FleetLaunched/FieldTuned observations are based on preview-valid successful **enqueue**, not tick acceptance; selection/hover effects follow their own rows (`src/app/core.rs:678-689,714,723-727,771-783,985-992`). Pure preview emits none.

Successful scenario replacement/restart clears all three IDs, resets atmosphere and sequence zero; failed admission preserves them and queued intent. Mode changes, pause and suspend/resume retain command-selection state (`zig/src/classic_app.zig:171-200,261-282`). Keep that reset in existing AppCore code; an empty controller needs no separate reset hook.

## Minimal meaningful 3B tests

1. Table-driven valid-ID mouse/helper/hover/touch transitions, including friendly replacement, empty clear, captured source, source==target, hover-overrides-touch and hover-null fallback. Assert no enqueue from selection/hover/touch. Pin keyboard hover-only versus contextual fallback (`tests/app.rs:34-63,177-199`; `tests/usability.rs:130-203`).
2. Editing/Settings reject Playing-gated selection/hover/tap/field/queue; direct clear remains ungated. HUD-consumed helper selection is unchanged while HUD-consumed hover clears H. Test paused and onboarding-held Playing still queues. Test proposed unknown ID admission preserves controller state; separately raw unknown-ID commands enqueue and tick-reject.
3. Tray/field projection covers missing source/target, valid numeric strength, same-target, nonowned, insufficient-energy, upper/lower field limits and finished campaign; compare exact labels. Around repeated preview/tray calls assert canonical fingerprint, complete queue contents, sequence, all IDs, field and allocator allocation count unchanged (`tests/usability.rs:350-425,531-542`; `zig/src/simulation_tests.zig:436-467`).
4. Explicit launch/field dispatch covers valid success, gameplay-invalid successful enqueue and canonical tick rejection, with consumed sequence and touch-latch/event differences. Two commands valid at preview can yield one rejected at tick; no reservation/retroactive UI observation rollback.
5. Force sequence max success to zero, then a duplicate collision: no consumption, no queue loss, no observation. Finished enqueue likewise consumes nothing. Right-click missing source/destination is inert; right-click failed enqueue retains its pre-enqueue H/T update.
6. Inject failures across first queue allocation and queue growth with existing commands. Verify explicit OOM, queue bytes/order and authority unchanged, sequence unchanged, no observation, retry consumes exactly once, and no leak. Keep this separate from 3A step/report OOM tests (`zig/src/simulation.zig:99-105`; `zig/src/simulation_tests.zig:565-602`).
7. Exercise successful restart and rejected replacement after nondefault IDs/field/queue/sequence: success clears according to existing reset; failure preserves. Move the core value before controller operations to witness absence of stored self pointers. Effects should report source-derived events without implementing onboarding progress.
