# Workshop Library: the product route for archive and pack transfer and save-slot management

Date: 2026-09-08

Status: Design. Authorized by
`docs/superpowers/reviews/2026-09-04-workshop-library-addendum-review.md`, which
granted "APPROVE FOR IMPLEMENTATION PLANNING" and was explicit that it "does not
claim that any Library/store/transfer source has been implemented, tested, built,
or exercised." Nothing here is implemented. It closes
`docs/superpowers/reviews/2026-09-04-workshop-v1-baseline-review.md` Finding 2
only when built.

Binding contract: `docs/superpowers/specs/2026-09-04-nyon-workshop-library-addendum.md`,
cited below by section.

## Why this blocks more than it looks like it does

Finding 2 is the baseline review's only P1, and it is a route gap rather than a
logic gap. `WorkshopAction::RequestExport`, `RequestImport` and `RequestLoad` are
declared at `src/workshop/session.rs:35-37`, and `WorkshopSession::take_exported_archive`
at `session.rs:330` **has no caller anywhere in `src/`** — its only occurrence in
the whole source tree is its own definition. `WorkshopViewAction` offers thirteen
variants, all navigation, scroll and drawer.

The consequence reaches past the feature. Two-System Forge steps 16-18 are
save/quit/continue and export/re-import of both pack and archive, so the
qualification plan's Task 3 cannot execute a single one of them through the
product, and every matrix row's mandatory imported/exported archive hash field is
unfillable. The qualification plan forbids fixing it in place: a product gap
returns to the owning child plan. This design is that return.

## The correction that matters most

The obvious reading — "the APIs exist, so this is wiring" — is wrong, and the
addendum says so. Its §4 states that `RequestLoad` and `RequestImport` are **not
Library entry points**, because they decode using the resident catalog. Wiring
`Open` to `RequestLoad` would compile, pass a naive test, and ship a defect.

Slot management and active-session export are wiring. `Open`, `Import archive`,
`Set Continue` and row `Export` are not: they need an exact-catalog resolution
path that does not exist yet.

## Screen, not drawer section

Library is `ClientScreen::Library`, a sibling of `Settings`. Four independent
reasons, each checked against the tree:

1. The addendum requires main-menu entry, and at `MainMenu` there is no active
   Workshop session, so `build_workshop_frame` bails to the shell. A drawer
   section cannot exist there at all.
2. "Closing it returns to the exact prior screen" is verbatim the existing
   `open_settings` / `close_settings` / `settings_return` pattern in
   `src/app/client_runtime.rs`. Library reuses it with a `library_return` field.
3. The Navigator tab grid is hard-coded to four sections with a fixed header
   height. A fifth tab reflows the header in every mode and perturbs Inspector
   and Hierarchy geometry — a large regression surface for an unrelated feature.
4. Compact has no room for a third top-bar button: at the 320 floor, `Create` and
   `Navigator` already sit adjacent to the main-menu control.

**`library_return` must not call `prepare_for_durable_transition`.** Returning to
the main menu forces pause and save; Library is a lateral route, and the addendum
requires the resident session and its recovery obligations to survive opening it
untouched. Gating happens per action inside Library, not at the door.

In-Workshop entry is one new control on `SaveStatusModel`, beside the existing
save control, which already has both a docked and a Compact bottom-bar placement.
It inherits a proven two-mode story rather than inventing chrome.

## What exists, and what genuinely does not

Wiring, all present: `SlotSummary` already carries exactly the six fields the
addendum's row requires; `ListSlots`, `RenameSlot` and `ArchiveSlot` exist in all
three adapters with typed errors for every UI state (`SlotCapacity`,
`ArchivedSlot`, `StaleGeneration`, `NoValidGeneration`, `Busy`); active-pack
import and export exist on the client runtime; `poll_catalog_import` is a working
async store state machine to copy; `VisibleWindow` and `reveal_action` give
virtualized rows; the modal, its focus trap and its scrolling variant all exist.

Genuinely absent, each verified:

- **`UnarchiveSlot`** — zero occurrences in `src/`. Needed in all three adapters.
- **Generation-checked `SelectContinue`** — today it is `SelectContinue { slot }`
  with no expected generation. The addendum requires a compare-and-swap inside
  the same serialized mutation, and requires commit and promotion to clear
  `selected_continue` atomically.
- **A retryable catalog-import state** — the current import path calls
  `enter_recovery` on failure, replacing the screen. Library cannot offer
  "Retry / Cancel" over a runtime that has already switched to
  `RecoverableError`, and the addendum forbids collapsing Library failures into
  the global recovery screen.
- **`WorkshopLibraryClient`** — the exact-catalog open algorithm. It is an
  extraction of the existing continue-bootstrap phases, which the addendum asks
  to reuse rather than duplicate.
- **The transfer protocol** — nothing named transfer, picker, blob or download
  exists anywhere in `src/`.

## Journeys

**Export.** Two stages, deliberately. Stage 1 prepares bytes into a `Ready`
state; stage 2 is a **separately focusable** control that performs the platform
handoff, because the addendum requires a second direct user activation and
reusing one control instance would let a queued repeat activation fire the
handoff without a fresh gesture. Labels are authority-bound: while the Workshop
is not continue-ready the export is labelled portable-but-not-saved, and on the
browser path the terminal label is "download started", never "saved", because
consumption is not observable.

**Import.** Validate envelope, read the declared catalog hash, resolve the exact
pack, decode and hash-check, replay. A missing exact pack is a **retryable state,
not a failure**: the panel offers `Import pack` and `Cancel`, retains the archive
bytes bounded, and resumes without a re-pick once the pack stores durably. On
success the Workshop installs paused, slotless, dirty and ineligible for
Continue.

A naming consequence worth stating rather than discovering: the first durable
save of an import always creates a slot named "Workshop", so "save imports as
distinct slots" is satisfied by **save-then-rename in Library**. The alternative,
an optional slot name on the durable-save intent, is a session-contract change
and must not be smuggled in.

**Slot management.** Open, Rename, Archive/Unarchive, Set Continue, Export. The
resident slot cannot be archived, with a visible reason. Row identity derives
from `SlotId`, never a list index.

## Layout

Wide and Medium get a docked action panel in `right_panel`. **Compact returns no
right panel at all**, so its action surface is `drawer_sheet`, which is computed
for Compact and is the existing idiom for the Navigator and Creator drawers. Row
selection in the canvas opens the sheet, so pointer users get one gesture.

The transfer strip cannot be a Compact bottom bar: four labels plus a scroll pair
exceed 320 outright, and an oversized fixed label is **rejected, not truncated**
— the SDF batch refuses the frame. In Wide and Medium the strip must reuse the
timeline's probe-then-reserve algorithm so a squeezed panel degrades into a
scrollable strip rather than silently dropping controls. Silently dropping
controls is exactly baseline Finding 5.

**Measured 2026-09-16 (task 8, slice 3): the docked strip cannot be squeezed.**
The Library's transfer strip lives in `bottom_bar`, which spans the whole
window, and a docked layout needs at least 900 logical pixels, so four
controls always share one line; a scroll pair would be unreachable from any
real frame and was not built. The Finding 5 guarantee is kept by a sweep of
every docked width at every scale
(`every_docked_width_places_every_transfer_control_and_action_on_screen`) and
a debug assertion in the builder, both mutation-checked. Revisit if the strip
moves into a panel, gains controls, or its labels grow.

**Slot names are the only user-authored string on screen** and may be 64 bytes,
far wider than any panel. The row control takes `SemanticRole::Option`, which
routes it through the variable-label path to single-line ellipsis. Any other role
makes a long slot name reject the entire frame.

**One inherited conflict to decide rather than absorb.** Modal body rows are 40
high and the footer is 40, and `tests/workshop_ui_layout.rs` codifies that
exception as `>= 39.99`. Reusing the modal for Library confirmations inherits
sub-44 controls. The recommendation is to raise both to 44 as a separate first
task and update those two assertions, so the regression is attributable to a
commit that did nothing else.

## Deferrals this design respects

The native file-picker adapter is deferred pending a compatibility spike, and the
browser Object-URL cleanup is deferred past the synchronous click. The design
depends on neither: the transfer protocol is an object-safe trait with bounded
jobs, and with **no adapter installed** stage 1 still runs, the panel still
reports `Ready`, and stage 2 renders **disabled with a visible reason** — keeping
its box, focus slot and 44×44 target, so layout, focus order and SDF budget are
under test before any adapter exists. A test adapter closes the loop end to end
without touching a platform API.

**The one deferral this cannot route around:** the addendum gates *both* adapters
on the native spike, so the in-product byte handoff is not live until that spike
is accepted. Everything else — slot list, Open, Rename, Archive, Unarchive, Set
Continue, active-pack import and export, save-then-rename — is live without any
adapter. Making the browser path live earlier is an amendment to the addendum and
needs its own authority; it is not resolved here.

## Task order

Tasks 1-4 are addendum prerequisites and are not the screen.

0. Modal control geometry to 44, updating the two codified assertions.
1. Retryable catalog-import state replacing the `enter_recovery` calls.
2. `UnarchiveSlot` across all three adapters.
3. Generation-checked `SelectContinue` plus atomic Continue-clear on commit and
   promotion.
4. Extract `WorkshopLibraryClient` from continue-bootstrap and migrate startup
   Continue onto it.
5. Generalize modal presentation to take a semantic tree rather than the Workshop
   model.
6. `ClientScreen::Library`, return handling, and the poll machine over the four
   slot requests.
7. `src/ui/library.rs` — model, intents, focus order. **Not** added to
   `src/ui/workshop.rs`, which is 1,987 lines, is Finding 9's named example, and
   is being split.
8. `src/ui/platform/library.rs` and dispatch: docked panel, Compact sheet,
   transfer strip on the probe pattern.
9. Library confirmations on the shared modal; rename via a text-input field.
   **9a landed 2026-09-16:** Archive and Unarchive confirm in a dialog whose body
   names the save and says whether Continue will be cleared. The body must fit
   without paging (the Library has no dialog paging), which at 320x460 and 1.3
   limits it to about seven lines, four of them a 64-byte name.
10. In-Workshop entry control in both placements.
    **⚠️ GAP FOUND 2026-09-09 while building task 6: this task list has NO task for the
    MAIN-MENU entry control, and the addendum §2 requires main-menu entry.** Task 10 is
    the in-Workshop control only. Task 6 satisfies the constraint **at the runtime seam
    only** — `open_library()` succeeds from `MainMenu` and is tested there — but nothing
    routes to it, so the screen is currently unreachable in the product from the main
    menu. Task 6 deliberately did not add `MainMenuRoute::Library`, because that drags in
    `menu_copy` / `menu_slug` / icon arms and every `menu_capabilities()` assertion, and
    this design assigns entry controls to their own tasks rather than to the runtime.
    **Assign it to task 8's dispatch or to a new task before the screen ships**, or the
    two-slice plan below ships a screen no user can open.
11. Transfer protocol trait, bounded jobs, test adapter, two-stage export wired
    to the existing byte sources.
12. Enable Open, Import archive, Set Continue and row Export on the library
    client.

Ship the screen after tasks 1-4, in two slices: the wiring subset first with the
client-dependent controls **rendered disabled with a visible reason**, then
enabled. Silently omitting them is not acceptable — Finding 5 is precisely about
controls that exist logically but cannot be reached.

## The dispatch-compensation rule, and the qualifier that carries it

Established by task 6 (`5668455`) and refined by its re-review (`2851391`). Recorded
here because tasks 7-12 will cite it, and the short form is dangerous on its own.

**The rule:** the moment a request reaches the store is the last moment its effect is
knowable. `WorkshopStore::abandon` drops the *outcome*, not the *work* — a mutation
that was going to land still lands — so **compensation belongs at dispatch, never at
success**. Task 6 puts the residency gate and the Continue-candidate withdrawal in
`dispatch_slot_request`, the single `workshop_store.start` for slot requests, so all
four edges (first attempt, retry from a retained `Failed`, cancel of an in-flight job,
and a forgotten `StoreJobState::Unknown`) are covered by construction rather than by
enumeration.

**⚠️ The qualifier, which the short form drops:** compensating at dispatch is safe
*because withdrawing a Continue candidate is cheap and idempotent*. It is **not**
safe for state that is expensive or user-authored. Applied without thought to retained
rename text or prepared export bytes, it discards the user's work on a request that is
merely *refused* — and task 6 already hit the near miss: moving its guard to dispatch
would have swallowed the retained request, turning "silently succeed" into "silently
vanish", until it restored the retained `Failed` on refusal.

So: **compensate at dispatch when the compensation is cheap and idempotent; otherwise
preserve at dispatch and compensate on the outcome you can observe.**

## Verification

Store suites, in all three adapters: unarchive is a flag-only mutation with
byte-identical preservation and no Continue restoration; compare-and-swap
succeeds at head and returns `StaleGeneration` otherwise, inside the same
transaction as the write; commit and promotion clear Continue atomically; the
list-then-concurrent-commit race rejects; capacity counts archived slots.

Client suite: Library round-trips from all three prior screens with no durable
transition side effect; the gating matrix holds with a dirty resident session;
store failure yields Retry/Cancel and never `RecoverableError`; cancel never
deletes an already-stored pack; an imported Workshop is paused, slotless, dirty
and not Continue-eligible.

Layout and SDF suites: every control at least 44×44 across three modes and three
UI scales at both the 320 floor and 1440×900; the batch builds `Ok` for every
frame and every dialog, including sixteen slots with 64-byte names.

Accessibility suite: pointer hit-test and keyboard traversal both reach row 16 of
16; every action has both a pointer target and a focus slot; the modal traps and
restores focus.

Not covered by any suite, and to be stated as such: live native picker
interaction, live browser download handoff, and the named cross-platform browser
matrix.
