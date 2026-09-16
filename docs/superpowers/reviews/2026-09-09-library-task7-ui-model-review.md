# Review: Library task 7 — the UI model, its intents, and focus order (`fae6c37`)

**Verdict: APPROVE WITH FINDINGS.**

Date: 2026-09-09
Range reviewed: `436b915..fae6c37` (isolated from the docs commit `36c9969` on top).
Files: `src/ui/library.rs` (new, 1049), `tests/workshop_ui_library.rs` (new, 1027,
32 tests), `src/ui.rs` (+1), `src/app.rs` (visibility only).

Authority: `docs/superpowers/specs/2026-09-08-workshop-library-route-design.md`
task 7; contract
`docs/superpowers/specs/2026-09-04-nyon-workshop-library-addendum.md`.

**Why APPROVE and not REQUEST CHANGES:** I found no defect in the shipped
behaviour. Every gating rule, every intent payload, every content state and the
focus order are correct against §2, §3, §4 and §6 as written, and the gate is
green with the claimed arithmetic. Every finding below is either an evidence gap
(a correct behaviour with no witness), one presentation-copy defect in a
reachable state, or a docstring that will mislead task 12. Exactly one (F3)
calls for a change to this file, and it is one conditional in a copy string; the
rest are test and documentation additions.

**Why the findings still matter:** four of fifteen mutations survived the suite,
and one of the four is a named contract this commit's message claims is "pinned
by a test." Task 8 dispatches off this model and task 9 puts a modal in front of
the exact control whose direction is unwitnessed. F1 and F2 should close before
either lands.

---

## What I verified, and how

**By running** (all commands from the repo root, exit codes read out of the log
file, never through a pipe or a trailing `echo`):

- `cargo test --workspace --all-targets` → `EXIT: 0`, **47 `Running` lines**,
  47 `test result:` lines summing **686 passed / 0 failed / 0 ignored**, zero
  `failures:` blocks. Log `/tmp/nyon-task7-gate.log`.
- `cargo fmt --all --check` → `EXIT: 0`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` →
  `EXIT: 0`, zero warning/error lines.
- `tests/workshop_ui_library.rs` alone reports **32 passed**. So 686 − 32 = 654
  and 47 − 1 = 46. **The commit message's `686/47` up from `654/46` is
  arithmetically correct**, and it is the `--all-targets` figure, so it is the
  right comparison basis.
- **15 mutations**, each applied to `src/ui/library.rs`, run as
  `cargo test --test workshop_ui_library`, each confirmed to have recompiled
  (`Compiling nyon` present in every log), each reverted with
  `git restore --staged --worktree` followed by a `git status --porcelain` that
  printed nothing. `workshop_ui_library.rs` is the only suite that exercises
  this model, on two independent grounds: `src/ui/library.rs` is created by this
  commit, so no pre-existing suite can import it; and the only other file
  matching `library::` under `tests/` is `workshop_library.rs:10`, whose import
  is `app::client_runtime::library::{…}` — the runtime submodule, not
  `ui::library`. So the single-suite mutation runs are not hiding kills
  elsewhere.

**By reading**: `src/ui/library.rs` in full; `tests/workshop_ui_library.rs` in
full; `src/app/client_runtime.rs:960–1340` for the three runtime contracts;
`src/workshop/store.rs:238–251` for `SlotSummary`/`SlotList`;
`src/workshop/store/native.rs:195–215` for slot-id minting;
`src/ui/platform.rs:398–430` and `src/ui/platform_projection.rs:40–60` for the
role→ellipsis path; both spec documents in full.

**Could not check**: the SDF batch's actual refusal of an oversized fixed label
(no frame is built here — I verified only that `SemanticRole::Option` selects
`PlatformTextOverflow::SingleLineEllipsis`, which is the mechanism the design
names); any 44×44 or layout property (task 8); real store asynchrony; whether
`build_actions`'s deferred §6 gate is correct once task 12 supplies the client.

**Not verifiable as stated**: the commit message's "12 mutations each killing the
test named for the property." Those twelve are enumerated nowhere in the commit,
the tree or the review directory, so the claim cannot be checked. The fifteen
below are an independent set, chosen from the spec and the code rather than from
that list. This matters because F1 shows one evidence claim in the same message
is unsupported.

---

## The three contracts it was given

### Contract 1 — `None` + `Working { List }` is Loading, not Empty. **Verified, and the decomposition is right and complete.**

Read against the runtime rather than the commit message:

- `finish_slot_request` (`client_runtime.rs:1319`) sets `slot_list = None` and
  immediately dispatches `ListSlots` after every successful mutation → `None` +
  `Working { List }` for one frame. The `ListSlots` success arm returns *early*
  (`:1281`–`:1282`) without clearing, so a cached list survives a refresh.
- `cancel_library_slot_request` (`:1111`, `:1117`) drops the list for mutations
  and leaves the machine `Idle` → `None` + `Idle`.
- An empty store is `Some` with empty `slots`; that is the only representation
  of emptiness the runtime produces.

Four states is therefore correct: `Some(non-empty)`→`Slots`,
`Some(empty)`→`Empty`, `None`+`Working{List}`→`Loading`, `None`+anything
else→`NotListed`. Nothing is collapsed and nothing is missing.

The claimed near miss is real and is caught. **Mutation 1** rewrote
`library_content` (`library.rs:550`) to match `status` first and `slots` second
— the shape that flashes Loading over a good cached list on every refresh. It
failed exactly `a_cached_list_survives_a_refresh_without_flashing_loading`, no
other test. The four contract-1 tests assert the positive variant
(`assert_eq!(content, Loading)`), not a negation, so they discriminate against a
three-state implementation.

One defect *inside* this otherwise-correct partition: see **F3**.

### Contract 2 — `open_library` auto-lists only from `Idle`. **Verified, and the test does not pass for the wrong reason.**

`client_runtime.rs:983` guards the auto-list with
`if matches!(self.slot_requests, LibrarySlots::Idle)`, so a `Failed` from a
previous visit survives re-entry. `build_request` (`library.rs:611`) produces
Retry and Cancel from the `Failed` arm unconditionally — it never consults
`slots` — so the decision controls cannot be gated on there being nothing to
show.

`a_preserved_failure_renders_retry_and_cancel_over_cached_rows`
(`tests:161`) does supply `Some(rows)` alongside the failure and asserts
`content == Slots` *and* both controls present and enabled. **The claim that it
cannot pass for the wrong reason is correct**: an implementation that gated the
controls on an empty list would fail it on the `expect("retry control")` line.

### Contract 3 — a store failure never yields `RecoverableError`. **Behaviour verified. The test does not witness it.** See **F1**.

The behaviour is right, and for the reason claimed: `fail_slot_request`
(`client_runtime.rs:1249`) routes every failing arm to
`retain_failed_slot_request` and never calls `enter_recovery`, and
`LibraryUiIntent` (`library.rs:68`) contains no leaving variant but `Close`. The
structural argument — "there is no leaving intent to emit" — holds.

But the test named for it does not check it. See F1.

---

## Findings

**Status update, 2026-09-16.** This file carried twelve findings marked open for a week after `0dc00ae` had closed nine of them, because only `2773bd5` ever touched it. Each status below was re-verified against source and tests rather than copied from that commit's message.


### F1 — P2 — Contract 3 has no discriminating witness, and the commit message says it does

`tests/workshop_ui_library.rs:227–247`, assertion at `:246`.

`a_store_failure_offers_no_intent_that_leaves_the_screen` counts controls whose
intent matches `LibraryUiIntent::Close` and asserts the count is `1`. That is
not the property in its name. It proves *Close exists exactly once*; it says
nothing about whether some *other* control leaves the screen.

**Mutation 10b** added a `LibraryUiIntent::EnterRecovery` variant and pointed
`library.transfer.import-pack` at it, leaving the action id, the label and the
enablement untouched. **All 32 tests passed.** A control that silently routes
the user off the Library into the global recovery screen — the precise thing §6
forbids — is invisible to this suite.

I also ran the coarser form first. **Mutation 10** added a *new* `library.recover`
control emitting the same variant. That one died — but by
`focus_order_is_deterministic_and_reads_top_to_bottom`, whose hard-coded id list
noticed the extra entry. The contract-3 test passed in that run too. So the
accidental defence is an unrelated test's id list, which by construction cannot
see an intent swapped onto an existing control.

The commit message states this contract is "pinned by a test that `Close` is the
only one across a shape with every capability enabled." **That claim is not
supported.** The guard is the enum's shape, which is real but is not what the
test checks.

*Suggestion:* replace the count with an exhaustive `match` over every
`control.intent()` in the model, listing each non-leaving variant explicitly, so
that adding a variant fails to compile rather than passing silently. Correct the
commit-message claim in the task-7 ledger entry rather than leaving it standing.

*Status: closed by `0dc00ae`.* `leaves_screen` is an exhaustive match with no wildcard arm, and `a_store_failure_offers_no_intent_that_leaves_the_screen` now asserts leaving-iff-Close per control.

### F2 — P2 — The Archive direction is unwitnessed, which is the one thing the shared identifier depends on

`src/ui/library.rs:802`; test `tests/workshop_ui_library.rs:498`.

The shared-identifier decision is correct and I verified the fix that established
it. **Mutation 4** changed the Unarchive branch's id to
`"library.action.unarchive"`; it failed both
`an_archived_row_offers_unarchive_and_refuses_open_and_continue` and
`focus_order_is_stable_across_a_selection_change_including_archived_rows`. I also
reproduced the author's own near miss: with mutation 4 applied *and* the focus
test's fixture reduced to all-active rows (the pre-amendment shape), **all 32
tests passed**. The archived row in the fixture is what makes that test
discriminate, exactly as the amendment note claims.

The residual gap is the other half of the argument. The commit reasons that one
id is safe "because the two intents are already distinct variants, so nothing is
ambiguous about which direction was submitted." The suite pins `UnarchiveSlot`
(`tests:498`) but never names `ArchiveSlot` at all.

**Mutation 11** made the Archive branch emit `LibraryUiIntent::UnarchiveSlot`
while keeping the label `"Archive"`, the id `library.action.archive` and every
gate. **All 32 tests passed.** A model whose Archive button submits Unarchive is
green.

This is the single most consequential survivor: task 9 puts a confirmation modal
in front of this exact control, and the confirmation text ("including whether
Archive will clear Continue", §3) will be chosen from the label while the store
receives the intent.

*Suggestion:* assert `built.actions.archive.intent() ==
Some(&LibraryUiIntent::ArchiveSlot { slot })` in the active-row case, beside the
existing archived-row assertion. Ideally in the same test, so the two directions
are read together.

*Status: closed by `0dc00ae`.* `ArchiveSlot` is now named by the suite, and both Archive directions are read together in one test.

### F3 — P2 — `NotListed` under `Failed { List }` tells the user to press a disabled control

`src/ui/library.rs:550` (`library_content`), `:900` (`content_message`), `:596`
(`lane_reason`, reached from `build_refresh`).

The four-way partition is complete, but `NotListed` collapses two causes with
different remedies, and its copy is written for only one of them.

Measured directly (throwaway probe test, since reverted), with `slots: None` and
`status: Failed { kind: List, code: Store }`:

```
content = NotListed
refresh.enabled = false  reason = Some(DecisionPending)
content node = "Saved galaxies have not been listed. Refresh to list them."
announcement Status library-content       = Saved galaxies have not been listed. Refresh to list them.
announcement Error  library-request-failed = Listing saved galaxies failed. Retry or cancel.
```

The screen's status line and its `Status` announcement both direct the user to
Refresh, which is disabled with `DecisionPending`; the live route is Retry. Two
announcements are emitted, one of them pointing at a control that refuses
activation.

**This state is reachable on the first visit.** `open_library`
(`client_runtime.rs:983`) dispatches `ListSlots` from `Idle`; if the store
refuses, `fail_slot_request` leaves `Failed { List }` with `slot_list` still
`None`. It is also reached whenever a mutation's automatic follow-up re-list
fails. No test covers it —
`a_dropped_list_with_a_mutation_in_flight_is_not_listed` (`tests:137`) covers
the other, and rarer, `NotListed` cause.

*Suggestion:* either make the message conditional on `request.decision_pending()`
("Listing failed. Retry or cancel."), or split the state. Add the
`None`+`Failed{List}` case to the content suite either way; it is the first-run
failure path.

*Status: closed by `0dc00ae`, and the defect was wider than reported.* `content_message` takes the lane reason, and all three `NotListed` arms (resting, in flight, failed) are pinned.

### F4 — P3 — The §6 docstring has one omission, one misattribution, and one thing this model does not represent at all

`src/ui/library.rs:738–743` (`build_actions` docstring); `:841`
(`build_transfer`, which carries no forward note).

Read against §6 verbatim: *"Library Open, Workshop Archive import, Use for
Continue, and recovery of another slot remain disabled until replacement and
persistence invariants are safe."*

The docstring names Open, Use for Continue and **row Export**. So:

1. **Omission — `ImportArchive`.** §6's "Workshop Archive import" is
   `library.transfer.import-archive` (`:847`), gated today only on
   `TransferUnavailable`. The author flagged this himself. Worth adding: the
   omission is *doubly* invisible, because `build_transfer` has no docstring at
   all, so there is no forward note anywhere near the control that needs the
   gate. `ImportPack` correctly needs none — §6 explicitly permits content-pack
   storage that does not request session replacement.
2. **Misattribution — row Export.** §6 does not name it, and §4 says row Export
   "does not mutate storage, select Continue, or install/replace a session," so
   there is no replacement-and-persistence gate to add to it. It does need the
   exact-catalog client, which is why `LibraryClientUnavailable` is the right
   reason for it today — but the docstring tells task 12 to add a gate that does
   not apply.
3. **Not modelled — "recovery of another slot."** There is no recovery or repair
   control in this model at all, and none of §4's recovery offers (the same-slot
   promotion offer, the export-recovery choice with its
   `Recovered predecessor; …` label) appear. That is presumably tasks 9 and 12,
   but nothing in this file records it, so the §6 list reads as complete when a
   quarter of it is out of scope.

So the author's own answer — "it may be missing more" — is correct in two further
directions, one of them an over-inclusion rather than an omission.

*Suggestion:* restate the docstring as §6's own four items with a per-item
disposition (gate here / gate in `build_transfer` / not applicable, §4 / not
modelled yet, task 9), and put a matching two-line note on `build_transfer`.

*Status: closed by `0dc00ae`.* The §6 docstring is restated as §6's own four items with a disposition each.

### F5 — P3 — Permitting Export of an archived row is an unpinned decision

`src/ui/library.rs:835` (`&[no_selection, client]`).

The behaviour is correct. §3 and §10 forbid *opening* and *selecting for
Continue* an archived slot; neither forbids exporting one, and §4 makes row
Export a non-mutating read. Allowing it is the right reading.

But nothing defends it. **Mutation 9** added `archived` to Export's gate list —
which would silently remove the only way to get bytes out of an archived save.
**All 32 tests passed.** `an_archived_row_offers_unarchive_and_refuses_open_and_continue`
(`tests:498`) asserts Open and Use for Continue are refused and stops there.

*Suggestion:* one line in that test —
`assert!(built.actions.export.enabled)` — turns a silent reading of the spec into
a pinned one.

*Status: closed by `0dc00ae`.* Archived Export stays enabled and is asserted: §3 forbids opening an archived save, not reading it.

### F6 — P3 — Six of fifteen intent variants are never named by the suite

Counted across `tests/workshop_ui_library.rs`: `RefreshSlots`, `ArchiveSlot`,
`ImportArchive`, `ImportPack`, `ExportActiveArchive` and `ExportActivePack`
appear zero times. The other nine appear at least once.

This is the shared root of F1, F2 and F5: for those six controls the suite
asserts identity, enablement and disabled reason, but never the payload the
runtime will act on. Mutations 10b and 11 are two instances of the class; the
transfer strip's three remaining intents are three more untested payloads.

*Suggestion:* one table-driven test asserting `(action_id, intent)` for every
control in a fully-enabled shape. It costs about fifteen lines and closes F1,
F2, F5 and this finding at once.

*Status: closed by `0dc00ae`.* `every_control_submits_its_own_intent_in_a_fully_enabled_shape` pins every control's payload.

### F7 — P3 — Transfer-strip reason precedence is unpinned

`src/ui/library.rs:868`, `:876` (`&[inactive, transfer]`).

The documented precedence rule (`:249–252`: row facts before capability
deferrals) is genuinely pinned for the actions panel — **mutation 14** swapped
Open's `archived` and `client` and died on
`row_state_outranks_capability_in_the_reported_reason`. It is not pinned for the
transfer strip. **Mutation 15** swapped both `&[inactive, transfer]` pairs to
`&[transfer, inactive]`; **all 32 tests passed**, because
`exporting_the_active_workshop_requires_an_active_workshop` (`tests:647`) sets
`transfer_available: true`, and
`client_and_transfer_controls_are_present_and_disabled_with_a_reason`
(`tests:419`) sets `workshop_active: true`. No shape has both false.

Low impact — it selects which sentence a disabled control shows — but it is the
same class as F5, and it is the case a user with no adapter and no session
actually sees.

*Suggestion:* add `(workshop_active: false, transfer_available: false)` to the
existing loop and assert `WorkshopInactive` wins.

*Status: closed by `0dc00ae`.* Precedence is observable in the no-session, no-adapter shape and documented on the test.

### F8 — P4 — `Debug` on `LibraryControl` prints the private `intent`

`src/ui/library.rs:178–193`.

The structural guard is real and I confirmed it two ways: the field is private to
`crate::ui::library`, both constructors are private, so `src/ui/platform/library.rs`
(task 8, not a descendant module) cannot reach it; and **mutation 6** — making
`intent()` return `Some` unconditionally — died on
`a_disabled_control_hands_out_no_intent_because_its_subject_is_a_real_slot_id`,
while **mutation 7** — dropping `activate`'s `.filter(|c| c.enabled)` — died on
`activation_yields_the_intent_and_a_disabled_control_yields_nothing`. Both read
paths are covered.

The `SlotId(0)` hazard itself is confirmed: `store/native.rs:205` mints from
`(0..slot_limit)`, so slot 0 is allocated first, and
`a_disabled_control_hands_out_no_intent_…` (`tests:996`) builds a real slot 0
rather than asserting the sentinel is safe.

The residual: the derived `Debug` at `:178` prints every field including
`intent`, so `{:?}` on a disabled Open emits `OpenSlot { slot: SlotId(0), … }`.
It defeats nothing on the typed path, but it puts a placeholder that names a real
save into any log or panic message.

*Suggestion:* hand-write `Debug` to render `intent` as `<disabled>` when
`!enabled`, or omit the field.

*Status: closed by `0dc00ae`.* A hand-written `Debug` renders a disabled control's intent as `<disabled>`, asserted by the suite.

### F9 — P4 — `controls()` iterates lexicographically, not in focus order, and says so nowhere

`src/ui/library.rs:517`.

`controls()` returns `BTreeMap::values()`, so `library.slot.10` precedes
`library.slot.2` and every action precedes every row. `focus_order()` (`:527`)
is documented as the traversal order; `controls()` has no docstring at all. Task
8 iterating `controls()` to lay out a panel would get a wrong order that the
existing tests cannot see, because both current consumers look controls up by id.

*Suggestion:* one line on `controls()`: lookup set, unspecified order, use
`focus_order()` for presentation.

*Status: closed by `0dc00ae`.* `controls()` now documents lexicographic order and points at `focus_order` for traversal.

### F10 — P4 — `build` silently deduplicates colliding action ids

`src/ui/library.rs:484–491`.

On a duplicate id the `insert` returns `Some`, the later control overwrites the
earlier in the map, and no focus slot is pushed — so one control silently loses
its focus slot while `activate` resolves to the *other* one.
`focus_order_covers_every_control_exactly_once` (`tests:763`) cannot see it:
both `order.len()` and `controls().len()` shrink together. Today the only
defence is the hard-coded id list in
`focus_order_is_deterministic_and_reads_top_to_bottom`, which happens to
enumerate all fifteen ids and so catches any collision *in the current shape*.
Task 8 adding a control is exactly when that stops being reliable.

*Suggestion:* `debug_assert!(controls.insert(...).is_none(), "duplicate Library
action id: {}", control.action_id)`.

*Status: closed on 2026-09-16 by the commit that adds `a_colliding_action_id_is_loud_rather_than_silently_dropped`.* A `debug_assert!` in `build`, witnessed by a `#[should_panic]` test with two rows sharing a `SlotId`; deleting the assertion reports "did not panic". **Deliberately not the one-liner suggested above:** `debug_assert!(controls.insert(..).is_none())` consumes the return value, so the push would have to become unconditional, and a release-build collision would then leave `focus_order` longer than `controls()`, which is worse than the silent drop. The insert result is bound first, asserted, then used, so release behaviour is unchanged.

### F11 — P4 — Every control is cloned into the lookup map

`src/ui/library.rs:486`.

`build` inserts `control.clone()` into `controls` while the same control is also
owned by `close_control`, `refresh_control`, `request`, `rows`, `actions` or
`transfer`. Each control's three `String`s (`label`, `description`, plus the
`SemanticActionId`) are therefore held twice per model, and the model is rebuilt
per frame. Correct, and small at fifteen controls plus sixteen rows, but the map
could hold indices or `&`-free `SemanticActionId` keys pointing at the owned
controls instead. Noting it because the model is on the per-frame path.

*Status: open, non-blocking.* Unchanged on 2026-09-16.

### F12 — P4 — `AlreadyContinue` inherits a store invariant that nothing here records, and `SlotList::selected_continue` is never read

`src/ui/library.rs:755`.

Judged against the design: **`AlreadyContinue` is faithful.** §4 requires every
`CommitSlot` and `PromoteRecoveredSlot` to clear `selected_continue` atomically
when it names the mutated slot, so a set marker always names the current head,
and offering "Use for Continue" on the row that already *is* Continue would
submit a no-op compare-and-swap. Disabling it is the right call, and the design
does not decide it, so pinning it here is correct behaviour for a UI model.

The test does discriminate. **Mutation 5a** (`row.map(…)`, marker ignored, always
disabled) failed four tests including
`the_row_already_selected_for_continue_cannot_be_selected_again`; **mutation 5b**
(`.filter(|_| false)`, never disabled) failed that same test alone. The
neighbouring row at `tests:889` is what isolates the second edge — the claim is
correct and both edges are witnessed by that one test.

Two residuals worth a note rather than a change:

- The disable's correctness is entirely inherited from a store invariant
  enforced in `CommitSlot`/`PromoteRecoveredSlot`. Neither the variant's
  docstring (`:120`–`:121`) nor the test records that dependency. If §4's atomic clear were ever relaxed, this becomes
  a stale marker the user cannot re-select past, with no failing test.
- `SlotSummary::selected_for_continue` and `SlotList::selected_continue` are two
  representations of one fact, and the model reads only the former. The test
  helper `list()` (`tests:40`) derives the latter from the former, so no fixture
  can make them disagree. Not a defect; worth knowing before task 8 reads
  `selected_continue`.

*Status: closed on 2026-09-16, documentation only, in the same commit as F10.* Both residuals are now recorded on `LibraryDisabledReason::AlreadyContinue` itself: the inherited §4 atomic-clear invariant, and that the model reads `selected_for_continue` rather than `selected_continue`.

---

## Claims I checked and found correct, with nothing to change

- **`SlotId(0)` is really allocatable.** `store/native.rs:205` mints
  `(0..slot_limit).map(SlotId).find(|id| !occupied.contains(id))`. The
  placeholder does name somebody's first save, and the guard against reading it
  is structural (private field, private constructors, accessor returning `None`),
  not conventional. Both read paths are witnessed (mutations 6 and 7). Only the
  `Debug` leak in F8 is left.
- **Archive/Unarchive share one action id, and the fix works.** Mutation 4 died
  on two tests; the pre-amendment all-active fixture lets it survive, exactly as
  the amendment note claims. The direction half is F2.
- **Row identity derives from `SlotId`.** `row_action_id` (`:702`) formats
  `slot.0`. **Mutation 12** rewrote both row builders to use the enumeration
  index; it failed four tests, including
  `row_identity_survives_reordering_because_it_derives_from_the_slot_id`.
  **Mutation 8** (drop sectioning, sort by id) failed that test and
  `archived_rows_are_a_distinct_section_after_the_active_ones`, which asserts
  both the section vector *and* `[2, 4, 1, 3]` — it discriminates on store order
  within a section, not just on grouping.
- **Slot names take `SemanticRole::Option`, and that role is the ellipsis path in
  this tree.** `platform.rs:399–409` computes `variable_label` from
  `TreeItem | TextInput | Option` and `:416` maps it to
  `PlatformTextOverflow::SingleLineEllipsis`; everything else gets `Wrap`.
  **Mutation 13** (rows as `Button`) died on
  `every_row_node_takes_the_option_role_for_the_ellipsis_path`. The onward claim
  that the SDF batch *rejects* an oversized fixed label is the design's, not
  verified by this tree — the fixture does carry a real 64-byte name
  (`tests:328`, `:936`) and the semantic tree validates over it.
- **Client-dependent controls render disabled with a visible reason, never
  omitted.** `LibraryControl::enabled`/`disabled` (`:209`, `:227`) set `enabled`
  and `disabled_reason` together, and `a_disabled_control_always_carries_exactly_one_reason`
  (`tests:385`) walks every control across three shapes.
  `every_disabled_node_states_its_reason` (`tests:972`) additionally asserts the
  semantic node carries the text, and asserts `disabled > 0` so it cannot pass
  vacuously. `client_and_transfer_controls_are_present_and_disabled_with_a_reason`
  looks controls up by id through a helper that panics on absence, so omission
  fails rather than passes.
- **The resident slot cannot be archived, delegating to the runtime's own
  refusal.** `LibraryDisabledReason::message` (`:159`) calls
  `crate::app::safe_client_diagnostic(…)` at `:159`
  rather than restating, which is why `src/app.rs` needed `pub(crate)` — a
  minimal and justified visibility widening. **Both edges of the residency
  predicate are isolated by the one test**, as claimed: **mutation 2**
  (`.filter(|_| false)`) and **mutation 3**
  (`.filter(|_| context.resident_slot.is_some())`) each failed
  `the_resident_slot_cannot_be_archived_and_says_why_in_the_runtimes_words` and
  nothing else. The neighbouring non-resident row at `tests:487`–`:493` is what kills
  mutation 3.
- **It is a selection model, not per-row buttons, and that matches the design.**
  The Layout section puts rows in the canvas and actions in `right_panel` /
  `drawer_sheet`, and says "Row selection in the canvas opens the sheet." Five
  panel controls aimed by selection is the faithful reading; sixteen rows × five
  controls would be eighty focus slots against a design whose transfer-strip
  paragraph is already counting labels against the 320 floor.
- **Focus order.** Header → decision → rows → actions → transfer, asserted as an
  exact fifteen-id list (`tests:795`), covered exactly once (`tests:763`), and
  stable across a selection change including an archived row (`tests:848`).
  Row 16 of 16 is reachable by id and activates (`tests:915`), which is §2's
  virtualization obligation at the model layer.
- **`activate` ignoring `InputModality`** is correct: §2 requires pointer and
  keyboard activation to submit the same typed intent, and `tests:718` asserts
  both modalities return the same intent.

## Mutation log

| # | Mutation | Outcome | Killed by |
|---|---|---|---|
| 1 | `library_content` matches `status` before `slots` | killed | `a_cached_list_survives_a_refresh_without_flashing_loading` |
| 2 | `resident` predicate always false | killed | `the_resident_slot_cannot_be_archived_…` |
| 3 | `resident` predicate = `resident_slot.is_some()` | killed | `the_resident_slot_cannot_be_archived_…` |
| 4 | Unarchive gets its own action id | killed | `an_archived_row_offers_unarchive_…`, `focus_order_is_stable_…` |
| 4b | mutation 4 + all-active focus fixture | **survived** | — (reproduces the author's near miss) |
| 5a | `already_continue` ignores the marker (always) | killed | 4 tests incl. `the_row_already_selected_for_continue_…` |
| 5b | `already_continue` never applies | killed | `the_row_already_selected_for_continue_…` |
| 6 | `intent()` returns `Some` unconditionally | killed | `a_disabled_control_hands_out_no_intent_…` |
| 7 | `activate` drops the `enabled` filter | killed | `activation_yields_the_intent_…` |
| 8 | rows unsectioned, sorted by slot id | killed | `archived_rows_are_a_distinct_section_…`, `row_identity_survives_reordering_…` |
| 9 | row Export gated on `archived` | **survived** | — → F5 |
| 10 | new control emitting a leaving intent | killed | `focus_order_is_deterministic_…` (not the contract-3 test) |
| 10b | leaving intent on an existing control id | **survived** | — → F1 |
| 11 | Archive control emits `UnarchiveSlot` | **survived** | — → F2 |
| 12 | row action id from the list index | killed | 4 tests incl. `row_identity_survives_reordering_…` |
| 13 | row role `Button` instead of `Option` | killed | `every_row_node_takes_the_option_role_…` |
| 14 | Open precedence: capability before row state | killed | `row_state_outranks_capability_…` |
| 15 | transfer precedence: `[transfer, inactive]` | **survived** | — → F7 |

Eighteen rows, fifteen mutations: 5a/5b and 10/10b are two edges of one mutation
each, and 4b is a fixture experiment on the test rather than a mutation of the
code under review. **Eleven killed**, each by the test named for the property;
**four survived**, all in the intent-payload / reason-selection layer, all
addressed above.

## Before task 8

F1 and F2 should close first: task 8 dispatches these intents and task 9 puts a
modal in front of the one control whose direction has no witness. F6's single
table-driven `(action_id, intent)` test closes F1, F2 and F5 together. F3 is a
user-visible defect in a first-run failure path and is a one-line conditional.
F4 should close before task 12 reads that docstring as its work list.
