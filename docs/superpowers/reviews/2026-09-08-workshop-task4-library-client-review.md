# Review — Library prerequisite Task 4 (`df2457c`, `75cf146`, `a1d3c8d`)

**Verdict: APPROVE WITH FINDINGS.**

Status: review record. Reviewer: independent pass over the three-commit range, HEAD
`a1d3c8d` re-read from `git log` at review time (the interleaved docs-only `5881a21`
is excluded, as instructed). No source was modified by this review; four temporary
mutations and one compile probe were applied and reverted, each verified back to a
clean `git diff` before the next step.

All three task obligations are met. The extraction is behaviour-preserving for
startup Continue by construction and by evidence, `abandon` exists on every
implementor, and the `a1d3c8d` guard closes the leak it names rather than narrowing
it. Every mutation claim in the three commit messages that I re-ran reproduced
exactly, including the one that was offered as the weaker instrument. The findings
below are one real coverage regression, two forward-looking API risks that will
become defects at task 12 if nobody writes them down now, and a set of low/nit
observations. None of them is a data-corruption path.

---

## How each claim was established

Three separate registers are used throughout, and the boundary between them is the
point of this document.

- **Read** — established by reading source or spec in the working tree at
  `a1d3c8d`.
- **Ran** — established by executing a command on this machine, exit code taken
  from the command itself (never through a pipe or a trailing `echo`).
- **Inferred** — reasoned from documented platform behaviour or from code shape,
  with no execution behind it.

### Gate ledger (Ran)

| Command | Exit | Result |
|---|---|---|
| `cargo fmt --all --check` | 0 | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 | clean — see the caching note below |
| `cargo test --workspace --all-targets` | 0 | **566 passed, 0 failed, 43 suites** |
| `cargo check --target wasm32-unknown-unknown --lib` | 0 | see probe below |
| `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings` | 0 | genuinely recompiled the lib |

**Host clippy caching, stated precisely.** My first two host clippy runs were
cached greens: both reported `Finished` in under a second with no `Checking nyon`
line. That is valid evidence by cargo's fingerprint for an unchanged tree, but it is
not an independently forced run, and my second attempt to force one failed for a
reason worth recording — the file I had touched, `store/web/wasm.rs`, is
`cfg(target_arch = "wasm32")`-gated and so is not in the host build's dependency
info, so touching it invalidates nothing on the host. The third run, after the
mutation restores below had bumped `library.rs`, `store.rs` and `client_runtime.rs`,
did rebuild (`Checking nyon v0.1.0`) and exited 0. Treat the clippy row as: green,
independently forced on the third run.

The first wasm `check` reported `Finished in 0.35s`, which is exactly the cached
green the task warned about. I appended `compile_error!("reviewer probe: this wasm
gate really compiled")` to `src/workshop/store/web/wasm.rs` and re-ran: **exit 101**,
with the probe message quoted at `wasm.rs:1481`. So the wasm gate does compile that
file. The probe was reverted and `git diff --stat` confirmed the file clean before
the follow-up green.

**Test arithmetic verified (Ran).** `git diff df2457c~1 a1d3c8d -- tests/` adds 11
`#[test]` items and removes none; 555 + 11 = 566, and 42 + 1 = 43 for the single new
file `tests/workshop_library.rs`. The reported baseline and the reported result are
consistent with the diff, and 566/43 is what the suite actually printed here.

**Environment note.** The session's opening snapshot listed dirty
`crates/nyon-workshop-core/src/living/*` files and an untracked review document.
Those were gone by mid-review with `HEAD` unchanged, so another session discarded or
stashed them. I touched none of those paths; every file I edited was restored and
re-verified against `HEAD`.

---

## The four specific questions

### 1. Is the extraction genuinely behaviour-preserving for startup Continue? — Yes.

**Read.** `LibraryOpen::SelectedContinue` sets `expected_generation = None` and
`selects_continue = false` (`src/app/client_runtime/library.rs:178-179`). Those are
the *only* two writes to either field in the whole module
(`library.rs:178,179,187,188`, all inside `begin`), so neither can be set by any
other path. The §4 item-1 check is guarded by `if let Some(observed) =
self.expected_generation` (`library.rs:259-261`) and the `SelectContinue` start is
guarded by `if !self.selects_continue || ... recovered_from_previous`
(`library.rs:542-544`). Both new paths are therefore unreachable from startup
Continue, as claimed.

**Read, arm by arm.** I diffed each removed `ContinueBootstrap` arm against its
`LibraryEvent` disposition in `poll_continue_bootstrap`
(`src/app/client_runtime.rs:888-918`). Every pairing is equivalent:
`Failed(store_failure(e))` → `enter_recovery(Store, e.to_string())`;
`Failed(original_failure)` → `enter_recovery(original_failure.code, .message)`;
`ProtocolFailure(msg)` → `bootstrap_protocol_failure(msg)` (which still clears
`continue_candidate`); `NoCandidate` → `screen = bootstrap_return`; `Ready` →
the identical recovered/not-recovered split. `Phase::Idle` is restored by
`std::mem::replace` at `library.rs:200` exactly as the old `mem::replace` did.

**Ran — independent behavioural evidence.** The extraction is not merely
"compiles + green": `df2457c` landed five pinned protocol-message assertions
*before* the move (`tests/workshop_client.rs`, `continue_bootstrap_protocol_messages_are_the_pinned_arm_identities`
plus the recovered-predecessor offer test), and they pass at HEAD. Independently, I
extracted every string literal ≥15 chars from the removed block and from
`library.rs`: **17 of 18 old literals appear verbatim**, the missing one being
`"The latest save was invalid; a previous valid generation is available"`, which is
in `client_runtime.rs:897-899` — correctly retained as screen policy. At `75cf146`
the set of *new* literals was **exactly three**, all in the `Selecting` phase, which
matches the commit message precisely. (At HEAD there are five, the extra two being
`LibraryBeginError`'s from `a1d3c8d`.) This is a stronger instrument than
`--color-moved`, and I did not bother re-counting the move lines.

### 2. Does `abandon` free the lane in all four adapters plus `RuntimeWorkshopStore`? — Yes, in code; tested in two.

**Read.** Five production implementors exist and all five implement it:
`MemoryWorkshopStore` (`memory.rs:397`), `NativeWorkshopStore` (`native.rs:725`),
`IndexedDbTransactionModel` (`web.rs:76`), `IndexedDbWorkshopStore`
(`web/wasm.rs:229`), `RuntimeWorkshopStore` (`platform/native.rs:72`). Three delegate
directly to the single `JobTable::abandon` (`store.rs:536-549`), which is the only
place the lane is cleared; native does the same after dropping the worker's receiver;
`RuntimeWorkshopStore` forwards to native, and its `Unavailable` arm returns `false`, which is correct because its `start` always errors
so no job can exist. Seven test doubles were updated as well.

**Ran.** Covered by test on memory (`tests/workshop_store.rs:634`) and native
(`:713`), plus the runtime-level inversion (`tests/workshop_client.rs:1126`). The
browser model and the wasm adapter have no `abandon` test — see Finding 8.

### 3. `LibraryOpen::Slot` has no product caller. Adequate? — Deliberate, and adequately covered *as an open*; not as an export.

**Read.** The design's task order defers it explicitly: task 12 is "Enable Open,
Import archive, Set Continue and row Export on the library client"
(`docs/superpowers/specs/2026-09-08-workshop-library-route-design.md:195-196`), and
`library.rs:18-22` says so. **Ran.** Five of the seven tests in
`tests/workshop_library.rs` drive `LibraryOpen::Slot`, covering stale row, race
conflict, success-with-selection, and the `Selecting`-phase begin refusal — that is
better coverage than most shipped paths get. The gap is not coverage but semantics:
see Finding 2.

### 4. Are the four reported mutations real discriminators? — Yes, and the reasoning in question holds only as a **pair**.

Every mutation below was applied by me, run scoped, and reverted (Ran).

| # | Mutation | Result | Matches report |
|---|---|---|---|
| M1 | Delete the §4 item-1 observed-generation check (`library.rs:259-267`) | Only `an_open_of_a_superseded_row_...` fails, panicking with `Ready(... generation: SaveGeneration(2) ...)`; race test **passes**; `workshop_client` 23/23 pass | exactly |
| M2 | `Selecting`'s `StaleGeneration` arm returns `Ready` instead of `ContinueConflict` (`library.rs:410-412`) | Only `a_commit_racing_a_validated_open_...` fails; `workshop_client` 23/23 pass | exactly |
| M3 | Remove the lane clear from `JobTable::abandon` (`store.rs:539-549`) | `abandoning_an_in_flight_job_...` **and** `abandoning_a_native_job_...` both fail with `the Commit lane is free again: Busy { class: Commit }`; the runtime-level `cancelling_an_in_flight_catalog_import_...` fails too | exactly, plus a third dependent test the message did not claim |
| M4 | Remove the `is_active` guard from `begin` (`library.rs:172-174`) | Both new begin tests fail; from `Listing` the failure is `Err(Store(Busy { class: LoadOrImport }))` vs expected `Err(Active)` | exactly, including the nuance that only the `Selecting` test isolates the guard |

**Judgement on the offered reasoning.** The implementer wrote that M1 leaving the
race test green "is how that test proves it exercises `SelectContinue` rather than
the load check". That inference is only half-valid on its own. M1 proves the race
test is **independent of** the item-1 check — necessary, not sufficient; a test that
asserted nothing would also survive M1. What proves **dependence on** the `Selecting`
arm is M2, where the race test is the single failure. The pair is a sound
discriminator; M1 alone is not, and the commit message attributes the proof to the
weaker half. No correction to the code is needed — only to the argument.

---

## Judgement on the three claims flagged for hardest scrutiny

### `abandon` abandons the outcome, not the work — the safety argument is **sound**.

The claim reduces to: no mutation that depends on the head generation may proceed
without comparing it. I enumerated the request vocabulary
(`store.rs:305-400`) and split it:

- **Head-dependent, all three adapters CAS (Read).** `CommitSlot`,
  `PromoteRecoveredSlot`, `SelectContinue` — memory at `memory.rs:145-152`,
  `:205-211`, `:277-283`; native at `native.rs:250`, `:308`, `:409`, each inside
  `NativeStoreWorker::execute` which holds `store-v1.lock` from `native.rs:190`
  across the whole body; wasm at `web/wasm.rs:400` (commit), `:488` (promote),
  `:690` (select), each inside a single `Readwrite` transaction opened by
  `mutate_references` (`wasm.rs` transaction scope `[SLOTS, ARCHIVES]`). The read-side
  `LoadPreviousGeneration` CASes too (`memory.rs:112`, `native.rs:540`, `wasm.rs:582`).
- **Not head-dependent, so nothing to lose (Read).** `CreateSlot`, `RenameSlot`,
  `ArchiveSlot`, `UnarchiveSlot`, `PutPack` never read a head to compute a write.
  `ArchiveSlot` clears the Continue marker, but unconditionally on slot identity, not
  on a generation.

**I found no head-dependent mutation that does not compare and swap.** The worst
outcome of an abandoned mutation is therefore a `StaleGeneration` refusal on the
next attempt, exactly as the doc comment at `store.rs:426-450` says. One cost the
doc does *not* name is Finding 5.

### `JobTable::finish` as a no-op cannot mask a real protocol error today.

**Read.** Each adapter finishes at most once per job: memory calls it inline
immediately after `reserve` (`memory.rs:389-392`); native calls it only from `poll`
after `pending.remove` (`native.rs:730-747`) or from the spawn-failure branch; wasm
calls it from exactly one `spawn_local` future (`wasm.rs:218-225`). A finish after a
consuming `poll` is unreachable, because `poll` only returns `Complete` once the
result is set and it removes the record in the same step (`store.rs:558-570`). So the
only reachable missing-record case is abandonment, which is what the no-op is for.
The `expect` did not protect anything that is still reachable. It is, however, an
invariant that used to be enforced and now is not — Finding 4.

### Native's two-Commit-workers-in-one-process claim — **verified in source, sound on every target NYON ships.**

**Read.** `NativeStoreWorker::execute` acquires the lock as its second statement
(`native.rs:190`) — before `read_manifest` at `:191` — and `_store_lock` is a live
binding for the whole match, so the entire read-modify-write is under it.
`acquire_store_lock` (`native.rs:759-770`) opens `store-v1.lock` fresh per worker and
calls `File::lock()`.

**Inferred.** `std::fs::File::lock` is `flock(LOCK_EX)` on macOS and Linux and
`LockFileEx` on Windows; both associate the lock with the *open file description* /
handle, not the process, so two threads that each opened the file independently do
serialize. The exception is the POSIX-`fcntl` fallback std uses on Solaris/illumos,
where locks are per-process and two same-process descriptors would **not** block each
other. NYON's native targets are macOS, Windows and Linux
(`native.rs` `NativeWorkshopPlatform`), so the claim holds — but it rests on a
platform property the doc comment does not name (Finding 6).

**Ran — one empirical data point.** `abandoning_a_native_job_frees_its_lane_while_the_worker_thread_finishes`
(`tests/workshop_store.rs:713`) really is the two-Commit-workers case: it abandons a
live `PutPack` worker and immediately runs a `CommitSlot` to completion on this
machine, and it passes. That is a single scheduling sample, not a proof of absence of
a race, and I am recording it as such.

### The inverted test is a contract change, not a weakened test.

**Read.** Addendum §6 requires that "the resident Workshop's own
recovery-persistence obligation always remains eligible to run", and `ff9b6ca` could
only honour that by refusing the cancel, because `poll` was the sole lane release.
With `abandon` in the vocabulary the refusal is no longer the only way to honour §6,
so inverting the assertion is the spec-conforming move. Note the honest nuance: §6
never says "cancel while storing is offered"; the justification is the eligibility
sentence, not an explicit clause.

The replacement test is strictly stronger than the one it replaced — it observes the
wedge (`Busy`) rather than assuming it, cancels, proves the resident Workshop then
commits, asserts no diagnostic was pushed, and asserts the stored pack survived
(§6's "never deletes a pack that has already been stored"). **Ran:** M3 shows it
fails when the lane clear is removed, so it is load-bearing. One assertion was lost
in the inversion — Finding 1.

### `a1d3c8d` closes the leak rather than narrowing it.

**Read.** `begin` refuses when active (`library.rs:172-174`), so the phase can never
be overwritten while it holds a job. The remaining question is whether anything else
can strand a phase: `self.library` is written in exactly one place besides `poll`
(`client_runtime.rs:398`, inside the guarded `begin_continue_bootstrap`), and
`poll_continue_bootstrap()` is called **unconditionally** at the top of `update()`
(`client_runtime.rs:414`), not gated on `screen == Loading`. So an active open is
always driven to a terminal event regardless of screen transitions, and every
terminal arm of `WorkshopLibraryClient::poll` has already polled its job before
returning. **Ran:** M4 confirms both new tests depend on the guard, and confirms the
subtle point that only the `Selecting` test isolates it. The leak is closed for the
present caller set. It is not closed for a future caller that wants to *stop* an open
— Finding 3.

---

## Findings

### 1. `Stored` refusal branch of `cancel_catalog_import` lost its only assertion — **Low**

`src/app/client_runtime.rs:586` — `restored @ (CatalogImport::Idle | CatalogImport::Stored { .. })`.

The test that `df2457c` inverted previously asserted both halves of the refusal: it
ended with "A terminal `Stored` has no intent left to cancel either" and an
`assert_eq!(runtime.cancel_catalog_import(), Err(RouteUnavailable))` taken from the
`Stored` state. The replacement (`tests/workshop_client.rs:1126-1210`) asserts the
refusal only from `Idle`. **Ran (M5):** moving `CatalogImport::Stored { .. }` into the
`Ok(())` arm — i.e. making a stored import silently cancellable — passes
`workshop_client` 23/23, `workshop_library` 7/7, `workshop_recovery` 7/7 and
`workshop_session` 14/14. Nothing in the suite pins that branch.

Suggestion: restore the two lines at the end of the inverted test — drive an import
to `Stored` and assert `Err(ClientRuntimeError::RouteUnavailable)` — or add a small
dedicated test. Cheap, and the branch is the one a future "make cancel uniform" edit
would delete first.

### 2. `LibraryOpen::Slot` always selects Continue, but §4 forbids that for row Export — **Medium (forward risk, not a live defect)**

`src/app/client_runtime/library.rs:188` (`self.selects_continue = true;`),
`:542-548` (the `SelectContinue` start), against
`docs/superpowers/specs/2026-09-04-nyon-workshop-library-addendum.md` §4: "Row Export
also carries the observed generation, resolves the exact catalog, and fully replays
before bytes become Ready. **It does not mutate storage, select Continue, or
install/replace a session.**"

The module doc at `library.rs:19-22` claims the client exists because "§4 assigns
Library Open, Use for Continue and row Export to this client". But the enum offers
exactly one non-startup mode, and that mode unconditionally issues a Commit-lane
`SelectContinue`. Wiring row Export to `LibraryOpen::Slot` at task 12 would compile,
pass a naive test, and mutate the Continue marker on an export — which is the exact
failure shape the design document warns about for `RequestLoad`
(`2026-09-08-workshop-library-route-design.md`, "would compile, pass a naive test,
and ship a defect"). Reintroducing that shape *inside the client built to prevent it*
is worth closing now while the API has no callers.

Suggestion: make the intent explicit in the variant rather than implicit in a private
bool — e.g. `LibraryOpen::Slot { slot, expected_generation, intent: OpenIntent::{Open,
Export} }`, with `Export` leaving `selects_continue` false — or, at minimum, a doc
line on `LibraryOpen::Slot` stating that it is the Open/Use-for-Continue mode and that
row Export needs a non-selecting variant.

### 3. The client has no abandon of its own, so §6's Retry/Cancel has no route out — **Low (forward risk)**

`src/app/client_runtime/library.rs:136-196` — `WorkshopLibraryClient` exposes
`begin`, `poll`, `is_active`, and nothing else.

`df2457c` added the store-level escape precisely so a caller that gives up on a job
does not wedge a lane. The client that will own the Library screen cannot use it: a
caller that wants to leave the Library while the client sits in `Selecting` (holding
the Commit lane) has no option but to keep polling to completion, which is the
condition §6 calls out as needing to stay eligible. Today `ClientRuntime` always
polls, so nothing leaks — the gap is entirely prospective, but it is prospective in
the same commit that added the fix for it.

Suggestion: add `WorkshopLibraryClient::abandon(&mut self, store: &mut impl
WorkshopStore) -> bool` that calls `store.abandon(job)` for whichever phase holds one
(and drops the decoder for `Decoding`), returning to `Idle`. It is ~15 lines, it is
testable now with the memory adapter, and it makes the `LibraryBeginError::Active`
refusal a policy rather than the only available answer.

### 4. `JobTable::finish` no longer enforces the invariant it used to — **Low**

`src/workshop/store.rs:529-534`.

Correct today, per the analysis above: no adapter can finish twice, so the only
missing-record case is abandonment. But the change replaced an enforced invariant
with a comment, and the comment's reasoning ("the browser adapter finishes from a
callback that cannot be recalled") is about the *abandoned* case only. If a future
adapter gains a second completion source — an error callback beside a success
callback is the obvious one — the outcome is silently dropped and the job hangs
`Pending` forever with no signal.

Suggestion: keep the no-op but make it narrow — record abandoned IDs in a small
bounded set (or a monotonic "last abandoned" watermark) and `debug_assert!` on a
finish for an ID that was neither reserved nor abandoned. That preserves the browser
behaviour and keeps the protocol violation loud in tests.

### 5. `abandon`'s documented cost is incomplete for `CreateSlot` — **Low**

`src/workshop/store.rs:437-447` documents the cost as "not knowing the slot's head
generation". For `CreateSlot` the abandoned outcome is `SlotCreated { slot,
generation }`, so the caller loses the **identity** of a slot that now exists: a slot
that consumes one of the 16 capacity places (`MAX_WORKSHOP_SLOTS`) and whose id the
caller never learned. A subsequent `ListSlots` recovers it, but the doc's "re-list
before you trust any generation you held" does not say that re-listing is *mandatory*
after abandoning a create. Not reachable today — `cancel_catalog_import`
(`client_runtime.rs:581`) is the only caller and abandons a `PutPack`, which is
content-addressed and idempotent.

Suggestion: one sentence naming `CreateSlot` as the case where the lost outcome is an
identity rather than a generation.

### 6. The native lock argument depends on an unnamed platform property — **Low**

`src/workshop/store/native.rs:715-724`. The comment says an abandoned worker and its
successor "serialize on that lock exactly as two store instances over one root
already do". Two store instances in *different processes* serialize under any locking
flavour; two workers in *one process* serialize only because `File::lock` is
descriptor-scoped on this platform family. That is true for macOS, Windows and Linux
and therefore for every target NYON builds, but the doc reads as if the two cases were
identical, and they are not.

Suggestion: add the reason — "each worker opens the lock file independently, and
`File::lock` is per-open-file-description (`flock`) on the native targets, so
same-process workers block each other as different processes do."

### 7. `abandon` guards lane identity; `poll` does not — **Nit**

`src/workshop/store.rs:539-549` (identity-checked) versus `:563-566` (clears
`active_commit`/`active_load` by class, unconditionally). I traced this and the
asymmetry is currently harmless: for a record to survive while a *different* job owns
its lane, that other job must have been reserved, which requires the lane free, which
requires the first record to have been removed. So the guarded case is unreachable
and the unguarded case is safe. The asymmetry still reads as if one of the two were
wrong.

Suggestion: either apply the same identity check in `poll`, or say in the `abandon`
comment that the guard is defensive against a state the current `reserve`/`poll` pair
cannot produce.

### 8. Browser and wasm `abandon` are untested — **Low (boundary, partly unavoidable)**

`src/workshop/store/web.rs:76-78` and `src/workshop/store/web/wasm.rs:228-231`. Both
are one-line delegations to the `JobTable::abandon` that the memory test covers, so
the risk is small. `IndexedDbTransactionModel` *is* host-testable — it is driven by
`tests/workshop_store_web.rs` and `tests/workshop_recovery.rs:343` — so a
lane-freeing assertion there would cost three lines and would close the browser
model. `IndexedDbWorkshopStore` cannot be tested on the host at all; its `abandon`
has compile evidence only (verified real by the `compile_error!` probe).

Suggestion: add the memory-shaped abandon assertion to the browser model. State the
wasm adapter's abandon as compile-checked-only in the task's verification notes.

### 9. The native abandon test does not assert the work survived — **Nit**

`tests/workshop_store.rs:713-753`. The memory test asserts the abandoned `PutPack`
still stored its pack, which is the "abandons the outcome, not the work" half of the
contract. The native test — the only one where the work is genuinely in flight, and
therefore the only one where "the write still lands" is a non-trivial claim — checks
only the lane. A `ListPacks`/`GetPack` at the end would make the native case carry the
property it is best placed to demonstrate. It would need a bounded poll for the
worker to finish, so it is slightly more than a two-line addition.

### 10. Terminal events leave the open's configuration set — **Nit**

`src/app/client_runtime/library.rs:141-144`. After a `StaleRow` or `Failed`, `phase`
returns to `Idle` but `expected_generation` and `selects_continue` keep the finished
open's values. Safe only because `begin` writes both fields on both arms
(`:178-179`, `:187-188`) — which I verified is the complete set of writes. Clearing
them alongside the phase would make the safety local instead of requiring that
whole-module check.

---

## Explicitly not checked

- **Browser behaviour.** No IndexedDB was exercised. The claim that two overlapping
  readwrite transactions serialize by scope is **Inferred** from the IndexedDB
  specification plus the transaction scopes I read (`mutate_references` takes
  `[SLOTS, ARCHIVES]`, `put_pack` takes `[PACKS]` only — so an abandoned `PutPack`
  and a successor `CommitSlot` do not even contend). Nothing here was executed.
- **Concurrency beyond one sample.** The native two-worker overlap was exercised once
  by its own test on this machine. I did not stress it, did not run under a thread
  sanitizer, and did not attempt to force the abandoned worker to win the lock race.
  A passing test is not evidence that both interleavings were taken.
- **`RuntimeWorkshopStore`.** Read only; it has no test and is only reachable from
  the real native binary, which I did not run.
- **The wasm adapter at runtime.** Compile and clippy evidence only, both proved real
  by the probe.
- **Baseline 555/42.** Not measured directly — I did not check out `df2457c~1` and
  run it. It is corroborated arithmetically from the diff's 11 added tests and one
  added suite, which is consistency, not measurement.
- **`--color-moved` line counts.** Not re-run; the literal-by-literal comparison
  supersedes it.
- **UI, layout, accessibility, and the web build.** Untouched by this diff and not
  exercised.

---

## Summary

The three obligations are discharged. `abandon` is implemented once and delegated
five times, its safety argument survives an independent enumeration of the request
vocabulary across all three adapters, and the native overlap it newly permits is
serialized by a lock I confirmed is taken before the manifest read and held across
the whole read-modify-write. The extraction preserves startup Continue by
construction — the two new conflict paths are gated on fields that
`LibraryOpen::SelectedContinue` sets to their disabled values, and those are the only
writes in the module — and by evidence stronger than the gate: 17 of 18 message
literals verbatim, the 18th correctly retained as screen policy, plus five
protocol-arm assertions landed before the move and still passing. The `a1d3c8d`
guard closes its leak rather than narrowing it, because the runtime polls the client
unconditionally and nothing else writes the phase. All four reported mutations
reproduced exactly; the one questionable claim is an argument about M1, not a defect
in the code.

What to do before task 12: Finding 2 (row Export must not select Continue) and
Finding 3 (the client cannot abandon its own open) are the two that will cost real
debugging if they are discovered from the screen side. Finding 1 is a three-line test
restoration.

---

*Reviewed at `a1d3c8d`. Gates re-run on 2026-09-08 by the reviewer, exit codes taken
from the commands themselves. Mutations M1–M5 applied and reverted; working tree
verified clean against `HEAD` after each.*
