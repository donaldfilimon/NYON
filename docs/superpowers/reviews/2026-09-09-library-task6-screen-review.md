# Review — `6e0cec9` Library task 6: the Library screen, its return handling, and its slot machine

**Status: REQUEST CHANGES**

Two witnessed violations of the addendum's own safety invariants, both in code this
commit introduces, both the *same defect the commit set out to close* reappearing on a
different edge of the same state machine: the archive-versus-residency gate and the
archive-versus-Continue-candidate compensation are attached to one transition each
instead of to the request's lifecycle. Finding 1 lets an archived slot become the
resident Workshop (§10). Finding 2 archives the resident Workshop's own slot (§3).
Both are reproduced with failing tests at HEAD, quoted below.

Everything else in the diff is good, and unusually so. The four design constraints hold,
the M4 hazard the implementer self-reported is real and fixed at the right layer, the
`Cancel` list-drop reasoning is correct and is the store's own written contract, and the
thirteen new tests are of a materially better shape than the four wrong-reason
assertions this repository has produced today — several of them carry explicit *controls*
proving the precondition they depend on is genuinely in force. The tightened M4
assertion was checked in isolation and does discriminate.

Reviewer: Abbey (Claude Opus 5). Reviewed at `HEAD = 6e0cec9`, working tree clean before
and after; every mutation and scratch test below was reverted with `git checkout --` and
the suite re-verified green (36/36 in `workshop_client`) before this file was written.

---

## Evidence classes

- **By running** — gates, counts, mutations M1/M4/M8 (both directions), the M4
  discrimination check, and the three new failing witnesses in Findings 1, 2 and 3.
- **By reading** — the four design constraints, the store's `abandon` and `ArchiveSlot`
  contracts, the two task-7 contracts, the deferral, and the test-shape audit.
- **Inferred / not checked** — stated explicitly in *Not verified* at the end.

## Gates run (by running)

Exit codes were read out of each log, not through a pipe or a trailing `echo`.

| Command | Result |
|---|---|
| `cargo fmt --all --check` | `EXIT: 0` |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `EXIT: 0`, 0 warnings. Re-run after `touch`ing the three changed sources so the check genuinely recompiled (`Checking nyon v0.1.0` present) rather than replaying a cached fingerprint. |
| `cargo test --workspace` | `EXIT: 0`, **654 passed / 0 failed** across 47 `test result:` lines. |

**Arithmetic verified.** 654 includes the 3 `nyon` doc-tests; excluding them gives **651**,
matching the implementer's report, and the same convention gives 641 − 3 = 638 at the
baseline. Independently: `git show 6970490:tests/workshop_client.rs` has **23** `#[test]`
functions, HEAD has **36**, and `git diff --stat 6970490 6e0cec9 -- tests/` shows
`tests/workshop_client.rs` is the only test file touched. **+13 tests exactly, 638 → 651,
suite count unchanged.** The claim is accurate.

## Constraints (by reading, with one confirmed by running)

1. **`library_return` must not prepare a durable transition — HOLDS.**
   `return_to_main_menu` (`src/app/client_runtime.rs:1241`) calls
   `session.prepare_for_durable_transition()`; `open_library` (`:950`) does not, and
   `close_library` (`:977`) writes only `self.screen`. `prepare_for_durable_transition`
   (`src/workshop/session.rs:296`) enqueues `Pause` + `RequestSave`, so the copy-paste
   failure mode is observable as a cleared dirty flag *and* an occupied Commit lane —
   M1 below trips both.
2. **Reachable from `MainMenu` — holds at the runtime seam only.** `open_library` admits
   `MainMenu | ClassicSector | GalaxyWorkshop` (`:951-957`). See Finding 8 for the product
   reachability gap, which is disclosed rather than hidden.
3. **Failure never enters `RecoverableError` — HOLDS.** Every arm of `poll_library_slots`
   (`:1141`) reaches `fail_slot_request` (`:1163`) → `retain_failed_slot_request` (`:1172`);
   `enter_recovery` is not called on any Library path. Grep for `self.screen = ` shows the
   Library machine writes the screen nowhere.
4. **Exactly the four requests — HOLDS.** `ListSlots`, `RenameSlot`, `ArchiveSlot`,
   `UnarchiveSlot`, and `finish_slot_request` (`:1187`) refuses any result shape that does
   not match the exact request that asked, including a right-shaped answer naming a
   different slot.

---

## Findings

### 1. HIGH — Cancelling an in-flight `ArchiveSlot` leaves the validated Continue candidate installed, so an archived slot becomes the resident Workshop

`src/app/client_runtime.rs:1077-1091` (`cancel_library_slot_request`), against
`src/app/client_runtime.rs:1215-1221` (the candidate withdrawal inside `finish_slot_request`).

The commit correctly identifies that the store clears its own Continue marker on Archive
(`src/workshop/store/memory.rs:249-257`) but cannot reach a `LibraryCandidate` the runtime
already holds, and adds the withdrawal on the **success** edge. `abandon` is documented as
dropping *the outcome, not the work* — "a mutation that was going to land still lands"
(`src/workshop/store.rs:448-453`, with the re-list obligation at `:461-463`) — which is precisely the reasoning the same author used,
correctly, to justify dropping the cached slot list on cancel. That reasoning applies
identically to the Continue candidate, and it was not applied. Cancelling an in-flight
`ArchiveSlot` therefore archives the slot while leaving the candidate live, so
`continue_available()` stays true and `continue_selected_workshop` (`:1263`) installs it.

Witnessed at HEAD (scratch test appended to `tests/workshop_client.rs`, since reverted):

```
runtime.archive_library_slot(slot).unwrap();
runtime.cancel_library_slot_request().unwrap();   // before any update()
runtime.refresh_library_slots().unwrap(); settle_library(&mut runtime);
assert!(slot_archived(&runtime, slot));           // PASSES — the archive landed
assert!(!runtime.continue_available());           // FAILS
```

```
panicked at tests/workshop_client.rs:2175:
an archived slot must not remain openable through a retained candidate
```

With that first assertion removed, the consequence is the full §10 violation, not merely a
stale flag:

```
assert_eq!(runtime.select_menu_route(MainMenuRoute::Continue), Err(ContinueUnavailable));
  left: Ok(None)          <- Continue succeeded and installed the archived slot
 right: Err(ContinueUnavailable)
```

Addendum §10: *"No archived slot is opened or selected for Continue."* This is the exact
invariant the success-path clear exists to protect.

**Suggestion:** attach the withdrawal to the request's lifecycle, not to one transition —
e.g. a single `fn withdraw_candidate_for(slot)` called from `finish_slot_request`'s archive
arm **and** from the `Working { request: ArchiveSlot { slot }, .. }` arm of
`cancel_library_slot_request`. A `Failed { ArchiveSlot }` produced by
`StoreJobState::Unknown` deserves the same conservative treatment, since the store forgetting
a job says nothing about whether the write landed.

**Status:** open.

### 2. HIGH — `retry_library_slot_request` bypasses the §3 resident-slot refusal, so a retried Archive can archive the resident Workshop's own slot

`src/app/client_runtime.rs:1051-1053` (`retry_library_slot_request` calls
`dispatch_slot_request` directly) against `src/app/client_runtime.rs:1033-1042`
(the guard lives only in `archive_library_slot`).

The guard's own doc comment justifies refusing at request time with "no retry cures it
while the session is still resident" — which silently assumes residency cannot change
between the failure and the retry. It can, by the most ordinary path there is: the archive
fails while the slot is *not* resident, the user closes the Library, opens that slot through
Continue, and returns. `open_library` deliberately does **not** auto-list from a retained
`Failed` (`:966-968`), so the stale decision is still sitting there, and Retry re-dispatches
it with no gate.

Witnessed at HEAD with an `ArchiveFaultStore` that fails only the first `ArchiveSlot`
(scratch, since reverted):

```
panicked at tests/workshop_client.rs:2230:
Retry archived the resident Workshop's own slot: Ok(())
```

Addendum §3: *"The currently resident slot cannot be archived through the Library. Doing so
would leave a live authoritative session whose next save must fail as archived."*

**Suggestion:** move the residency check into `dispatch_slot_request` (`:1124`), where every
path — first attempt, auto-relist, and retry — passes through it. A retry that has become
illegal should not silently succeed *or* silently vanish; refusing it back to the caller
with `ResidentSlot` and leaving the row un-archived is the honest answer.

**Status:** open.

### 3. MEDIUM — A catalog import completing inside `update()` replaces the Library screen

`src/app/client_runtime.rs:867` (`finish_catalog_import` → `install_new_workshop`) and
`src/app/client_runtime.rs:897` (`self.screen = ClientScreen::GalaxyWorkshop`), against
`open_library`'s stated reasoning at `:945-948`.

`open_library` refuses `Loading` and `RecoverableError` on the explicit grounds that they
"are owned by machines that write `self.screen` themselves, so a Library opened from either
would be silently replaced." That reasoning is right and incomplete: `poll_catalog_import`
runs from `update()` on every frame and, for an import started with a seed, calls
`install_new_workshop`, which writes `self.screen`. The Library is opened from a screen the
route allows and is then replaced under the user, with `library_return` stranded.

Witnessed at HEAD (scratch, since reverted):

```
runtime.begin_new_workshop_from_catalog(&custom_pack(), 7).unwrap();
runtime.open_library().unwrap();
runtime.update(Duration::ZERO);
assert_eq!(runtime.screen(), ClientScreen::Library);
  left: GalaxyWorkshop
 right: Library
```

Severity is MEDIUM rather than HIGH only because nothing routes to `open_library` yet
(Finding 8), and no data is lost — the user is moved, not corrupted.

**Suggestion:** either refuse `open_library` while `catalog_import_active()`, or have
`install_new_workshop` respect a Library that is currently on screen by writing
`library_return` instead of `screen`. Whichever is chosen, the enumeration in
`open_library`'s doc comment should name the third screen-writing machine so the next
reader does not re-derive the same incomplete list.

**Status:** open.

### 4. MEDIUM — The new shell arm ships with zero UI qualification coverage

`src/ui/platform/shell.rs:216-229`, against `tests/workshop_ui_layout.rs:491-496` and
`tests/workshop_ui_layout.rs:594-598`.

Both shell sweeps enumerate screens **by hand** — `MainMenu`, `Settings`,
`RecoverableError` — and neither was extended with `ClientScreen::Library`. So
`shell_chrome_qualifies_at_every_required_viewport_and_scale` and the action-witness sweep
never build the new arm, and the commit message's claim that this is "enough that the screen
is not a dead end for pointer users" has no test behind it. The risk of an actual defect is
low: the geometry is `centered_button(viewport, 0)` (`shell.rs:360`, 360×48, clearing the
44 px minimum) — byte-identical to the Settings arm's first control, which is qualified. The
defect is that the sweep's blind spot is now one screen wider, and the next arm added here
inherits it.

**Suggestion:** add `ClientScreen::Library` to both screen lists. Two lines, and it converts
the commit message's claim into evidence.

**Status:** open.

### 5. LOW — `library_slots() == None` with status `Idle` now has two meanings, and the documented one is stated as if exclusive

`src/app/client_runtime.rs:168-173` (the `LibrarySlotsStatus::Idle` doc) and
`:983-988` (the `library_slots` doc), against `:1077-1091`.

The doc says `None` with `Idle` "means the Library has not listed yet, which is distinct
from having listed an empty store." After cancelling a mutation the runtime is `Idle` with
`slot_list == None` and **has** listed — and no re-list is dispatched, so within one visit
the panel stays empty until the user presses Refresh. (Re-entering the Library re-lists,
because `open_library` auto-lists from `Idle`, which limits the blast radius.) Task 7 is
explicitly told to rely on this three-state distinction, so a doc that is false in one of
the three cases is a defect worth fixing now rather than after the panel is built on it.

**Suggestion:** either dispatch the re-list from the cancel path — it is a `LoadOrImport`
request and cannot contend with the `Commit` lane the cancel just released — or amend both
doc comments to name the second case.

**Status:** open.

### 6. LOW — The resident-slot refusal reports a diagnostic code that contradicts its error

`src/app/client_runtime.rs:1034-1040`.

The refusal returns `ClientRuntimeError::ResidentSlot` while pushing
`ClientDiagnosticCode::RouteUnavailable`, the same code `open_library` pushes for "you asked
from the wrong screen." A UI reading diagnostics rather than the `Result` cannot tell the two
apart, and the user-visible reason for a refused Archive is materially different from a
refused route.

**Suggestion:** a distinct code, or at minimum a note explaining why the collision is
acceptable.

**Status:** open.

### 7. LOW — The "a `Failed` survives a visit" contract task 7 must honour has no test

`src/app/client_runtime.rs:962-966`. Verified by reading — `open_library` dispatches its
auto-list only `if matches!(self.slot_requests, LibrarySlots::Idle)`, so a `Failed` left by a
previous visit is preserved and must render as the pending decision it is. No test closes and
re-opens the Library across a retained `Failed`. This is exactly the property Finding 2
exploits, so it is worth pinning in both directions.

**Suggestion:** one test: fail a rename, `close_library`, `open_library`, assert the status is
still `Failed { Rename, .. }` and that no list was dispatched.

**Status:** open.

### 8. INFO — The main-menu entry deferral is correctly deferred and correctly disclosed

`grep` finds **no caller of `open_library` under `src/`** — only `tests/workshop_client.rs`.
`MainMenuRoute` (`src/app/client_runtime.rs:72-80`) has no `Library` variant, and
`menu_copy` / `menu_slug` (`src/ui/platform/shell.rs:369-396`) have no arm for one. The
Library screen is therefore unreachable in the product today, from the main menu **and** from
the Workshop.

This is an acceptable deferral, not a hidden gap, on three grounds: the design's task list is
annotated with the gap in its own ancestor commit `1c8a73f` ("docs(design): record the missing
main-menu entry task"), the design's shipping plan is explicitly two-slice with the screen
shipped after the wiring subset, and adding `MainMenuRoute::Library` here would drag in every
`menu_capabilities()` assertion for a control with nothing behind it. The condition that would
make it a defect is stated in the design itself and should be enforced at review of task 8 or
10: **the screen must not ship with no entry control.**

---

## Mutation evidence

Reproduced, each with `git checkout -- <file>` afterwards and `Compiling nyon` confirmed in
the rebuild output, so no result rests on a stale artifact.

| # | Mutation | Claimed | Observed | Verdict |
|---|---|---|---|---|
| M1 | `open_library` calls `prepare_for_durable_transition` | 3 | **3** — `opening_the_library_prepares_no_durable_transition` ("Library must not clear the dirty flag"), plus `library_slot_mutations_proceed_while_the_resident_workshop_is_dirty` and `a_library_request_still_in_flight_survives_closing_the_library`, both `Err(Store(Busy { class: Commit }))` | **Confirmed, and for the right reasons.** One failure is the direct property (dirty cleared); the other two are the second observable consequence (the enqueued `RequestSave` seizes the Commit lane the Library's own mutations need). Three independent witnesses of two distinct properties. |
| M4 | archive keeps the Continue candidate | 1 | **1** — `archiving_the_validated_continue_candidate_withdraws_it` | **Confirmed.** See the discrimination check below. |
| M8a | cancel keeps the cached list (the reported direction) | 1 | **1** — same test, at "an abandoned mutation invalidates every generation the list held" | Confirmed. |
| M8b | cancel *always* drops the list (the unreported opposite direction) | — | **1** — same test, at "an abandoned list invalidates no generation" | **The single test pins both halves.** Worth recording: `cancelling_a_mutation_drops_the_cached_list_and_a_cancelled_list_does_not` is bidirectionally discriminating, which is what makes a single-witness mutation trustworthy here. |

**M2, M3, M5, M6, M7, M9 were not reproduced** (time). M2, M3 and M7 were checked by reading
and each has a mechanism that must fail: M2 is witnessed by
`library_returns_to_the_main_menu_a_resident_workshop_was_left_on:1677`, whose whole premise
is the one state where `self.screen` and `screen_for_active_session()` disagree — and that
difference is real, `return_to_main_menu:1241` sets `MainMenu` while leaving
`ActiveSession::Workshop` resident, so `screen_for_active_session:1355` would answer
`GalaxyWorkshop`. M7 fails through `settle_library`'s bounded-poll panic rather than an
assertion, which is a correct but indirect witness.

### The tightening in `archiving_the_validated_continue_candidate_withdraws_it` — checked, and it is genuine

The task flagged this as the highest-value thing to check, so it was checked directly rather
than inferred. With M4 applied, the test fails at `tests/workshop_client.rs:1935`
(`!continue_available()`), which is the *older* assertion — so the run alone does not prove
the tightened one discriminates. I therefore removed the earlier assertion and re-ran with M4
still applied:

```
assert_eq!(runtime.select_menu_route(MainMenuRoute::Continue), Err(ContinueUnavailable));
  left: Ok(None)
 right: Err(ContinueUnavailable)
```

The tightened assertion fails on its own, and it fails by observing the *strongest* form of
the property: Continue did not merely stay enabled, it succeeded and installed the archived
slot as the resident session. The comment now names a property the assertion actually
discriminates, and the surrounding conditions are right — the assertion is made from
`MainMenu`, where the route is otherwise available, and `ContinueUnavailable` is a distinct
error from the `RouteUnavailable` a wrong-screen refusal would produce, so it cannot pass for
the wrong reason. **The reported defect was real and the fix is correct.**

## Test-shape audit — the other twelve

Read against the wrong-reason pattern. No further instances found, and three tests carry
deliberate controls, which is the right habit:

- `library_slot_mutations_proceed_while_the_resident_workshop_is_dirty:1790` first asserts
  `start_new_workshop(1) == Err(RouteUnavailable)`, proving the session genuinely blocks
  replacement. Without that control the test would pass on a session that never blocked
  anything.
- `a_busy_commit_lane_becomes_a_retryable_library_failure:2014` first asserts
  `store.commit_pending`, proving the lane is really held before observing the `Busy` refusal.
- `the_resident_workshops_own_slot_cannot_be_archived:1837` archives an *unrelated* row after
  the refusal, proving the refusal is slot-specific and not a blanket failure.
- `a_library_store_failure_offers_retry_and_never_enters_recovery:1951` asserts the exact
  `Failed { kind, slot, code }` triple plus `screen() == Library` plus
  `recovery_diagnostic().is_none()` — three independent ways for an `enter_recovery`
  regression to be caught.
- `library_lists_slots_and_re_lists_after_every_successful_mutation:1740` pins the one-frame
  `None` + `Working { List }` window explicitly, which is the exact contract task 7 is warned
  about.

One structural note in the tests' favour: `FaultySlotStore:702` injects its failure through
`poll`, which is the path the runtime's own machine takes, rather than through `start`, which
would exercise a different arm.

## The two task-7 contracts — both verified by reading

- **`library_slots() == None` with `Working { kind: List }` is loading, not empty.** Correct:
  `finish_slot_request:1235-1236` sets `slot_list = None` and immediately dispatches
  `ListSlots`, so every successful mutation passes through exactly that pair for one frame,
  and `library_lists_slots_and_re_lists_after_every_successful_mutation:1758-1768` asserts it.
  An empty store lists as `Some` with empty `slots`. **But see Finding 5** — the `None` +
  `Idle` half of the same distinction is now ambiguous.
- **`open_library` auto-lists only from `Idle`.** Correct, `:966-968`. A `Failed` from a
  previous visit survives re-entry and must render as a pending decision. Untested
  (Finding 7), and load-bearing for Finding 2.

## Things the diff gets right and should not be "simplified" later

- `open_library` ignoring the auto-list's `Result` (`let _ =`) is safe rather than sloppy:
  `dispatch_slot_request:1124` routes a start failure into `retain_failed_slot_request`, so the
  refusal lands in the machine's own `Failed` state and surfaces as Retry/Cancel. The comment
  at `:963-965` explains exactly this. Same for the auto-relist at `:1236`.
- `finish_slot_request` rejecting a right-shaped result naming a *different* slot as a protocol
  failure (`:1223-1230`) is the difference between reporting a rename of the row the user
  activated and reporting a rename of some other row.
- Retaining the whole `WorkshopStoreRequest` as retry material, rather than a code plus a
  re-derivable argument, is what makes Retry not ask the user to retype a slot name.
- `slot_request_kind:281` being total over the request vocabulary rather than panicking on an
  impossible variant is the right call for a product path, and it is documented as such.

## Not verified

- **M2, M3, M5, M6, M7, M9 were not re-run.** M2/M3/M7 are supported by reading as above;
  M5/M6/M9 are not independently checked at all.
- **No WASM, browser or native store path was exercised.** The Library machine was measured
  only against `MemoryWorkshopStore` and the two test doubles. The native adapter's real
  asynchrony widens the cancel window Finding 1 depends on rather than narrowing it, but that
  is inference, not measurement.
- **No rendering, SDF budget or accessibility measurement of the new shell arm** — see
  Finding 4; there is no test to run.
- **The three witness tests in Findings 1, 2 and 3 were scratch and are not in the tree.**
  They are reproduced verbatim enough above to be re-derived; the working tree was restored to
  `6e0cec9` exactly and `--test workshop_client` re-run green (36/36) afterwards.
- **The claim that an archived resident slot's next save actually fails** (the harm behind
  Finding 2) is taken from addendum §3 and from `SelectContinue`'s documented
  `ArchivedSlot` ordering; the failing commit itself was not exercised.
