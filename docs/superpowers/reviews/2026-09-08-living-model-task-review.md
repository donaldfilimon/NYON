# Review: `50c50f9` — feat(living): freeze the V2 authority state schema

Status: **APPROVE WITH FINDINGS**

Reviewer: post-hoc scrutiny of a commit whose implementing agent died on a rate limit before
writing any report. There is no implementer claim set to check against, so every statement below
is either something I read in the tree, something I ran, or something I explicitly mark as
inferred.

Range: `c4398e7..50c50f9`. Files: `crates/nyon-workshop-core/src/living/model.rs` (new, 1848),
`crates/nyon-workshop-core/tests/living_model.rs` (new, 966),
`crates/nyon-workshop-core/src/living/mod.rs` (+17),
`crates/nyon-workshop-core/tests/living_wire.rs` (+7/-3).

Nothing in this review was fixed. Four source mutations and one scratch test file were used as
evidence and were reverted; `git diff --name-only -- crates/` is empty as of writing.

---

## Verdict summary

The transcription is correct. The 24-field order matches the spec table field for field, the
digest cannot see an allocator mark, the two `living_wire.rs` edits are both done and correctly
prefixed, no V1 vocabulary or `unsafe` appears, no constant is redefined, and no genesis type
leaked in. The tests are not decode round-trips wearing obligation names: four mutation
experiments showed real failures at the expected places.

What holds it back from a clean APPROVE is one measured coverage hole (F1: **record-level** field
order is pinned by nothing, verified by mutation) and two completeness gaps in a `validate()`
method whose own doc comment overclaims what it checks (F2, F3). None of these is a byte-format
error in the committed schema; all three are things that will silently permit one later.

---

## What I verified by running

Gate, run from the repo root with `--workspace`, exit codes read out of the log files rather
than through a pipe or trailing `echo`:

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | `FMT_EXIT=0` |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `CLIPPY_EXIT=0` |
| `cargo test --workspace --all-targets` | `TEST_EXIT=0`, **549 passed across 42 suites** |

549/42 matches the stated baseline exactly; nothing dropped.

Mutation experiments (each applied to `src/living/model.rs`, run as
`cargo test -p nyon-workshop-core --lib --test living_model`, then reverted with `git checkout --`):

| # | Mutation | Outcome |
| --- | --- | --- |
| M1 | Swap top-level fields 5 (`stars`) and 6 (`worlds`) | **2 failures**, both unit tests in `model.rs`. The 50-test integration file passed. |
| M2 | `interval()` `<` becomes `<=` (empty intervals accepted) | **4 failures** in `living_model.rs`. |
| M3 | Drop the nested-hull `flat_map` from `check_global_identity_uniqueness` | **1 failure**: `a_nested_hull_shares_the_one_global_identity_space`. |
| M4 | Swap `archetype` and `name` **inside** `LivingWorldV2` | **0 failures. Exit 0 across `--lib`, `living_model`, `living_wire`, `living_catalog`.** |

Scratch decode probes (temporary `tests/zz_scratch.rs`, since deleted):

- `LivingGalaxyStateV2` with a trailing `"accepted_sequence_high_water":0` → rejected
  (`LivingWireErrorV2::Json`). An allocator mark cannot be smuggled into a state document.
- `LivingFleetLocationV2` as `{"type":"docked","world":"…","extra":1}` → rejected; the
  well-formed variant without `extra` decodes. So `#[serde(deny_unknown_fields)]` **is** honored
  on this crate's internally tagged enums. That was worth measuring rather than assuming.

---

## Answers to the six questions

### 1. Field order — matches exactly. Verified by reading both sides.

Spec §10 "Authoritative state schema" table vs `model.rs:1128-1179`:

`tick`, `accepted_sequence`, `branch_sequence`, `systems`, `stars`, `worlds`, `lanes`,
`civilizations`, `deposits`, `colonies`, `facilities`, `construction_jobs`, `hull_jobs`,
`fleets`, `routes`, `shipments`, `relations`, `agreements`, `wars`, `observations`, `hazards`,
`settlement_claims`, `occupations`, `counters` — 24 fields, same names, same order, and the
`/// N.` numbering on each field matches its row. Fields 4–23 are all `LivingSortedVecV2<_>`;
field 24 is `LivingCountersV2 {}` with braces (`model.rs:1117`), which encodes `{}` and not
`null` as the spec requires. `model.rs:1772` pins the whole 24-name order as a byte literal.

The same list also matches the civilizations plan line 97 verbatim.

### 2. `deny_unknown_fields` — present everywhere it is meaningful. Controller's classification confirmed in substance, refined in detail.

31 occurrences. Nine declared types lack it. My census, read from the file:

- **Seven serde unit-variant enums** — `LivingResourceV2:145`, `LivingCivilizationStatusV2:361`,
  `LivingFacilityStatusV2:467`, `LivingFleetOrderKindV2:631`, `LivingRouteKindV2:694`,
  `LivingShipmentDispositionV2:752`, `LivingAgreementKindV2:871`. The attribute is a no-op on
  these.
- **One `#[serde(transparent)]` newtype** — `LivingPolicyV2:371`. No-op.
- **One enum that is not serde at all** — `LivingValidationErrorV2:97` derives
  `Clone, Copy, Debug, Eq, PartialEq, thiserror::Error` only.

So the count is 7 + 1 + 1, not the "eight unit enums plus one newtype" the controller recorded;
the error enum was miscounted as a unit enum. The conclusion is unchanged and is the one that
matters: **both struct-shaped enums carry the attribute** — `LivingBlockedReasonV2:209`
(`tag = "type"`, three struct variants) and `LivingFleetLocationV2:609` (`tag = "type"`, two
struct variants) — and the scratch probe proves serde honors it there. No real gap.

See F5 for the fact that no test in the commit exercises this.

### 3. Allocator marks and `state_digest` — no path exists. This is the strongest part of the commit.

`digest()` (`model.rs:1192`) is `state_digest_v2(&self.canonical_bytes()?)`, and
`canonical_bytes()` (`model.rs:1183`) is `encode_canonical_v2(self, …)` over the struct. The
struct has 24 fields and none of them is a live mark; fields 2 and 3 are typed `u64` and
documented as the historical values (`model.rs:1132-1139`, and the module header at
`model.rs:33-37`). There is no allocator type in scope: `model.rs` imports only
`LivingEntityIdV2`, `LivingStateDigestV2`, `LivingTickV2`, `state_digest_v2` from `ids.rs`.

Three independent confirmations: the exact-bytes pin at `model.rs:1772` leaves no room for an
extra field; `deny_unknown_fields` on the state struct plus my scratch probe rejects one on
decode; and `the_state_carries_no_allocator_high_water_field` (`model.rs:1794`) asserts the
substring `high_water` is absent. The third of those is the weakest of the three on its own
(a differently named field would pass it) but it is redundant with the first, which is exact.

### 4. Test quality — real obligations, with one measured exception.

57 tests, not 57 in one file: **50 integration tests** in `tests/living_model.rs` plus **7 unit
tests** in the `mod tests` at `model.rs:1763`. Worth stating because the seven carry
disproportionate weight (see F1).

The integration tests follow a consistent and honest shape: build a valid state, assert it
validates, mutate one field, assert the *specific* typed error. `expect_capacity`,
`expect_dangling`, `expect_range`, `expect_interval` all match on the error variant, and
`expect_range`/`expect_dangling` also compare the `field`/`reference` label, so a test cannot be
satisfied by the wrong rejection. Several assert both sides of a boundary rather than only the
failing side — `occupation_progress_stops_at_the_transfer_boundary` passes at 100 and fails at
101; `systems_are_bounded_at_sixty_four` validates at 64 and rejects at 65.

M2 and M3 confirm this empirically: breaking the interval predicate failed four tests, and
deleting the nested-hull identity collection failed exactly the test named for it.

The exception is field order, covered in F1 below. Also see F8 for the one place where the
`expect_range` label discipline is undermined by the production code reusing labels.

### 5. Task 3b — nothing leaked, but `validate()` is unplanned scope. See F3.

Zero occurrences of `genesis`, `manifest` (as a type), or `generator` in `model.rs` apart from
one ordinary use of the English word "manifest" in the shipment doc comment (`model.rs:765`).
No `LivingGenesisManifestV2`, `LivingGenesisGeneratorV2`, or `ValidatedLivingGenesisV2`, and no
`Validated…` wrapper of any kind. 3b's types are untouched.

### 6. Frozen record types — all 29 present, plus the topology types the state fields require.

Plan `2026-09-04-nyon-living-galaxy-civilizations.md` line 99 freezes 29 types. All 29 are
declared in `model.rs` and all 29 are re-exported from `living/mod.rs`. Nothing named there is
absent.

`model.rs` additionally declares `LivingSystemV2`, `LivingStarV2`, `LivingWorldV2`,
`LivingLaneV2`, `LivingDepositV2`, `LivingResourceV2`, `LivingFleetOrderKindV2`,
`LivingObservedFacilityV2`, `LivingObservedHullV2`, `LivingValidationErrorV2`, and
`LivingGalaxyStateV2`. Every one is required by a field the spec's own table names, so this is
not scope creep.

---

## Repo-specific mechanical rules

| Rule | Result |
| --- | --- |
| V1/V2 vocabulary isolation, doc comments included | **Pass.** `grep -nE '\b(RevisionId\|EntityId\|BranchId\|CatalogHash\|StateDigest\|WorkshopTick\|BatchLocalId\|WORKSHOP_RULES_VERSION\|WORKSHOP_TICK_HZ)\b'` on `model.rs` returns nothing. The prose describes V1 equivalents without naming them ("the WorkshopV1 version", `mod.rs:60`). |
| Two-place `living_wire.rs` edit | **Pass, both places.** `CRATE_SOURCES` 12 → 13 (`living_wire.rs:503`) with the entry `("living/model.rs", …)` at line 518 — correct `living/` prefix, correct alphabetical slot. Count assertion 11 → 12 (`living_wire.rs:667`) with its message updated to "four living submodules". |
| No `unsafe` outside `lib.rs` | **Pass.** `grep -c unsafe model.rs` = 0. |
| No constant redefinition | **Pass.** `LIVING_RESOURCE_STORAGE_LIMIT_V2` is imported from `catalog.rs` (`model.rs:41`), not redeclared. `LIVING_COLONY_INDUSTRY_SLOTS_V2` is neither redeclared nor imported — see F2. |
| Section-1 maxima values | **Pass.** 16/64/128/512/256/1024/2048/2048/4096/256/2048/16/128 at `model.rs:53-81` match the spec §1 table row for row. `LIVING_MAX_INDUSTRIES_V2` bounds the state's `facilities`; the rename is documented at `model.rs:65-67`. |
| No re-implementation of `LivingSortedVecV2`'s sorted/duplicate check | **Pass.** `validate()`'s doc (`model.rs:1198-1201`) states the reason, and no ordering or duplicate-key loop appears. The one `windows(2)` in `check_claims_and_occupations` (`model.rs:1683`) checks a different property (shared `close_tick`), and it is sound because `LivingSettlementClaimV2::living_key` is `(world, civilization)` (`model.rs:1069-1074`), so same-world claims are adjacent. |

---

## Findings

### F1 — Medium — Record-level field order is guarded by nothing, and the file claims to be its authority

`crates/nyon-workshop-core/src/living/model.rs:16-24` (module header), `:1772` (the pin),
`crates/nyon-workshop-core/tests/living_model.rs:948-965`

The module header states that §10 does not publish the field list of each record, so "their
**order** is declared here", and that §10's "the schema-declared field order is normative" makes
this file that declaration. That is an honest and correct reading. But nothing pins it.

Measured (M4): swapping `archetype` and `name` inside `LivingWorldV2` — a canonical format break
by the spec's own rule, and a change that moves every `state_digest` of any state containing a
world — passes `--lib`, `living_model`, `living_wire`, and `living_catalog` with **exit 0 and
zero failures**.

The reason is structural, not an oversight in any one test. The only byte-exact pin
(`model.rs:1772`) is over the *empty* state, where all twenty collections are `[]`, so it
constrains the 24 top-level names and nothing inside them.
`the_reference_fixture_round_trips_through_canonical_bytes` (`living_model.rs:948`) is
order-blind: it encodes and decodes with the same struct definition, so it agrees with itself
under any order. `the_historical_sequences_are_carried_through_encoding_unchanged`
(`living_model.rs:961`) is a `starts_with` over fields 1–3 only.

M1 also showed the top-level order is defended by exactly two tests, both of them the unit tests
inside `model.rs`; the entire 50-test integration file is order-blind there too. That is
adequate for the top level but it means the guard is thinner than the test count suggests.

Suggestion: add one byte-literal pin over a **populated** state — the `valid_state()` fixture
already exists — so every record's field order is constrained. Task 3c's hand-authored
`minimal-state.json` will close part of this for the records it happens to contain, which is a
reason to sequence it soon rather than a reason to leave the gap; a fixture that omits a record
type still leaves that type unpinned.

Status: open.

### F2 — Medium — `validate()`'s doc overclaims: the six-slot colony limit is not checked, and `LIVING_COLONY_INDUSTRY_SLOTS_V2` is not imported

`crates/nyon-workshop-core/src/living/model.rs:1196-1197`, `:1219-1289`

The doc comment says validate "Check[s] **every** collection maximum, every intra-state
reference, every catalog reference, and the declared integer and interval ranges."

Spec §3: "Each colony has a persistent hub, **six industrial slots**, one building queue, and at
most one shipyard with one hull queue. A defense battery consumes one slot and is limited to one
per colony. Reserved building slots count toward the six-slot limit."

`check_capacities` bounds twelve global collections plus per-fleet and total hulls. It does not
bound per-colony facilities, does not count construction jobs against the slot limit, and does
not enforce one-battery-or-one-shipyard-per-colony. `LIVING_COLONY_INDUSTRY_SLOTS_V2` (6) exists
in `catalog.rs` and is not imported by `model.rs` at all (`model.rs:40-43`).

Whether the slot limit is a *schema* invariant or a *simulation* invariant is a genuine design
question — §3 also says "Creator-created noncolonized industry can exist", so the bound applies
to colony worlds only, not to every world. Either answer is defensible. What is not defensible
is the doc comment asserting the strong reading while implementing the weak one. Fix the
sentence or add the check.

Status: open.

### F3 — Medium — `validate()` is scope beyond plan 3a, and 3b will be tempted to write it again

`crates/nyon-workshop-core/src/living/model.rs:1202-1217`

Plan `wild-snuggling-treehouse.md` §3a specifies four things: the 24-field order, universal
`deny_unknown_fields`, fields 2/3 as historical with no digest path, and the reserved `counters`
record. It does not mention a validator. §3b, by contrast, does: `ValidatedLivingGenesisV2`
"follows `ValidatedLivingCatalogPackV2`'s shape — private document, typed accessors, decode then
validate then construct".

Roughly 520 lines of referential-integrity and range checking against a
`ValidatedLivingCatalogPackV2` therefore landed in the task that was not asked for it. It is
good code and it belongs on the state type rather than inside genesis, so I am not asking for it
to be removed. The risk is duplication: 3b's implementer, reading only §3b, will write the same
checks inside genesis validation. Also, capacity/reservation accounting is assigned to the
civilizations plan's `civilization.rs`, so there are now two plausible homes for the checks F2
names.

Suggestion: record in the plan that `LivingGalaxyStateV2::validate(&ValidatedLivingCatalogPackV2)`
exists and is 3b's validation step, so 3b composes rather than re-derives.

Status: open — a plan edit, not a code edit.

### F4 — Low/Medium — `construction_jobs` and `hull_jobs` are bounded by nothing

`crates/nyon-workshop-core/src/living/model.rs:1219-1289`

Spec §1: "Accepting a building or hull job **reserves its global entity capacity** and its
destination industry/fleet slot through completion… Creator operations count all outstanding
reservations."

`check_capacities` counts `facilities` against `LIVING_MAX_INDUSTRIES_V2` and total hulls
against `LIVING_MAX_HULLS_V2`, but adds neither `construction_jobs` nor `hull_jobs` to those
sums, and applies no independent bound to either collection. A state holding 2,048 facilities
*and* 2,048 construction jobs validates, as does one with 2,048 hulls and any number of
outstanding hull jobs, and a `hull_jobs` entry with `target_fleet: None` reserves a fleet that
is not counted against `LIVING_MAX_FLEETS_V2`.

Same caveat as F2: this may be deliberately deferred to the reservation accounting in
`civilization.rs`. It is not deferred *in writing* anywhere I could find, and the doc comment
claims otherwise, which is why it is a finding rather than a note.

Status: open.

### F5 — Low — No test in the commit exercises unknown-field rejection

`crates/nyon-workshop-core/tests/living_model.rs` (whole file)

`grep -i 'unknown\|deny'` over the test file returns nothing. `deny_unknown_fields` is on 31
types and is described in the plan as "load-bearing for error fidelity, not style", yet the
property is asserted nowhere. I confirmed by scratch test that it works today, including on the
two internally tagged enums, so this is a missing guard rather than a defect — but a future
`#[serde(flatten)]` (which silently disables it) or a dropped attribute would go unnoticed.

Suggestion: two tests — one on `LivingGalaxyStateV2`, one on `LivingFleetLocationV2` — asserting
a decode error on an extra field. The state one doubles as the positive form of the
`high_water` assertion at `model.rs:1794`.

Status: open.

### F6 — Low — Diplomacy reason bounds are magic numbers while every sibling bound is a named constant

`crates/nyon-workshop-core/src/living/model.rs:1573-1595`

`bounded(reasons.delivered_aid, 0, 20, …)`, `(…, 0, 20, …)`, `(…, -20, 0, …)`,
`(…, -30, 0, …)`, `(…, -60, 0, …)`. All five values are correct against the §6 table (aid cap
+20, trade cap +20, adjacent-settlement floor −20, war-declared floor −30, colony-captured floor
−60 — I checked each), and each is documented on its field at `model.rs:810-826`. But the same
commit introduces `LIVING_RELATION_BOUND_V2`, `LIVING_DELIVERY_ACCUMULATOR_BOUND_V2`, and
`LIVING_OCCUPATION_TRANSFER_TICKS_V2` for exactly this kind of value. The inconsistency will
read as an oversight to the next editor.

Status: open.

### F7 — Low — Doc comments attribute the delivery accumulators to §6; the spec's §10 says §5

`crates/nyon-workshop-core/src/living/model.rs:84`, `:1109`

`model.rs:1109` says "The per-relation delivery accumulators of section 6 are not this record".
Spec §10's `counters` paragraph says "The per-relation delivery accumulators of section 5 are not
this record; they live inside the relation record where section 5 places them."

The code is arguably right on the merits — the reason table with the 100-unit accumulators is in
§6 ("Diplomacy and bounded conflict"), not §5 ("Settlement and freight") — so this looks like a
spec error that the implementer silently corrected rather than a transcription slip. Either way
the two documents now disagree in writing. Fix the spec, or note the correction in the code.

Status: open — needs a decision, not a patch.

### F8 — Low — `LivingValidationErrorV2::Range` is overloaded, and reference labels repeat

`crates/nyon-workshop-core/src/living/model.rs:1355`, `:1398`, `:1477`, `:1627`, `:1688`, `:1700`

`Range { field }` is returned for genuine numeric range violations (`hull hit_points`,
`occupation progress_ticks`), for ordering violations (`lane endpoints`, `agreement
participants`, `war participants`), for cross-record consistency (`colony owner`, `war
declarer`), and for a shared-boundary violation (`claim close_tick`). A caller cannot tell a
number-out-of-band from a broken invariant without string-matching the `field` label.

Separately, `DanglingReference` labels are reused across distinct references: `"fleet world"`
appears three times (`:1448`, `:1456`, `:1457`), `"route owner"` and `"route world"` and
`"shipment owner"` twice each. When one fires, the label does not localize it.

The commit is explicitly built around error fidelity (that is the stated reason for
`deny_unknown_fields`); the same standard applied to the error enum would argue for distinct
`Ordering`/`Consistency` variants, or at least unique labels.

Status: open.

### F9 — Low — Only four of eight `interval()` call sites are covered by a test

`crates/nyon-workshop-core/src/living/model.rs:1739-1749`

M2 (changing `<` to `<=`) failed exactly four tests: construction-job duration, fleet leg,
shipment travel, and agreement interval. The remaining four call sites — hull-job duration
(`:1435`), war interval (`:1630`), hazard window (`:1672`), and claim window (`:1681`) — have
tests that exercise their *references* but never a degenerate interval. Cheap to close; the
existing tests already build the records.

Status: open.

### F10 — Low — Nothing forbids a truce and a trade agreement coexisting for one pair

`crates/nyon-workshop-core/src/living/model.rs:910-919`, `:1598-1618`

`LivingAgreementV2::living_key` is `(participant_a, participant_b, kind.as_wire_str())`, so the
sorted-vec uniqueness rule permits one record of each kind per pair simultaneously. §6 says
"War/truce blocks new trade and nonaggression", and `check_diplomacy` does not cross-check
agreement kinds against each other or against `wars`.

I flag this as an observation rather than a defect because it is plausibly a simulation
invariant that `diplomacy.rs` will own, and because a state schema that can represent an
expiring truce overlapping a still-listed trade record may be deliberate. Worth an explicit
decision, since the key choice freezes the representation.

Status: open — needs a decision.

### F11 — Info — The record layouts have no upstream authority, by the file's own admission

`crates/nyon-workshop-core/src/living/model.rs:15-24`

The header says plainly that "A reader comparing an independent implementation against the
specification alone cannot reproduce these bytes; that gap is recorded rather than papered
over." That is exactly the right disclosure and I want it noted as a credit, not a defect.

The consequence is that §10's promotion of the state table (commit `7200e8a`) solved the problem
one level deep. Recommend the same promotion for the record field lists, for the same reason:
otherwise `model.rs` is simultaneously the implementation and its own authority, which is the
condition Task 3c's "do not regenerate vectors from the implementation" rule exists to prevent.

Status: open — process, not code.

### F12 — Info — Two counts in circulation are slightly off

- "57 new tests in `living_model.rs`" — it is 57 total: **50** in `tests/living_model.rs` and
  **7** in the `#[cfg(test)] mod tests` at `model.rs:1763`. The distinction matters because F1
  and M1 show the seven carry the field-order guarantee alone.
- `~/dev/active/NYON/CLAUDE.md` states the `living_wire.rs` module-count assertion is "exactly
  10". It was 11 before this commit and is 12 after (`living_wire.rs:667`). That doc line is
  stale; not this commit's fault, but it will mislead the next module author.

### F13 — Info — Another session was writing to this checkout during the review

While I worked, `src/workshop/{session,store,store/memory,store/native,store/web/wasm}.rs` and
`tests/workshop_{client,recovery,store}.rs` went from clean to modified, with mtimes 18:23–18:25
on 2026-09-08. I did not touch those files. My gate run started at 18:19 and therefore reflects
the tree as of that moment; the 549/42 result should not be read as covering whatever that
session is currently writing. No file under `src/living/` or `tests/living_*` was affected, so
the findings above are unaffected.

---

## Things I did not check, stated plainly

- **I did not verify the record field lists against §§3–7 prose exhaustively.** I spot-checked
  the ones the spec enumerates near-verbatim (route: source, destination, resource, batch size,
  cadence, source reserve, participating owners — all present at `model.rs:712-739`; shipment:
  dispatch owner, intended receiver, units, departure/arrival, return status — all present at
  `model.rs:771-792`; observation: observed tick, owner, remaining deposit, facilities,
  stationed hulls — all present at `model.rs:996-1011`, with rival stockpile correctly absent).
  A field-by-field audit of all 29 records against the prose is a larger job than this review.
- **I did not verify any digest value against an independent computation.** No frozen vector for
  `LivingGalaxyStateV2` exists yet; that is Task 3c's job and the plan is explicit that it must
  be derived outside this crate. Nothing in this commit pretends otherwise, which is correct.
- **I did not audit `living/wire.rs`, `living/ids.rs`, or `living/catalog.rs`.** They are
  outside the range and I relied on their existing tests.
- **I did not run the fuzz targets or the wasm sequence.** Neither is reached by the three gate
  commands, and neither touches this module.
- **Whether the F2/F4 checks belong to this task or to `civilization.rs`** is a plan question I
  cannot settle from the artifacts; I have framed both as "the doc comment overclaims", which is
  true either way.

---

## Recommended disposition

Merge stands. Before Task 3b dispatches, resolve F3 (record `validate()`'s ownership in the
plan) so 3b does not duplicate it, and fix the F2 doc sentence, which is a one-line honesty fix
independent of whether the slot check ever lands here. F1 is the one worth real effort and is
best folded into 3c's fixture work: a byte-literal pin over a populated state, not only the
empty one.
