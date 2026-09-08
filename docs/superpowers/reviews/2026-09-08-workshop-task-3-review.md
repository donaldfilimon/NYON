# Library prerequisite Task 3 review — `8af2eae`

Status: **APPROVE WITH FINDINGS**. Reviewed 2026-09-08 18:5x against
`8af2eae` ("generation-check SelectContinue and clear it on every new head"),
read from `git log` at review time rather than from a SHA reported earlier.

Scope: `SelectContinue` becomes a compare-and-swap carrying
`expected_generation`; `ContinueSelected` reports the `generation` it selected
against; `CommitSlot` and `PromoteRecoveredSlot` clear any marker naming the
slot inside their own mutation. Nine files, 640 insertions.

## Confirmed correct

The three adapters implement one rule, and the rule is the documented one.

- **Error precedence matches `CommitSlot` in all three**: `UnknownSlot`, then
  `ArchivedSlot`, then `StaleGeneration`. The ordering rationale is recorded on
  the request variant — an archived slot is ineligible until explicitly
  unarchived, which no refresh cures, so that eligibility failure outranks a
  freshness failure whose remedy is to re-list and retry.
- **The comparison and the write are one serialized mutation in each adapter,
  not a read followed by a write.** Memory: `execute` is the serialized
  mutation. Native: `execute` holds the store lock and re-reads the manifest
  from disk, so the check sees another instance's commit — and
  `native_continue_selection_rejects_a_generation_another_instance_superseded`
  pins exactly the race a cached-manifest CAS would pass wrongly. Browser: the
  head read and the metadata write share one `mutate_references` read-write
  transaction.
- **The shared contract body is the right structure.**
  `continue_selection_is_generation_checked` is written once in
  `tests/workshop_store.rs` and run against memory, native, and the IndexedDB
  transaction model, so three adapters are held to one description rather than
  three drifting copies. It covers refusal-writes-nothing, selection at the
  head, the commit-clears-then-refuses race, restore after commit, promotion,
  and the full precedence ladder including archive and unarchive.

## F1 (Important) — the browser half of clear-on-new-head is unenforced against single-site removal

`wasm.rs` calls `clear_continue_for_sync` at two sites, `commit_slot` (:434)
and `promote_recovered_slot` (:515), mirroring memory (:183, :239) and native
(:282, :343). Memory and native are covered behaviorally. The browser arm is
not, and the gap is structural rather than an oversight in one test:

1. **`store/web/wasm.rs` executes nowhere in this repository.** There is no
   `wasm-bindgen-test` harness; `[dev-dependencies]` is `pollster` alone. It is
   compile-checked only.
2. **The shared contract's browser arm runs against
   `IndexedDbTransactionModel`, a separate host-side implementation.** A model
   pass is evidence about the model, not about the adapter. Two
   implementations, one exercised.
3. **`wasm_source_uses_atomic_transactions_and_bounded_diagnostics` exists
   precisely to bridge that gap, and this task did not extend it.** Its
   required-substring list gained no `clear_continue_for_sync(` entry.

The protection that does exist is accidental and partial. The helper is
private, so deleting **both** call sites makes it dead code and trips
`dead_code` under the gate's
`cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`.

What nothing catches:

- **Deleting one call site.** Drop the `promote_recovered_slot` call at :515
  and the helper is still used, no lint fires, the workspace suite is
  unaffected, and the browser would silently keep a marker naming a generation
  promotion had already superseded.
- **Hoisting a call outside the `mutate_references` closure.** That is the
  atomicity property the doc comments actually claim — "the archive bytes, the
  new slot reference and the cleared marker are staged through one read-write
  transaction" — and no test, lint, or source contract constrains where the
  call sits.

Suggested fix, cheap and in the idiom already used: add
`"clear_continue_for_sync("` to the required-substring list, and prefer an
occurrence-count assertion of 2 over a `contains` check, since `contains`
proves one site while the invariant needs both.

This is the same shape as F1 of the `50c50f9` review — a rule whose only pin
was too weak to see the mutation that breaks it — and it is worth closing the
same way, by mutation-verifying that the new pin fails when a call site is
removed.

## F2 (Minor) — `8af2eae`'s formatting is partly not its implementer's

At 18:28, before this commit existed, a third session ran `cargo fmt --all`
across the in-flight working tree, including all five source files and four
test files this commit touches. The formatting in `8af2eae` is therefore in
part that session's, not the implementer's. Nothing was lost — the
implementer's subsequent content landed on top and the tree kept growing — and
`cargo fmt --all --check` is gate step one, so the result is what the gate
would demand anyway. Recorded because commit authorship implies the whole diff
is one author's work, and here it is not.

## Fixture footprint, recorded for the next task

The diff as first written omitted the re-select in the `commit` helper of
`tests/workshop_recovery.rs` and failed **7 tests** in that suite —
`ContinueUnavailable`, and sessions landing on `MainMenu` where the test
expected `RecoverableError`. The implementer fixed it in-flight before
committing. The lesson generalises past this commit: because a commit now
clears the marker, **every fixture that commits and then expects a Continue
candidate must re-select**, and the failure surfaces as a session-state
mismatch several layers above the store rather than as a store error. Task 4
should expect that footprint rather than rediscover it.

## Boundary, stated rather than implied

The browser guarantees in this commit rest on reading the source and on the
compiler. Proven: it compiles for the wasm target, the match arms are
exhaustive, clippy is clean, and the transaction model implements the same
contract. Not proven: that the IndexedDB transaction commits, that
`write_metadata_sync` round-trips a record the browser accepts, or that the
clear and the head advance are genuinely atomic against a real abort.
