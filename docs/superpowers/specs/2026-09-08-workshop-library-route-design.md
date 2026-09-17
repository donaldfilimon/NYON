# Workshop Library: the product route for archive and pack transfer and save-slot management

Date: 2026-09-08

Status: Design. Authorized by
`docs/superpowers/reviews/2026-09-04-workshop-library-addendum-review.md`, which
granted "APPROVE FOR IMPLEMENTATION PLANNING" and was explicit that it "does not
claim that any Library/store/transfer source has been implemented, tested, built,
or exercised." It closes
`docs/superpowers/reviews/2026-09-04-workshop-v1-baseline-review.md` Finding 2
only when built.

**Implementation state, 2026-09-16:** tasks 0 to 10 have landed, and task 12's
three row slices (12a Open, 12b Use for Continue, 12c row Export to Ready) have
landed. Task 11 has landed **except its real adapters**: the transfer
protocol, its bounded jobs, the test adapter, the stage-2 handoff of the bytes
12c prepares, and (2026-09-17) all four transfer-strip routes: Export this
galaxy, Export content pack, Import content pack and Import galaxy, which is
task 12's Import archive (12d below). Still open: every real adapter, which
waits on the §8 compatibility spike, and with it any live picker or download;
the task 11 note says exactly what. The main-menu gap noted under task 10 is closed: `3b0e229` added
the entry. The notes under each task below say what landed and where it moved
from this plan.

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
Continue, save-then-rename, and stage 1 of row Export, Export this galaxy and
Export content pack — is live without any adapter. The two imports start by
choosing a file, so they are disabled with a visible reason until an adapter is
installed (measured 2026-09-17; this paragraph formerly listed active-pack
import as adapter-free). Making the browser path live earlier is an amendment to the addendum and
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
   **9b landed 2026-09-16:** Rename opens a dialog with a single-line name field,
   validated live through `SlotName`; an invalid draft disables Rename and shows a
   one-line notice, and a refused submit keeps the dialog and the draft. The
   `slot_changes_available` flag is gone.
10. In-Workshop entry control in both placements.
    **Landed 2026-09-16, with both placements moved from the plan above:** the
    docked entry sits in the top bar left of the guide control, because the
    right panel has no height to spare at 1280x480 and 1.3 (the Inspector title
    stopped fitting under Save); Compact's sits in the Navigator header left of
    Close, because its bottom bar is already full at the 320 floor. Focusing it
    in Compact opens the Navigator. Close and Escape return focus to it once.
    **RESOLVED 2026-09-16 by `3b0e229` (main-menu entry landed); the note below is the
    record.** **⚠️ GAP FOUND 2026-09-09 while building task 6: this task list has NO task for the
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
    **Landed in part 2026-09-16: everything that does not wait on the §8
    spike, for row Export.** `src/app/transfer.rs` holds the object-safe
    `TransferAdapter` (start, poll, abandon; one job at a time; no `Send`
    bound, so a browser adapter over `Rc` mailboxes can implement it), §7's
    vocabulary (`ChooseImport` with kind and maximum bytes, `HandOffExport`
    with kind, sanitized `SuggestedName` and bytes; Import Chosen, Export
    Handed Off, Cancelled, or a bounded `TransferFailureCode` that carries no
    platform text), the two suggested-name forms, and `HandoffOutcome`, whose
    four labels are the only words a finished handoff may use: only
    `DurablySaved` says "saved" (§1, §8, §9). `ScriptedTransfer` is the test
    adapter; no product entry point installs any adapter. The runtime holds
    `Option<Box<dyn TransferAdapter>>`. **Save copy** (`library.request.handoff`)
    now dispatches `hand_off_library_export`, live exactly when an adapter is
    installed and the lane is `ExportReady`. The handoff keeps the lane:
    in flight it is `Working { kind: HandOff }` with **Stop waiting**, which
    abandons the job and returns to Ready; a user-dismissed save returns to
    Ready without a diagnostic; a refused start, a failed job, a forgotten job
    or a wrong-shaped answer is `HandOffFailed`, which **keeps the bytes**
    (the dispatch-compensation qualifier's "prepared export bytes" case) and is
    retried only on the handoff's own control, never on the generic Retry; a
    finished handoff is `ExportHandedOff`, which releases the bytes, reports
    the outcome (with the recovered-predecessor label when it applies) and
    offers **Done**. The adapter cannot be replaced while it owns a job.
    **Split from the old single flag:** `LibraryUiContext::handoff_available`
    reads adapter presence. (The `transfer_available` flag this note once
    described was removed on 2026-09-17; see the next paragraph.)
    **Three strip routes landed 2026-09-17** (`58ca424`, with the export
    origin reshaped in `fcf89f5`). Each control now takes its own gate, facts
    the user can act on first and the missing capability last:
    - **Export this galaxy** queues the session's own `RequestExport` and
      takes the bytes with `take_exported_archive` right after the session
      update that drains it, so that function finally has a caller. The bytes
      are the in-memory state, dirty or not. They are labelled §10's
      `Portable export; Workshop not saved for Continue` unless the session
      was `continue_ready()` when they were taken; that predicate needs a
      paused session, so a saved but running Workshop still carries the
      qualifier (conservative by design). A slotless Workshop's copy is
      suggested as `Workshop.nyonworkshop.json`, the name its first save
      receives; a resident slot lends its listed name. A refusal (a
      replacement in progress) is a retryable Library failure; the archive of
      a cancelled queued export is dropped, never offered.
    - **Export content pack** is `export_active_catalog`'s canonical bytes,
      straight to Ready, named by `SuggestedName::content_pack`.
    - Both exports need an open Workshop and the lane, and **no adapter**:
      they reach Ready with Save copy disabled, exactly as "Deferrals" above
      describes. Neither takes a replacement gate; nothing is replaced.
    - **Import content pack** needs the lane and an adapter, and no
      replacement gate (§6 permits pack storage that replaces nothing). It
      asks for one pack, hands the bytes to the catalog-import machine, and
      shows that machine on the request strip: storing (Cancel), a store
      failure (Retry re-stores the retained pack without a new choice, and
      Cancel), and a stored notice (Done). A dismissed picker is ordinary; a
      failed or misbehaving picker is a `Transfer` failure; invalid content
      is a `Catalog` refusal whose **Choose again** is a new choice. Cancel
      never deletes a stored pack. Nothing reaches `RecoverableError`.
    - **Deviation from this note's earlier plan: the strip takes the lane.**
      Every strip route shows its progress, failure or prepared bytes on the
      one request strip, so all four take the lane reason.
    - **Import galaxy** takes §6's replacement gate, the lane and an adapter.
      Its route landed the same day as task 12d (below), which removed the
      interim `archive_import_available` flag and the deferred arm in
      `apply_library_intent`.
    Evidence: `tests/workshop_library_transfer_routes.rs`, the model tests in
    `tests/workshop_ui_library.rs`, the crowded-frame sweep over every new
    strip state in `tests/workshop_ui_library_frame.rs`, and
    `the_transfer_strip_routes_run_from_the_frame`; nine mutations of the new
    rules were each killed by a named test. All of it runs against
    `ScriptedTransfer`; no live picker or download is exercised.
    **Remaining after the §8 spike, or beside it:**
    - the native adapter (maintained dialog dependency chosen by the spike,
      size check before allocation, read at most max plus one, same-directory
      temporary write, flush, sync, atomic placement after overwrite
      confirmation, parent sync, and `Written`/`HandedToSystem` wording where
      that sequence cannot be proven), and the browser adapter (semantic file
      input, `File.size` check, Blob plus Object URL plus anchor during the
      activation, deferred cleanup), which the route design gates on the same
      spike; installing either in `app.rs` is what makes Save copy live in
      the product;
    - live native picker and browser download evidence, which no suite
      covers.
12. Enable Open, Import archive, Set Continue and row Export on the library
    client.
    **12a landed 2026-09-16: Open.** It runs on a second `WorkshopLibraryClient`
    (startup Continue keeps the first), and it occupies the Library's one
    request lane rather than running beside it, for two reasons: its
    `SelectContinue` needs the Commit lane a Rename holds, and its progress,
    failure and held candidate all belong in the request strip that already
    offers Retry and Cancel. So Open now takes the lane reason, which this plan
    and the task 7 model had excluded. Gated in the model and refused in the
    runtime on §6's replacement invariant (`ReplacementBlocked`) and on the
    resident Workshop's own slot (`AlreadyOpen`). Outcomes: a clean head
    installs only while the Library is on screen and replacement is safe,
    otherwise it is **held** and offered as Open (§4 item 9); an invalid head is
    always held and offered as **Open previous**, whose acceptance installs the
    predecessor so the session runs its existing promote-then-select
    obligation. That is how §6's "recovery of another slot" is reached; there
    is no separate repair control. A stale row, an archived row and a refused
    Continue compare-and-swap are one `StaleSave` state whose Retry is labelled
    **Refresh** and re-lists instead of repeating the stale request.
    **One deviation from §4:** a refused compare-and-swap does not keep the
    validated candidate. §4 says to preserve it, but it is a generation the
    store no longer calls the head, so nothing could install it, and §4's own
    remedy is "retry after refresh", which re-lists and replays anyway.
    Cancel from an open that reached `Selecting` can leave the Continue marker
    claimed, as `WorkshopLibraryClient::abandon` documents, so Cancel drops the
    cached list. **For a held clean candidate the marker has always moved**
    (selection ran before the hold), so Cancel from Held leaves it on that
    save, and the held message says so: "A saved galaxy is now the Continue
    save."
    **12b landed 2026-09-16: Use for Continue.** The same open path with a
    purpose tag (`OpenPurpose::SelectOnly`), meant to gain row Export as a
    third variant in 12c rather than a second flag. A clean head claims the
    marker and becomes the runtime's Continue candidate, so the Continue route
    installs it without a second replay, and the list is re-listed. An invalid
    head is held as the same Open previous offer, since installing is the only
    route that repairs it. **A second deviation:** it is refused while any
    Workshop is resident, not only while one cannot be replaced (§4, §6). A
    resident session tracks its own Continue selection and re-selects its slot
    whenever it saves, so a marker moved underneath it would disagree with what
    Continue returns to in this run and be undone by its next save. The main
    menu, before any Workshop opens, is where this control is live. Also folded
    in: a held open's Open control now takes the replacement gate. Row Export
    still reads `library_client_available`, which stays false.
    **12c landed 2026-09-16: row Export to Ready.** `LibraryOpen::Slot` now
    carries a `SlotIntent` (task 4 review Finding 2): `Export` never starts
    `SelectContinue` and, unlike `Open`, does not refuse an archived row, since
    task 7 review F5 keeps Export enabled there and §3 forbids opening an
    archived save, not reading it. `OpenPurpose::Export` is the third purpose
    the 12b note planned. Decisions recorded here: **no residency or
    replacement gate at all**, in the model or the runtime, because §4 says the
    export mutates nothing and §6 does not name it; exporting the resident's
    own row reads its stored generation, not the in-memory session, and the
    active-session export stays a separate transfer control. **It takes the
    lane**, through Ready: the prepared bytes stay in `LibrarySlots` until the
    user discards them or task 11 hands them off, so the one request strip is
    the one place they are offered. An invalid head becomes
    `ExportRecoveryOffered` (**Export previous**, on the strip's retry
    identifier, with no replacement gate because nothing is replaced);
    accepting it yields bytes labelled `ExportSource::RecoveredPredecessor`,
    whose label is §4's text verbatim, and calls neither
    `PromoteRecoveredSlot` nor `SelectContinue`. Ready shows stage 2 as its own
    control, `library.request.handoff` ("Save copy"), disabled with
    `TransferUnavailable`, never on the identifier Export previous used, so a
    repeated activation cannot reach the handoff; its intent is deferred in
    `apply_library_intent` like the transfer strip's. Cancel from any export
    state keeps the cached list, because nothing moved. The Ready bytes are
    `LoadSlot`'s own, already canonical-checked and replayed; they are exposed
    read-only through `ClientRuntime::prepared_slot_export` for task 11.
    `library_client_available` and `LibraryClientUnavailable` are removed,
    as 9b removed `slot_changes_available`.
    **12d landed 2026-09-17 (`2cafa03`): Import archive, as the strip's
    Import galaxy.**
    The file comes from the transfer adapter's `ChooseImport`; the bytes go
    through the same `WorkshopLibraryClient`, generalized over a private
    subject (a loaded slot, or imported bytes) rather than duplicated, so the
    catalog resolution, canonical pack check and bounded replay are §4's own.
    An import has no slot, no observed generation, no predecessor and no
    marker, so it never selects, promotes or stores anything, and it ends in
    `WorkshopSession::from_imported`: paused, slotless, dirty, labelled
    `Imported; not saved`, not a Continue target (§5). Decisions recorded
    here:
    - **§6's gate twice**: at dispatch (nothing is chosen while the resident
      cannot be replaced) and on arrival, where a validated import that finds
      the resident unreplaceable, or the Library off screen, is held as
      `ImportHeld` and offered as **Open** on the Held identifiers and gate.
      **Choose again** after a refused file takes the same gate.
    - **§5's missing pack is `ImportNeedsPack`**, which retains the archive
      (bounded by the archive limit the adapter enforces) and offers
      **Import pack** and Cancel, naming the declared pack by its §7 file
      name. Import pack is a new `ChooseImport` for a content pack; the chosen
      pack is decoded and **hash-checked against the declared hash before
      `PutPack`**, so a different, invalid or noncanonical pack is refused
      without storing anything and the archive keeps waiting. A matching pack
      goes through the retryable catalog-import machine, and the archive
      resumes only once that machine reports `Stored` (§5: durable, not
      in-memory), without a second archive choice.
    - **"Missing" is inferred, not observed.** The memory and native adapters
      answer `GetPack` for an absent hash with `CorruptPack`, the same answer
      a damaged stored pack gives, so both become `ImportNeedsPack`; a stored
      pack that decodes but fails the canonical check does too, with a
      `Catalog` problem. Any other store error (a busy lane) is a retryable
      `Failed { ImportArchive }` that resolves the retained bytes again. The
      canonical-check branch has no runtime test, because the memory store
      validates every pack it accepts.
    - A file that does not validate as an archive is `Failed
      { ChooseArchive, Archive }`: the bytes are released and Choose again is
      a new choice. Cancel from any import state releases the archive;
      cancelling while a matching pack stores leaves that pack stored.
    - **The resident's startup Continue candidate is dropped on install**,
      because the imported Workshop decides Continue once it saves.
    Evidence: `tests/workshop_library_archive_import.rs` (15),
    `import_galaxy_states_offer_their_own_strip_controls`, the crowded-frame
    sweep over every import state, and
    `import_galaxy_runs_from_the_frame_through_a_missing_pack`; eleven
    mutations of these rules were each killed by a named test. The file
    choice is `ScriptedTransfer` throughout; no live picker is exercised.

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
