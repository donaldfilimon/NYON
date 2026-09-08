# Review: `8af2eae` — generation-checked `SelectContinue`, atomic Continue-clear

**Verdict: APPROVE WITH FINDINGS.**

**Status:** Review recorded. Findings 1–6 are follow-on obligations; none blocks the commit.

**Reviewed:** `8af2eae94926f285567cc19c9b5244f81d9213f4`, range `8af2eae^..8af2eae`, 9 files, +640/−27.
**Review package:** `/Users/donaldfilimon/.claude/jobs/3457fc58/tmp/review-task3-8af2eae.diff`
**Authority:** `docs/superpowers/specs/2026-09-04-nyon-workshop-library-addendum.md` §3, §4, §10, §12; `docs/superpowers/specs/2026-09-08-workshop-library-route-design.md`.
**Date:** 2026-09-08

All three obligations are met, in all three adapters, with the comparison genuinely inside the
write's serialized mutation in each. The fixture change in `tests/workshop_recovery.rs` — the part
of the diff most likely to hide a weakened guard — is **state-preserving**: the store state those
seven tests observe is byte-identical before and after, so no assertion lost discriminating power,
and all seven gained some. The findings below are about coverage that the commit did not add and
about two doc statements that overclaim.

---

## Line-number convention

`src/` citations are line numbers **at `8af2eae`** and are unchanged in the working tree.
`tests/` citations are line numbers at `8af2eae` unless marked otherwise. `main` has moved **six**
commits past the reviewed one: `2b7bfbc`, `b6f33ab`, `06873f6`, and — arriving during this review —
`1ae4e3a`, `a0b1371`, `ba94320` from a concurrently live session (see *Environment*).

---

## Answers to the four specific questions

### Q1 — Is the comparison genuinely inside the write's mutation in each adapter?

**Yes in all three, but the three guarantees are not identical. Stated precisely:**

| Adapter | What the comparison reads | What serializes it | Strength |
|---|---|---|---|
| Memory (`memory.rs:268–299`) | `self.slots`, the live authoritative map | `execute` is the whole mutation; `&mut self` is the serialization | Read and write are the same borrow. Genuine CAS. |
| Native (`native.rs:386–421`) | `self.manifest`, **re-read from disk at `native.rs:191`** inside the same `execute`, under the exclusive file lock taken at `native.rs:190` | `acquire_store_lock(&self.root)` → `File::lock()` on `store-v1.lock`; `_store_lock` is bound for the whole of `execute` | Not a stale cache. Verified by reading, not by claim: line 190 acquires, line 191 assigns `self.manifest = read_manifest(&self.root)?`, and the `match` on `request` follows. Genuine cross-instance CAS. |
| Browser (`wasm.rs:663–695`) | `references`, decoded from a `get_all` issued on `SLOTS_OBJECT_STORE` **inside** the `IdbTransactionMode::Readwrite` transaction opened at `wasm.rs:1089` | One IndexedDB read-write transaction; the mutation closure runs in the `onsuccess` of the read request and writes through the same `IdbObjectStore` handle | Genuine transactional CAS. The metadata record lives in the same object store under `METADATA_KEY` (`write_metadata_sync`, `wasm.rs:860`), so the head read and the marker write cannot be split across transactions. |

Two supporting facts I verified rather than assumed:

- **`generations.first()` really is the head in all three.** Memory inserts at index 0
  (`memory.rs:170`) and truncates to 2; native's `validate_manifest` (`native.rs:808–813`) and the
  browser's `ReferenceState::validate` (`wasm.rs:145–150`) both reject empty `generations` and
  require strictly descending order. The `.expect(...)` calls beside each CAS are therefore
  justified by a validated invariant, not by hope.
- **Native's `persist_manifest` (`native.rs:641–659`) is temp-file + `sync_all` + `persist` +
  `sync_directory`.** The manifest replacement is atomic on disk, so the marker write is atomic too.

**Caveat on the native evidence.** `native_continue_selection_rejects_a_generation_another_instance_superseded`
(`tests/workshop_store.rs:300`) is *sequential*: instance A lists, instance B commits, then A
selects. That is exactly the right shape to prove the manifest is re-read from disk — a cached-
manifest CAS would both pass wrongly and clobber B's head back on the subsequent write — but it does
**not** exercise lock contention between two writers running at once. No test in this repository
does. Both halves belong in any claim made about this test.

### Q2 — Do commit and promotion clear the marker in the same mutation?

**Yes, in all three adapters, verified by reading.**

- Memory: `memory.rs:183` (commit) and `memory.rs:239` (promotion) call `clear_continue_for`
  (`memory.rs:334`) after `self.slots = candidate`, inside the same `execute`.
- Native: `native.rs:282` (commit) and `native.rs:343` (promotion) call the free function
  `clear_continue_for(&mut manifest, slot)` (`native.rs:778`) on the **candidate** manifest, *before*
  `self.persist_manifest(manifest)?`. One atomic replacement carries both the new head and the
  cleared marker. The implementer's claim is exactly right.
- Browser: `wasm.rs:434` (commit) and `wasm.rs:515` (promotion) call `clear_continue_for_sync`
  (`wasm.rs:848`) inside the `mutate_references` closure, through the same `slots_store` handle the
  slot reference was just written through. Archive bytes, slot reference and cleared marker are one
  transaction; an abort publishes none of them.

`clear_continue_for` is correctly conditional on the marker naming *this* slot in all three, so a
commit to slot B does not disturb a marker on slot A.

### Q3 — Is `StaleGeneration` reachable today, and are its tests adequate?

**Unreachable from the session as shipped; the implementer is right.** `session.rs:790–797` sets
`self.slot = Some(ResidentSlot { generation, head_generation: generation })` from the commit result,
then `session.rs:958–962` issues `SelectContinue { expected_generation: resident.head_generation }`.
The expectation is by construction the generation the same session's commit just produced, and the
Commit lane is held by that session. It becomes reachable when a second client (the Library, or a
second native instance over one root) commits to the same slot — which is precisely the case
`tests/workshop_store.rs:300` already builds.

**Store-level coverage is adequate.** The error is pinned at three distinct provenances: a never-
existing generation, a genuinely superseded one after a same-instance commit and after a promotion,
and a cross-instance supersession. Each asserts on `expected` and `actual` payloads, not merely on
the variant, so a store returning the wrong operands fails.

**Session-level coverage is zero, and that is Finding 3.** See below.

### Q4 — Coverage honesty: what does `tests/workshop_store_web.rs` actually prove?

**The implementer's claim is confirmed, and I would state it more bluntly.**
`IndexedDbTransactionModel::start` (`src/workshop/store/web.rs:61–77`) clones a
`MemoryWorkshopStore`, calls `candidate.execute(request)`, and publishes the clone only if no
injected failure fires. So:

- `model_continue_selection_is_generation_checked_and_cleared_by_every_new_head`
  (`tests/workshop_store_web.rs:394`) executes **`memory.rs`'s CAS**, not `wasm.rs`'s.
- The one fact it adds beyond the memory test is the abort case: a successful selection whose
  transaction aborts publishes nothing and leaves the marker on the old head. That is a real and
  worthwhile proof of the transaction-publication contract.
- **It proves nothing about `src/workshop/store/web/wasm.rs`.** That file executes nowhere in this
  repository. Its only evidence is (a) it compiles for `wasm32-unknown-unknown`, (b) it passes
  clippy at `-D warnings` for that target, and (c) the substring contract in
  `wasm_source_uses_atomic_transactions_and_bounded_diagnostics` (`tests/workshop_store_web.rs:503`),
  which is a source scan, not a behavioural test.

The suite name `_web` implies browser coverage it does not have. The test's own doc comment says so
honestly ("only compilation covers that file on the host"), which is to the implementer's credit.

---

## The `tests/workshop_recovery.rs` fixture change, assertion by assertion

**The count is right: exactly seven tests reach the changed `commit` helper.** Call sites are lines
239 (in `failed_promotion_runtime`, reached by two tests), 267 and 295 (in
`authority_invalid_head_is_promoted_and_retained`, instantiated twice), 329, 453 and 509.

**The central finding: the change is state-preserving.** The Continue marker is an
`Option<SlotId>` — it records no generation. Trace both worlds for the shared fixture shape
`create_selected(slot) → commit(slot)`:

| | Before `8af2eae` | After `8af2eae` |
|---|---|---|
| `create_selected` | marker ← `Some(slot)` | marker ← `Some(slot)` |
| `CommitSlot` | marker untouched (old `CommitSlot` never cleared) | marker ← `None` |
| helper's new `SelectContinue` | — | marker ← `Some(slot)` |
| **Observable end state** | **`Some(slot)`** | **`Some(slot)`** |

Every `commit` call in this file targets the slot that the immediately preceding `create_selected`
selected, so the identity holds at every call site. No test in this file observes the store between
the commit and the re-select. And `grep -n selected_continue tests/workshop_recovery.rs` returns
**zero hits** — this file never asserts the marker directly at all.

Per-assertion verdict for the seven:

I read all seven bodies at `8af2eae`. Every one of them reaches `finish_recovery_promotion`
(`:153`), directly or through `advance_promotion_backoff` (`:254`), and that helper **panics** unless
`continue_available()` becomes true within its bounded polls. So the verdict is uniform:

| Test | Where the verdict rests | Power |
|---|---|---|
| `memory_recovery_promotes_authoritative_bytes_and_retains_the_known_good_predecessor` (`:315`) | `finish_recovery_promotion` at `:280`; `assert!(runtime.continue_available())` at `:273` and `:288` | **UP** |
| `indexeddb_model_recovery_...` (`:320`) | same generic body | **UP** |
| `failed_recovery_promotion_stays_blocking_and_retries_after_bounded_backoff` (`:325`) | `finish_recovery_promotion` at `:361`; `assert!(runtime.continue_available())` at `:363`; and `:371` `assert!(reopened.continue_available())` on a **fresh runtime after a store reopen**, which reads the marker back out of the store during bootstrap — the closest thing in this file to a direct store-marker assertion | **UP (strongest)** |
| `failed_recovery_promotion_blocks_session_load_until_the_obligation_completes` (`:376`) | `advance_promotion_backoff` at `:390`; `assert!(runtime.continue_available())` at `:391` | **UP** |
| `failed_recovery_promotion_blocks_session_import_until_the_obligation_completes` (`:408`) | `advance_promotion_backoff` at `:423`; `assert!(runtime.continue_available())` at `:424` | **UP** |
| `native_recovery_survives_promotion_exit_reopen_and_a_second_corrupt_head` (`:449`) | `finish_recovery_promotion` at `:472`; `assert!(runtime.continue_available())` at `:481` | **UP** |
| `native_authority_invalid_head_is_promoted_with_the_exact_recovered_predecessor` (`:505`) | `finish_recovery_promotion` at `:528`; no explicit `continue_available()` assertion, so the guard is the helper's panic rather than an `assert!` — weaker in expression, same in effect | **UP (implicit)** |

**Why "UP" rather than "EQUAL".** `continue_available()`
(`src/app/client_runtime.rs:306`) routes through `resident_workshop_continue_ready()`
(`:866`) while a Workshop session is resident, which calls `WorkshopSession::continue_ready()`
(`src/workshop/session.rs:280`). That requires **both**
`continue_selected_generation == self.slot.map(|s| (s.slot, s.head_generation))` and
`recovery_persistence.is_none()`. Both are set only in the `ContinueSelected` arm at
`session.rs:816–846`, and that arm now additionally requires
`selected_generation == generation` — the field this commit added. So
`finish_recovery_promotion` blocks until the session's post-promotion
`SelectContinue { expected_generation: N+2 }` succeeds **against a store whose
`PromoteRecoveredSlot` just cleared the marker**, and until the store echoes back the exact
generation. These assertions are now end-to-end guards on the whole new mechanism. That is a
real increase in discriminating power, not a rationalization.

**No assertion in this file was deleted or relaxed.** The diff to the file is +23/−2; the two removed
lines are the two `SelectContinue { slot }` call sites replaced by their generation-carrying form,
and `complete()` (`:55`) panics on any store error, so each added `SelectContinue` is itself an
assertion that the selection succeeded.

### The one changed test in `tests/workshop_store.rs`

`native_store_recovers_the_previous_valid_generation_without_moving_the_head` had its
`SelectContinue` moved from before the commit to after it (`tests/workshop_store.rs:894–902`). Same
identity as above: end state `Some(slot)` either way. Its terminal assertion
`assert_eq!(slots.selected_continue, Some(slot))` is **preserved verbatim** and its power is
**EQUAL at worst** — arguably up, since the marker must now survive a clear-and-reselect across a
store reopen rather than merely never having been cleared.

The remaining test edits in `workshop_store.rs` (`:359`, `:382`, `:437`, `:1010`, `:1072`) and
`workshop_store_web.rs` (`:232`, `:270`, `:334`, `:377`) are mechanical: each threads the correct
head generation into an existing call. I read each one against the store state at that point; every
one passes the true head, so none was made to pass by supplying a generation the store would accept
for the wrong reason. Power **EQUAL** for all nine, and marginally up in the two archived-precedence
cases, which now pass a deliberately wrong `SaveGeneration(999)` so that `ArchivedSlot` must outrank
`StaleGeneration` rather than merely being the only possible error.

---

## Verification of the implementer's specific claims

### The result-type extension was authorized — by the spec, not by the brief

**Confirmed, and the implementer was right to follow the spec over the brief.** The addendum §4
contains this verbatim:

```rust
WorkshopStoreResult::ContinueSelected {
    slot: SlotId,
    generation: SaveGeneration,
}
```

`ContinueSelected { slot, generation }` is a literal requirement of the binding contract, not an
unrequested extension. It also turns out to be load-bearing: `session.rs:823` uses it, and the five
recovery assertions above depend on it.

### Mutation A2 — "any retained generation" is caught only by the race test

**Verified empirically, and the claim is exactly right.** I replaced the memory CAS
(`memory.rs:287`) with `!record.generations.iter().any(|held| held.generation == expected_generation)`,
restoring afterwards.

- `cargo test --test workshop_store --test workshop_store_web --test workshop_recovery --test workshop_client`
  → `A2_EXIT=101`. Exactly one failure in `workshop_store`:
  `memory_continue_selection_is_generation_checked_and_cleared_by_every_new_head`, panicking at
  **`tests/workshop_store.rs:203`** — the commit-race assertion `select(store, slot, observed)`.
- Run separately: `workshop_store_web` → one failure,
  `model_continue_selection_is_generation_checked_and_cleared_by_every_new_head` at
  **`tests/workshop_store_web.rs:440`**, the same race assertion (as expected — the model delegates
  to `memory.rs`).
- `native_continue_selection_...` passed, correctly: native was not mutated.
- Every other assertion in the shared body survived the mutation, exactly as claimed: the off-head
  case uses `SaveGeneration(2)` against a head of 1 (never retained), the post-promotion case uses
  `second` against `[3, 1]` (not retained), and the precedence cases use `SaveGeneration(999)`.
- Restore verified: `git checkout -- src/workshop/store/memory.rs`, `git status --porcelain` empty,
  file byte-identical to a pre-mutation copy, and the four suites back to `RESTORE_EXIT=0`.

**Conclusion: the CAS is a real compare-against-head, and exactly one assertion in the repository
proves it.** That single assertion runs against all three adapters, which is adequate — but it is a
single point of failure. If anyone ever "simplifies" the race section of
`continue_selection_is_generation_checked`, the CAS silently degrades to a membership test.

### Error precedence Unknown → Archived → Stale

**The ordering is correct and is pinned by assertions. The reasoning is sound but is inference, not
a spec mandate — record it that way.**

- Pinned: `tests/workshop_store.rs:252–272` and `tests/workshop_store_web.rs:485–498` assert
  `UnknownSlot` for `SlotId(9)` with a plausible generation, then `ArchivedSlot` for an archived
  slot with a deliberately wrong `SaveGeneration(999)`, then `StaleGeneration` for the same wrong
  generation after unarchiving. The `999` is what makes the middle assertion discriminating: only
  precedence can produce `ArchivedSlot` there.
- The addendum does **not** state an order. §3 ("An archived row cannot be opened or selected for
  Continue until it is explicitly unarchived") and §10 ("No archived slot is opened or selected for
  Continue") establish archived-ness as an eligibility rule; §4's freshness conflict is remedied by
  "a retry after refresh". The implementer's argument — an eligibility failure no refresh cures
  outranks a freshness failure a refresh does cure — is the right reading, and it matches
  `CommitSlot` and `PromoteRecoveredSlot` in all three adapters (`memory.rs:135–174`, `:186–239`).
  Consistency across the three mutation requests is the stronger justification and it holds.

### The fixture rationale is over-argued (see Finding 2)

The helper's doc comment (`tests/workshop_recovery.rs:82–90`) says a fixture stopping at the commit
"would build a store state the product can no longer produce". **That is false.** Addendum §4 says
explicitly: "If the process stops between those operations, startup exposes no selected Continue
candidate rather than opening a generation that was never explicitly selected", and §12 lists
"process failure after Commit or Promote but before Select Continue" as required race qualification.
Commit-without-select is a product-reachable state — *more* reachable after this commit than before
it, because the commit now actively clears the marker. The fixture change is still correct and still
state-preserving; only the stated reason is wrong.

---

## Findings

### 1. MEDIUM — One assertion, repo-wide, distinguishes the CAS from a membership test
`tests/workshop_store.rs:203` (and its hand-copied twin `tests/workshop_store_web.rs:440`).
Proven by mutation A2: every other assertion in the shared contract body survives a CAS weakened to
"any retained generation". The shared body's structure hides this — it reads like six independent
checks, five of which are insensitive to the property the test is named for.
**Suggestion:** add one assertion that is sensitive by construction — after the commit to `second`,
select with the *retained predecessor* generation explicitly and assert `StaleGeneration { expected:
first, actual: second }`, with a comment naming the membership-test mutation it kills. Cheap, and it
removes the single point of failure.
**Status:** Open.

### 2. MEDIUM — The fixture doc comment states a false reason, and the state it dismisses is untested
`tests/workshop_recovery.rs:82–90`. "A store state the product can no longer produce" contradicts
addendum §4's crash clause and §12's race qualification. Separately, the natural home for that
clause at the runtime level — create, commit, **no** select, fresh bootstrap, assert
`!continue_available()` — does not exist. `continue_remains_disabled_when_storage_has_no_explicit_selection`
(`tests/workshop_client.rs:260`) uses an *empty* store, so it never exercises "slot exists, head is
valid, marker absent".
**Suggestion:** correct the comment to "the sequence the addendum mandates for a completed save",
and add the missing runtime test. The reviewed commit makes the state strictly easier to reach.
**Status:** Open.

### 3. MEDIUM — The session's `SelectContinue` failure arms have zero coverage
`src/workshop/session.rs:838–846` (result-mismatch → `StoreProtocol` diagnostic) and `:863–871`
(`Err(_)` → `defer_continue_selection_retry`, status "Saved; Continue selection failed").
`grep` across `tests/` for `"Continue selection failed"`, `"stale slot or generation"` and
`"Saved and selected for Continue"` returns **nothing**. The `selected_generation == generation`
guard this commit added at `:823` is therefore exercised only on its true branch; the else-branch it
introduced is dead in test. Today that is defensive-only (Q3: the error is unreachable from the
session), but the moment the Library commits to the same slot it becomes a live path — and
`defer_continue_selection_retry` (`session.rs:986`) simply clears `select_continue_requested` with
no refresh, so a stale conflict currently ends in a silent give-up rather than §4's "retry after
refresh". §4 assigns the refresh to the Library client, which is not built yet, so this is a
follow-on obligation rather than a defect of this commit — but the arm should not reach the Library
slice untested.
**Suggestion:** a `FailSelectContinueOnce` store wrapper in the shape of the existing
`FailPromotionOnce` (`tests/workshop_recovery.rs:198`), asserting the diagnostic, the status string,
and that `continue_ready()` stays false.
**Status:** Open.

### 4. LOW/MEDIUM — `wasm.rs`'s clear-on-new-head had no host-side guard at the reviewed commit
`src/workshop/store/web/wasm.rs:434` and `:515`; contract test at `tests/workshop_store_web.rs:503`.
The source-scan contract gained no term for `clear_continue_for_sync`. Deleting **both** call sites
would make the private helper dead code and trip wasm clippy at `-D warnings`; deleting **one**
tripped nothing at all. Hoisting a call *out* of its `mutate_references` closure — the atomicity
property the doc comments claim — still trips nothing, because a substring scan cannot see nesting.
**Note:** a concurrently live session committed `ba94320` ("count both clear-on-new-head call sites
in the browser store") during this review, which adds a counted contract asserting
`clear_continue_for_sync(references, slots_store, slot)?` appears exactly twice. That closes the
delete-one half. The hoist half remains open by that commit's own admission. Recording the finding
against `8af2eae` regardless, since the review is of that commit.
**Status:** Partially addressed by `ba94320` (not part of this review); hoist case Open.

### 5. LOW — "the same body" is not the same body
`tests/workshop_store.rs:158–163`: "`tests/workshop_store_web.rs` runs the same body against the
IndexedDB transaction model, so memory, native and the browser model are held to one description of
the behavior rather than three drifting copies."
It is a hand-written parallel copy (`tests/workshop_store_web.rs:394`), not the same function, and
**it has already drifted**: the web copy adds an `inject_next_failure(Abort)` case the shared body
lacks, and omits the shared body's post-`ArchiveSlot` `selected_continue == None` reassert, its
final successful re-select, and the closing `Some(slot)` marker assertion. The comment claims the
exact property that is untrue.
**Suggestion:** either hoist the body into `tests/common/` (which already exists) and call it from
both, or reword the comment to "a parallel body, deliberately kept in step" and list the two
intentional differences.
**Status:** Open.

### 6. LOW — Precedence is recorded as if the spec mandated it
`src/workshop/store.rs:368–377`: "Errors are ordered exactly as `CommitSlot` orders them". The
ordering is right and consistent, but the addendum states no order; it follows from §3/§10 plus
parity. Doc-comment prose in this repository is read as contract elsewhere.
**Suggestion:** one clause — "the addendum does not fix an order; this follows from §3 treating
archived-ness as an eligibility rule and from parity with the other two mutation requests".
**Status:** Open.

### 7. INFORMATIONAL — Native lock contention is unproven
`tests/workshop_store.rs:300` is sequential and proves the disk re-read, not concurrent-writer
locking. No test in the repository exercises `acquire_store_lock` under contention. This is not a
gap this commit created and it may not be worth closing; it is worth not overclaiming.
**Status:** Noted.

---

## Gates run

Working tree at the time of the run, on `nightly-2026-09-01` (`rust-toolchain.toml`), exit codes read
out of redirected log files and cross-checked against each log's own summary. No `| tail`, no
trailing `echo` standing in for a command's status.

| Gate | Result | Evidence |
|---|---|---|
| `cargo fmt --all --check` | **0** | `/tmp/nyon-review-gate.log:1` |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` (first run, tree carried another session's uncommitted edit) | **101** | `/tmp/nyon-review-gate.log:31` — **not caused by `8af2eae`**; the sole error was `clippy::single_element_loop` at `tests/workshop_store_web.rs:559`, in that edit. See *Environment* |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` (re-run at `ba94320`, clean tree) | **0** | `/tmp/nyon-review-clippy2.log:3`, zero `error` lines. **Note:** this is host clippy at `ba94320`, six commits past the reviewed one. Host clippy was never run against a `8af2eae` checkout in isolation, so clippy-clean *at `8af2eae` itself* is inferred, not measured. |
| `cargo test --workspace --all-targets` | **0** | `/tmp/nyon-review-gate.log:840`; 42 test binaries, **554 passed, 0 failed, 0 ignored** |
| `cargo check --target wasm32-unknown-unknown --lib` | **0** | `/tmp/nyon-review-wasm2.log`, after `cargo clean -p nyon --target wasm32-unknown-unknown` forced a real recompile |
| `cargo check -p nyon-workshop-core --target wasm32-unknown-unknown` | **0** | `/tmp/nyon-review-wasm.log` |
| `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings` | **0** | `/tmp/nyon-review-wasm2.log`, freshly compiled, no warnings |
| A2 mutation (memory CAS → membership test) | **101**, 1 failure per suite | `/tmp/nyon-a2.log`, `/tmp/nyon-a2web.log` |
| Post-restore re-run of the four suites | **0** | `/tmp/nyon-restore.log` |

The first `--workspace` run was used deliberately; `default-members = ["."]` would otherwise have
dropped `crates/nyon-workshop-core` and printed green.

### Test-count arithmetic — verified, not accepted

- Measured at review time: **554 passed across 42 binaries**, summed from the `test result:` lines.
- `git show <c> | grep -c '^+#\[test\]'`: `8af2eae` **+4, −0**; `2b7bfbc` **+1**; `b6f33ab` **0**;
  `06873f6` **0**.
- Therefore `8af2eae` ended at **554 − 1 = 553**, and the baseline was **553 − 4 = 549**. Binary
  count unchanged at 42 (no new test file). **Both the implementer's 553 and the brief's 549 and 554
  are correct.**
- The four added tests are `memory_continue_selection_...` (`tests/workshop_store.rs:275`),
  `native_continue_selection_...` (`:280`), `native_continue_selection_rejects_a_generation_another_instance_superseded`
  (`:300`), and `model_continue_selection_...` (`tests/workshop_store_web.rs:394`).

---

## Environment — read this before trusting the clippy line

**A second agent session was live in this checkout throughout the review, and it is the sole cause of
the red clippy gate.**

- A stray untracked `tests/zz_probe_tmp.rs` (571 bytes, mtime 19:01, self-described "TEMPORARY PROBE
  - deleted immediately after reading") was present at the start. It **did not compile** (two
  unresolved imports) and failed `cargo fmt --check`, so the first gate run I made was red for that
  reason alone. It was deleted by the other session mid-review, not by me.
- That session then added a counted `clear_continue_for_sync` contract to
  `tests/workshop_store_web.rs`. The **only** clippy error in
  `/tmp/nyon-review-gate.log:3–20` is `clippy::single_element_loop` at
  `tests/workshop_store_web.rs:559` in that uncommitted edit. It has since been committed as
  `ba94320`, and the lint was fixed on the way in: a re-run of the same clippy command on the clean
  tree at `ba94320` exits **0** with zero error lines (`/tmp/nyon-review-clippy2.log`).
- **Conclusion: `8af2eae` itself is fmt-clean, test-green, and wasm-clean, and host clippy is green
  at `ba94320` on a clean tree. The clippy red in the first run belongs to work that is not part of
  this commit. Host clippy at `8af2eae` in isolation was not measured — see the gate table.**
- `main` moved from `06873f6` to `ba94320` during the review (`1ae4e3a`, `a0b1371`, `ba94320`).
  `1ae4e3a` is a second, independently written review of this same commit at
  `docs/superpowers/reviews/2026-09-08-workshop-task-3-review.md`. This file does not overwrite it.

**Actions I took in the tree, all reverted or additive:**

- Applied the A2 mutation to `src/workshop/store/memory.rs` and reverted it with
  `git checkout --`; verified byte-identical to a pre-mutation copy at
  `/tmp/nyon-memory-backup.rs` and confirmed the four suites green afterwards.
- Ran `cargo clean -p nyon --target wasm32-unknown-unknown` to force a genuine wasm recompile rather
  than trusting a cached "Finished". It reported **"Removed 35485 files, 5.9GiB"** — more than the
  wasm slice, so the concurrent session may face a rebuild. No source file was touched, and I
  deliberately did **not** `touch` any source, because `tools/check-workshop.sh` compares `dist/`
  mtimes against build inputs.
- Wrote this file. Nothing else.

---

## What I could not check

Stated explicitly, because this boundary matters more than the verdict.

1. **`src/workshop/store/web/wasm.rs` was never executed.** No browser run, no `wasm-bindgen-test`,
   no headless harness exists in this repository. Everything I assert about the browser adapter's
   CAS and clear-on-new-head comes from **reading the source and the `web-sys` transaction
   semantics**, plus a compile and a clippy pass. If `mutate_references`' `onsuccess` closure does
   not behave as read — for example if a real IndexedDB implementation delivered the `get_all`
   result after another transaction interleaved — nothing here would have caught it. Treat the
   browser conclusions as *code review*, not as *evidence*.
2. **Native lock contention under genuinely concurrent writers.** See Finding 7.
3. **Whether `File::lock()` gives cross-process exclusion on every target platform.** It is
   advisory on Unix and I did not test Windows or a network filesystem. The reviewed code does not
   change this; it inherits it.
4. **The gate numbers describe the working tree at run time**, not a clean `8af2eae` checkout. The
   fmt/test/wasm runs were made with the other session's uncommitted `workshop_store_web.rs` edit
   present (one assertion added inside an existing test, so the 554 count is unaffected); the clippy
   re-run was made at `ba94320` on a clean tree. Building `8af2eae` in isolation would have required
   a separate build tree, which I judged not worth forcing on a checkout another session was using.
5. **The 549 baseline itself.** I verified it by subtraction from a measured 554 and from counted
   `#[test]` deltas across four commits; I did not check out `8af2eae^` and run it.
6. **Whether the Library client (addendum §4's conflict-refresh-retry) will use these primitives
   correctly.** That slice does not exist yet; Finding 3 is written against its arrival.

---

## Summary

The three obligations are discharged correctly and the code is better than the brief asked for in
one respect the spec required. The comparison is inside the mutation in all three adapters, with
native's disk re-read under an exclusive lock and the browser's read and write inside one read-write
transaction — both verified by reading, not by claim. Commit and promotion clear the marker in the
same mutation, natively in a single atomic manifest replacement.

The high-risk part of the diff is clean. The `tests/workshop_recovery.rs` fixture change does not
weaken anything: the marker carries no generation, so the store state those seven tests observe is
identical before and after, and all seven gained real end-to-end power through
`continue_ready()`'s dependence on the new `generation` field. The one stated justification for the
change is wrong (Finding 2), which is a documentation defect, not a test defect.

The findings that matter are about *thinness*, not error: one assertion repo-wide separates a real
CAS from a membership test, the session's failure arms are untested against an error that a second
client will soon make reachable, and a suite named `_web` proves nothing about the browser file.
None of that blocks the commit.
