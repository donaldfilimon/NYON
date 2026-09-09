# Review: Living Galaxy authority Task 4 — commands, receipts, events, registries

**Verdict: APPROVE WITH FINDINGS.**

Status: review record, not a proposal. Reviewed range `4fc7f4b..78072b9` (five commits,
11 files, +3562/−42). Reviewed at working-tree HEAD `754b44c`, a docs-only commit
(`CLAUDE.md`, 23/10 lines) landed on top by a concurrent session; no reviewed source or
fixture differs between `78072b9` and `754b44c`.

The central discipline held. The frozen corpus was re-derived by a non-Rust path, and I
independently produced **stronger** evidence for that than the implementer's own control.
The findings below are one real mutation-surviving coverage gap, one normative-authority
question about what this diff freezes without a spec table, and a set of documentation and
coverage corrections. Nothing found requires renumbering an identity or invalidating a
vector.

---

## Evidence boundary

This section is the part that matters most, per the task framing. Everything below is
labelled by how it was established.

### Verified by running

| Check | Command | Result |
| --- | --- | --- |
| Repo gate, fmt | `cargo fmt --all --check` | `FMT_EXIT: 0` |
| Repo gate, clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `CLIPPY_EXIT: 0` |
| Repo gate, tests | `cargo test --workspace --all-targets` | `TEST_EXIT: 0`, **637 passed / 0 failed / 0 ignored across 46 `test result:` lines** |
| Vector control at HEAD | `python3 tools/living-v2-vectors.py verify` | exit 0, 65 OK / 0 MISMATCH |
| **Vector control on the pre-change corpus** | `git show 4fc7f4b:…/vectors.json > /tmp/vectors_old.json && python3 tools/living-v2-vectors.py verify /tmp/vectors_old.json` | **49 OK / 0 MISMATCH**, then `KeyError: 'creator_command'` (the tool's `verify` now requires the two new sections) |
| Baseline arithmetic | `cargo test -p nyon-workshop-core --no-fail-fast` at `4fc7f4b` vs HEAD | 187 tests / 12 suites → 233 / 14; **delta +46 tests, +2 suites**, so 637 − 46 = **591** and 46 − 2 = **44**. The claimed baseline 591/44 is exact. |

Exit codes were read from the command itself, never through a pipe or a trailing `echo`;
test totals were summed from the `test result:` lines rather than read off a summary.
Mutations ran in a disposable detached worktree (`/tmp/nyon-review-mut`) with an isolated
`CARGO_TARGET_DIR`, because another session commits to this repo. All review worktrees and
scratch target directories were removed; `git worktree list` shows only the canonical
checkout.

### The independence control, stated precisely

`tools/living-v2-vectors.py` at HEAD reproduces **all 49 checkable rows of the `4fc7f4b`
corpus**, including the six creator/autonomous digests that carried the *pre-renumber*
kinds 3, 4 and 7 — values this script never produced, derived in Task 1 by a different
tool recorded in that task's report.

What that proves: the Python computes the same function as an independently derived
corpus, and the Rust agrees with both (the Rust suite recomputes those same rows from
corpus inputs). Three-way agreement on 49 rows.

What it does **not** prove, and I state it plainly: it does not establish the script's
textual provenance. Whether its author read spec section 10 or `living/ids.rs` is not
recoverable from the artifacts. The formula ordering matches the spec's code block exactly
(`pack, state, archive_integrity, receipt, revision, creator, auto, root, fork, event,
claim`), but `ids.rs` declares them in that same order, so ordering is not discriminating
evidence. The behavioural control above is the strongest available substitute and it is
sound, because a script transcribed from a *wrong* `ids.rs` would have failed the old
corpus.

Note that `verify` at HEAD is partly tautological for the 16 rows added by this task:
Python checking values Python produced. The independent check on those is the Rust suite —
`the_reviewed_batch_encodes_to_the_reviewed_command_bytes`, `sealing_the_reviewed_batch_
produces_the_reviewed_revision`, `the_batch_creates_the_reviewed_entity_identities`,
`the_receipt_payload_encodes_to_the_reviewed_canonical_bytes`,
`sealing_derives_the_reviewed_event_identities` — all green at HEAD, and I confirmed by
mutation that they are load-bearing rather than decorative.

### Verified by reading

I compared every formula in `tools/living-v2-vectors.py:86-153` field by field against the
normative block at `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md:321-358`.
All eleven transcribe correctly, including the two asymmetries a mirror of the Rust would
be just as likely to get right and a careless transcription would get wrong:
`creator_entity_digest` and `fork_branch_digest` take **no** `rules_u32`, while
`revision_id`, `autonomous_entity_digest`, `root_branch_digest` and `claim_rank` do.
Little-endian widths, the `0x00`/`0x01` optional tag, the `command_length_u64` prefix, and
the 16-byte identity truncation all match.

### Inferred, not directly verified

Stated separately because the task asks for this boundary explicitly.

- **That all 26 event kinds map to behaviours sections 3-8 require a boundary to report.**
  I read section 8 (`rules.md:252-268`) and section 10 in full, and judged the kind list
  against the variant doc comments in `receipt.rs:84-135` rather than against sections 3-7
  clause by clause. The list is coherent and I found no kind without an evident referent,
  but this is a judgement from the module's own descriptions, not a section-by-section
  audit.
- **That the absence of an archetype-editing operation is correct by design.** Section 8
  (`rules.md:254`) names "topology and archetypes" among creator-editable things and none of
  the 28 operations edits an archetype. My reading is that archetypes live in the validated
  built-in catalog pack, which section 10 says ships as one validated pack rather than
  remotely fetched content, so editing them is a different mechanism. That is an inference
  about intent; the spec does not say it.
- **That section 8's "distinguishes a direct creator grant from a resource-funded
  civilization order" is a preview or UI concern outside the authority schema.** `grep` for
  `grant`/`funded`/`funding` in `command.rs` returns nothing, so the distinction is not in
  the envelope. Whether it belongs there is a judgement I am not in a position to settle.
- **That the wasm build is unaffected.** Reasoned from the crate's dependency set
  (`serde`, `serde_json`, `sha2`, `thiserror`) having no platform surface. Not measured; see
  below.
- **That `LivingEventKindV2::as_wire_str` has no non-test consumer.** From `grep` across the
  crate, distinguishing the same-named methods in `catalog.rs` and `model.rs` by hand. A
  reflective or macro-generated call site would not have shown up.

### Could not check

- **The claim that the two new vectors' input JSON was "hand-written from the spec/module
  declaration, never dumped from the encoder."** Neither input document is committed. See
  finding F6 for what would make it checkable.
- **The wasm gate.** The plan's Task 4 checklist asks for it; the commit messages report
  fmt/clippy/test only, and I did not run `tools/build-web.sh`. The crate is pure
  `serde`/`sha2` with no platform surface, so I have no reason to suspect it, but I did not
  measure it.
- **`--workspace` scope caveat:** the repo gate I ran is the one `AGENTS.md` mandates and
  includes `--workspace`. The mutation runs used `-p nyon-workshop-core`, which is the
  correct narrower scope for that crate and equally avoids the `default-members = ["."]`
  trap.

---

## Mutation evidence — reproduced

Every mutation ran with `--no-fail-fast`, because the default harness stops at the first
failing binary and silently understates the blast radius. Failing test names were
extracted from the `failures:` blocks, not from a summary line.

| Mutation | Implementer reported | **Measured** | Failing tests |
| --- | ---: | ---: | --- |
| `LivingEntityKindV2::Fleet` ordinal `10 → 11` (`ids.rs:226`) | 3 | **4** | `every_published_entity_kind_ordinal_is_pinned_individually`, `the_entity_kind_registry_table_is_total_and_gap_free`, `the_reviewed_entity_vectors_carry_registry_kinds`, `the_creating_operations_cover_the_entity_kind_registry_exactly` |
| Corpus `ai_fleet_phase_8.entity_kind` left stale at `4` | 3 | **3** | `autonomous_entity_framing_including_a_shipment_matches_reviewed_vectors`, `every_autonomous_identity_field_changes_the_identity`, `the_reviewed_entity_vectors_carry_registry_kinds` |
| `LivingReceiptPayloadV2` fields 3/4 swapped (`receipt.rs:233-235`) | 2 | **3** | `the_receipt_payload_encodes_to_the_reviewed_canonical_bytes`, `sealing_derives_the_reviewed_event_identities`, `the_public_receipt_field_order_is_pinned_against_literal_bytes` |
| Zeroed event ids in `seal_living_tick_receipt_v2` (`receipt.rs:380`) | 4 | **5** | `sealing_derives_the_reviewed_event_identities`, `hashing_the_public_receipt_instead_of_the_payload_yields_a_different_digest`, `the_public_receipt_round_trips_through_the_canonical_wire`, `two_identical_events_differ_only_by_ordinal_and_still_receive_distinct_identities`, `the_public_receipt_field_order_is_pinned_against_literal_bytes` |
| Drop `declarer` from the reference walk (`command.rs:599`) | exactly 1 | **exactly 1** | `every_reference_bearing_field_is_validated` |
| `LivingTickReceiptV2` `state_digest`/`digest` swapped, at HEAD | 1 | **1** | `the_public_receipt_field_order_is_pinned_against_literal_bytes` |
| Same swap, with the pre-`78072b9` test files (`git checkout 7d51a5b -- …/tests/living_*.rs`) | green | **229 passed / 0 failed, exit 0** | — |

Three reported counts were **understated**, none overstated. That is the safe direction,
and in each case the extra failure was a genuinely additional guard. The `declarer` case —
the one shape the task singled out — reproduces exactly as reported: 16 of 17 command tests
stay green and the single per-field test is the only thing standing between that hole and a
silent regression. That is precisely why F1 below matters.

The last row is the commit that justifies `78072b9`: the public receipt's declared field
order really was pinned by nothing before it, and really is pinned by exactly one test now.

---

## Normative acts, and whether each was authorized

### Authorized: the entity-kind and phase registries

The pre-change spec text now at `rules.md:394-400` delegated entity-kind assignment to "the task that fixes
the authority entity set, numbered freely", and the same passage makes the published tables
normative. Publishing them here is squarely within that grant.

Verified by reading:

- The published entity-kind table (`rules.md:450-469`) and `LIVING_ENTITY_KIND_REGISTRY_V2`
  (`ids.rs:196-211`) and `LivingEntityKindV2::ordinal` (`ids.rs:214-231`) agree row for row,
  1..14, no gaps, no duplicates.
- The registry order tracks the state schema's own field order (`rules.md:368-382`), which
  is the rationale the spec text gives. Consistent.
- The exclusion argument is coherent: colony, relation, agreement, war, observation,
  settlement claim and occupation are keyed by the identities they relate, so neither entity
  formula can take them as a subject. `Hull` is included and is nested inside the fleet
  record in the state schema — that is not a contradiction, because nesting concerns storage
  while the registry concerns identity, and a hull is destroyed individually
  (`HullDestroyed`), so it needs one.
- **The `rules.md:355` prohibition is honoured.** `ordinal()` and `from_ordinal()` are explicit
  per-variant match arms; `grep -n 'as u16\|repr(' living/ids.rs` returns nothing across
  all 665 lines, so there is no discriminant cast and no `#[repr]` on either enum. `living_registry_serde_v2!`
  (`ids.rs:341-367`) serializes through `ordinal()` and deserializes through
  `from_ordinal()`, refusing an unpublished value rather than guessing.
- The phase registry's one-based numbering is forced by the pre-existing corpus labels
  (`ai_fleet_phase_8`, `route_shipment_phase_9`) and matches section 2's numbered step list.
  Correct.

Both registries carry per-row pins *and* a totality/gap-free test, which is the right pair:
the row test names what moved, the table test catches a skipped or repeated value that the
row test would accept.

### Partly authorized, and the seventh-gap question: `provenance`, the 26 event kinds, and the creator operation schema

The implementer flags `provenance` and `LivingEventKindV2` as its own decisions. My read is
that the *authorization* is thinner than for the registries but real, and that the *scope*
of what got frozen is larger than the report says. See F2.

- Spec `rules.md:404` says each `event_payloads` entry uses "the same provenance and kind
  encoding `LivingEventV2` carries but omitting `id`". That is delegation by reference to
  the implementation, not a published table. Thin, but it is authorization.
- Spec `rules.md:469` explicitly leaves `LivingEventKindV2`'s discriminant ordinal
  unassigned and states why no byte depends on it. So **adding** an event kind is not a
  format break, and the 26-kind list is genuinely low-stakes.
- The shapes chosen are faithful. `LivingEventProvenanceV2` (`receipt.rs:64-75`) is a
  `type`-first tagged enum per the canonical contract; `Creator` carries the causing
  revision, which is exactly what section 8's "produce Creator intervention events rather
  than pretending the civilizations negotiated" requires to survive replay; `Autonomous`
  carries a registry phase and correctly omits one for `Creator`, since creator
  interventions are applied in phase 1 by definition. Every one of the 26 kinds maps to a
  behaviour section 3–8 requires a boundary to report, including the easily-missed
  `RefundDiscarded` that section 3 requires be receipted rather than silently dropped.
- **What the report does not say is that the receipt vector freezes only two of those kind
  strings** (`creator_intervention`, `shipment_dispatched`) and one provenance shape of each
  variant. The other 24 kinds are frozen by nothing.
- **The larger freeze is the creator operation schema, and it is unmentioned.** Decoding
  `creator_command.command_bytes_hex` yields
  `{"type":"creator_batch","operations":[{"type":"create_system","local":0,"name":"Vale"},…,
  {"type":"create_lane","local":2,"system_a":{"type":"local","local":0},…,
  "distance_units":101}]}`. Those field names — `local`, `name`, `system_a`, `system_b`,
  `distance_units` — are `living/command.rs` decisions with no spec table behind them, and
  `revision_id` hashes those exact bytes. The 28-operation schema is therefore now frozen by
  vector. That is a bigger normative act than provenance and deserves the same treatment.

### Confirmed: the plan/spec conflict was resolved correctly

The plan's Task 4 checklist at `2026-09-04-nyon-living-galaxy-authority.md:159` asks for
"the frozen event kinds from the rules spec with explicit `u16` ordinals". The spec at
`rules.md:469` deliberately leaves that ordinal unassigned. **The implementer followed the
spec, and that is right** — the spec is the binding contract, the plan states targets, and
the plan's *own prose* at lines 56-62 already records the deliberate non-assignment. The
plan therefore contradicts itself, and line 159 is the stale half. See F4.

### Deferrals

**`LivingStepContextV2` → Task 5: sound, and half of its obligation is already discharged.**
The plan bullet is "centralize event and autonomous identity allocation in
`LivingStepContextV2`; subsystem code cannot fabricate IDs." The event half is delivered
structurally, not by convention: `LivingPendingEventsV2::record` (`receipt.rs:317-332`) is
the only place an ordinal is assigned, and `LivingPendingEventV2` (`receipt.rs:197-202`) has
no identity field to fill in correctly or otherwise, so the non-conformant
zeroed-placeholder implementation the spec names is unrepresentable rather than merely
discouraged. The autonomous-identity half genuinely needs a step context and correctly
waits. Nothing was frozen that Task 5 will need to renumber.

**The intent registry unimplemented in Rust: sound, with one caveat.** Nothing in this diff
hashes an intent ordinal, so there is no identity to get wrong yet. But see F7 — the act of
publishing the intent table makes a pre-existing corpus value visibly unpublished, and Task
5 must not resolve that by editing a vector.

---

## Repo rules enforced mechanically — each checked

- **Two-place module edit:** `CRATE_SOURCES` grew `14 → 16` with `living/command.rs` and
  `living/receipt.rs` under their `living/` prefixes (`living_wire.rs:693-720`), and the
  declaration-count assertion moved `13 → 15` with its message updated to "seven living
  submodules" (`living_wire.rs:869-870`). Both places, correct prefixes, message kept
  truthful. ✓
- **V1/V2 isolation guard:** green in the measured run; the two new files are inside
  `CRATE_SOURCES`, so they are scanned, doc comments included. ✓
- **No `unsafe` outside `lib.rs`:** green. ✓
- **`#[serde(deny_unknown_fields)]` on every serialized struct:** verified by reading all
  fourteen new serialized types (six in `receipt.rs`, eight in `command.rs`; the two
  `thiserror` error enums are not serialized). Twelve carry the attribute. The only two
  without it are `LivingEventKindV2` (`receipt.rs:84`) and `LivingCommandModeV2`
  (`command.rs:646`), both unit-variant enums where the attribute has nothing to deny. ✓
- **Three field-order pins added by `78072b9`:** `the_revision_field_order_…`,
  `the_accepted_envelope_field_order_…`, `the_public_receipt_field_order_…`, each decoding a
  declared-order literal and refusing a swapped one. The pattern is right — a round-trip
  cannot see a reorder because both sides move together. Their value is measured above. ✓

---

## Findings

### F1 — MEDIUM — The reference-walk guard covers 6 of 28 operations, and a mutation survives it

`crates/nyon-workshop-core/tests/living_command.rs:412-465`
(`every_reference_bearing_field_is_validated`), against
`crates/nyon-workshop-core/src/living/command.rs:437-621`.

The test constructs six operations (`SetWorldOwner`, `SetRelationBase`, `SetFleetOrder`,
`ForceWar`, `RemoveEntity`, `CreateHullJob`). It is a hand-enumerated list, not a total
scan, so its name overclaims by more than four times.

**Reproduced:** I dropped `lane` from the `CreateHazard` arm of the reference walk
(`command.rs:563`, `Self::CreateHazard { lane, .. } => push_local(lane, out)` →
`{ lane: _, .. } => {}`) and the crate suite stayed **233 passed / 0 failed, exit 0**. The
mutation survives completely.

This is the same shape as the `declarer` hole the implementer found, and it is worse than
the `declarer` case: `declarer` at least had a per-field assertion. Roughly twenty
reference-bearing fields have nothing — `CreateLane.system_a/system_b`, `CreateRoute`'s
four, `CreateShipment`'s four, `CreateFacility.world/owner`, `CreateHazard.lane`,
`SetDepositRemaining.deposit`, `SetCivilizationPolicy.civilization`,
`SetShipmentDisposition.shipment`, `CreateHull.fleet`, the `SetAgreement`/`RemoveAgreement`/
`ForcePeace` participants, and more.

The consequence if one is dropped is not cosmetic. Verified by reading
`command.rs:770-801`: `LivingCommandV2::validate` calls `local_references` once and derives
**both** `ForwardLocalReference` and `UnknownLocalReference` from what that single walk
returns, so an omission disables both structural checks for that operation, and spec
section 8 (`rules.md:254`) requires that "free editing does not allow illegal IDs". No
identity is affected — `revision_id` hashes the bytes either way — so this
is a validation-coverage defect, not a corpus problem.

**Suggestion:** make the guard total rather than enumerated. Build one batch containing every
operation variant, each with exactly one field pointing at a missing local, and assert every
one is refused; or derive/expose the field list so a variant added without a walk arm fails
to compile. Rename the test if it is going to stay a sample. Do not simply add
`CreateHazard` to the list — that recreates the same trap for whatever lands next.

**Response:** Fixed. `every_reference_bearing_field_is_validated` no longer enumerates:
`scan_command_schema` reads `living/command.rs`'s own text and returns every field of every
enum in the file whose declared type carries a `LivingCommandTargetV2` — **49**, not the
"roughly twenty" uncovered plus six covered, because the six previously covered were
`SetWorldOwner.owner`, `SetRelationBase.to`, `SetFleetOrder.lane_path`, `ForceWar.declarer`,
`RemoveListed.dependents` and `CreateHullJob.target_fleet`, leaving **43** with no assertion.
Each field is now redirected at an undeclared local in turn, from one template per operation
built as a typed Rust value, so a field added to a variant is a compile error before it is a
test failure, and a twenty-ninth operation fails the scan's coverage assertion by name. The
walk itself is unchanged: no digest, vector or field order moves. Two nits in the finding —
`CreateFacility` has no `owner` field, and the uncovered count was understated rather than
overstated.

**Verified by mutation:** the exact `CreateHazard.lane` drop this finding reproduced now fails
`every_reference_bearing_field_is_validated` naming `create_hazard.lane` (233 → 1 failed,
exit 101); so do `SetShipmentDisposition.shipment` (the renamed-binding or-arm),
`CreateRoute.source_owner` (one of four in an arm) and `CreateWorld.owner` (the `Option`
shape). Deleting one template fails both new assertions with "no template declares
create_hazard"; adding a reference field to a variant fails to compile. Gate at the fix:
`FMT_EXIT: 0`, `CLIPPY_EXIT: 0`, `TEST_EXIT: 0`, 638 passed / 46 suites (was 637 / 46).

**Status:** fixed.

### F2 — MEDIUM — A seventh normative gap: the creator operation schema is frozen by vector with no spec table

`crates/nyon-workshop-core/tests/fixtures/living-v2/vectors.json` (`creator_command.
command_bytes_hex`, `receipt_payload.event_payloads`), against
`crates/nyon-workshop-core/src/living/command.rs:111-627` and
`crates/nyon-workshop-core/src/living/receipt.rs:64-135`.

The implementer names `provenance` and the 26 event kinds as its own frozen decisions. I
agree those qualify, and I judge them **low severity**: spec `rules.md:404` delegates the
encoding by reference, `rules.md:469` makes adding a kind explicitly not a format break, and
only two kind strings are in a vector at all.

The one that should carry the "seventh gap" label is the **creator operation schema**. Its
28 operations and their field names are hashed into `revision_id` through
`command_bytes_hex`, they have no published table, and unlike an event kind, changing one is
a hard format break that invalidates a frozen vector. Checked against spec section 8
(`rules.md:254`): the 28 operations do cover every ability that paragraph enumerates —
topology, finite deposits, inventories, colonies/ownership, facilities and queues,
civilization policies and base relations, agreements, fleets/orders, routes, scheduled
hazards, plus explicit cascades. The coverage is faithful. It is the *authority* that is
missing, not the content.

Two smaller items in the same family, recorded so a later reader does not have to
re-derive them: section 8 also names "archetypes" among creator-editable things and there is
no archetype operation (archetypes live in the validated built-in pack, so this is probably
correct-by-design rather than a gap), and section 8's "distinguishes a direct creator grant
from a resource-funded civilization order" has no representation in the envelope — plausibly
a preview/UI concern outside the authority schema, but it is unstated either way.

`LivingTickReceiptV2`'s field order, pinned in `78072b9`, is a lower-stakes case worth
distinguishing explicitly: it is public wire format that no hash consumes, so a reorder is a
compatibility break rather than an identity break.

**Suggestion:** the fix is a spec publication, not a code change — publish the operation
schema (and, more cheaply, the provenance shape and the kind list) as a normative section 10
table the way the entity-kind table was just published, and record it as the discharge of a
seventh gap. Do not change any byte while doing it; the current shapes are the ones the
vectors freeze.

**Status:** open, owner decision.

### F3 — LOW — `every_event_kind_round_trips_through_its_wire_string` covers 4 of 26

`crates/nyon-workshop-core/src/living/receipt.rs:470-486`.

The test iterates a hand-picked four (`AgreementExpired`, `CreatorIntervention`,
`ShipmentWaiting`, `WarEnded`). `LivingEventKindV2::as_wire_str` (`receipt.rs:141-170`)
duplicates what `#[serde(rename_all = "snake_case")]` derives, so the two can drift for the
other 22 arms undetected. Verified by grep: `as_wire_str` on this type has **no non-test
consumer** anywhere in the crate (the hits in `catalog.rs` and `model.rs` are unrelated
same-named methods), so today the drift would be invisible rather than harmful.

**Suggestion:** iterate a `const ALL: [LivingEventKindV2; 26]` and assert both the
round trip and `as_wire_str` agreement for every arm; the array doubles as the
"adding a kind is a two-place edit" tripwire. Or delete `as_wire_str` if it is genuinely
unused. Either way the test's name should stop claiming "every".

**Status:** open.

### F4 — LOW — The plan contradicts itself and the spec on event-kind ordinals

`docs/superpowers/plans/2026-09-04-nyon-living-galaxy-authority.md:159` versus the same
file's lines 56-62 and `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md:469`.

Line 159 still asks for "the frozen event kinds from the rules spec with explicit `u16`
ordinals"; lines 56-62 of the same plan already record that the ordinal is deliberately
unassigned. The implementer followed the spec. Confirmed correct.

Separately, every Task 4 checklist box at lines 156-163 is still `- [ ]` while the status
table at line 32 records the task as **landed**.

**Suggestion:** strike the `u16` clause from line 159 with a pointer to lines 56-62, and
tick the boxes the task actually completed (leaving the `LivingStepContextV2` and wasm-gate
boxes honestly open).

**Status:** open.

### F5 — LOW — The plan's landed-status line records 633/46; the measured total is 637/46

`docs/superpowers/plans/2026-09-04-nyon-living-galaxy-authority.md:32`.

The line was written in `7d51a5b`, before `78072b9` added exactly four tests
(`the_revision_field_order_…`, `the_accepted_envelope_field_order_…`,
`the_public_receipt_field_order_…`, `one_event_past_the_ordinal_space_is_refused`). 633 + 4
= 637, which is what I measured. `78072b9`'s own commit message states "637 passed / 46
suites, from 633 / 46" and is correct; only the plan line is stale. The implementer's report
of 637/46 is accurate, and the 591/44 baseline is exact (see the arithmetic above).

**Suggestion:** update the number, or drop the count from the plan line, since any count
written into a plan goes stale on the next commit.

**Status:** open.

### F6 — LOW — The "hand-written, never dumped" claim is unverifiable from the artifacts

`crates/nyon-workshop-core/tests/fixtures/living-v2/vectors.json` (the `creator_command`
and `receipt_payload` notes), `tools/living-v2-vectors.py:351-380`.

The corpus notes cite `python3 tools/living-v2-vectors.py revision <catalog> <seed> none 7 3
<batch.json>` and `… receipt <payload.json>`, but neither `batch.json` nor `payload.json` is
committed. **I state plainly that I could not check whether those documents were hand-written
from the spec and module declaration or dumped from the Rust encoder.**

Nothing in the tree can distinguish the two. The Rust tests that pin the bytes
(`the_reviewed_batch_encodes_to_the_reviewed_command_bytes`,
`the_receipt_payload_encodes_to_the_reviewed_canonical_bytes`) prove agreement between the
document and the encoder, which is exactly what they should prove — and is exactly what a
dumped document would also satisfy vacuously. The claim is plausible: the field ordering it
uses (`{ordinal, provenance, kind}`, `type` first, four payload fields in spec order) is
recoverable from the spec plus the module's declared field order without running anything.
Plausible is not verified.

**Suggestion:** commit both input documents beside `vectors.json` (e.g.
`fixtures/living-v2/inputs/creator-batch.json`, `receipt-payload.json`) and have `verify`
read them. That gives the derivation a durable, reviewable source, makes the two subcommands
reproducible by a third party, and turns a provenance claim into an artifact. It costs two
small files.

**Status:** open.

### F7 — LOW — The corpus carries an `intent_ordinal` the newly published registry does not publish

`crates/nyon-workshop-core/tests/fixtures/living-v2/vectors.json`
(`autonomous_entities[0]`, label `ai_fleet_phase_8`), against
`docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md:429-447` and line 343.

Spec `rules.md:355` says `intent_ordinal_u16` comes from the published tables, with one written
exception: "route dispatch uses the route as actor and its stable per-boundary dispatch
ordinal". The intent registry published here runs 1..16, so **0 is not a published intent**.

Applying that row by row:

- `route_shipment_phase_9`, `intent_ordinal` 0 — phase 9 is `DispatchOrders`. Covered by the
  dispatch-ordinal exception. Fine, and this is what the task description was describing.
- `ai_fleet_phase_8_next_intent_ordinal`, `intent_ordinal` 1 — phase 8 is `GenerateIntents`,
  and 1 is a published intent. Fine.
- **`ai_fleet_phase_8`, `intent_ordinal` 0 — phase 8 is `GenerateIntents`, and 0 is neither a
  published intent nor a route dispatch.** Not covered by either clause.

This is pre-existing from Task 1 and this diff did not touch it; publishing the intent table
is what made it visible. It blocks nothing now, because no Rust code consumes an intent
ordinal yet.

**Suggestion:** flag it for Task 5. When a typed intent registry lands, the resolution must
not be to edit `vectors.json` — that is the re-derivation-from-implementation failure this
whole corpus exists to prevent. Either the spec sentence needs an owner reading that admits
a zero sentinel for a framing-only vector, or the row needs re-deriving outside the crate
with a published ordinal and the change recorded in the spec's renumbering note the way the
entity kinds just were.

**Status:** open, carry to Task 5.

### F8 — INFO — The corpus's `actor_id` is now a dangling identity, deliberately

`crates/nyon-workshop-core/tests/fixtures/living-v2/vectors.json`, all three
`autonomous_entities` rows.

`actor_id` is `a65a0bf3dbebc95828914b1698e6a81b`, which was the **pre-renumber**
`entity_id` of `kind_3_local_0`. After the renumber that creator row is
`d183a56465e646d852f39d7d60bfa4cd`, so the corpus's internal story no longer closes.

This is correct as it stands. `actor_id` is an opaque 16-byte framing input, changing it
would have invalidated three more digests, and leaving it is precisely what keeps the
"no other byte changed" property true and auditable. (The same is true of
`receipt_payload.state_digest`, which is the spec's published `STATE domain || {}` golden
rather than any real state.)

**Suggestion:** none, other than not "fixing" it later. A one-line note in the corpus's
`entity_kind_note` saying `actor_id` is a deliberately opaque input that intentionally no
longer matches any row would pre-empt a future well-meant correction.

**Status:** informational.

### F9 — INFO — Unknown-field rejection inside the internally tagged `provenance` is untested but correct

`crates/nyon-workshop-core/tests/living_receipt.rs:375-398`.

`receipt_records_reject_unknown_and_reordered_fields` exercises an unknown field at the
payload level, a reordered payload, and a tag-not-first provenance — but never an unknown
field *nested inside* `provenance`. Internally tagged enums are the one place serde's
`deny_unknown_fields` has documented caveats, so this was worth confirming rather than
assuming.

**Verified by running**, in a throwaway worktree since removed: decoding
`{"ordinal":0,"provenance":{"type":"autonomous","phase":3,"extra":1},"kind":"fleet_arrived"}`
returns `Err(Json)`, and plain `serde_json::from_value` also errors. The attribute works
here. There is also a second line of defence regardless: `decode_canonical_v2`'s
re-encode-and-compare would reject the extra field as `NonCanonical`.

**Suggestion:** add that literal to the existing loop. It is one line and it pins a
behaviour that depends on serde internals.

**Status:** informational.

---

## What is good, and worth preserving

Recorded because a review that lists only findings misrepresents this diff.

- **The re-derivation discipline was actually honoured**, and the tool that honours it is
  checked in and runnable by the next reviewer rather than described in a report. That is
  the single most important property of this task and it holds.
- **`verify` is a whole-corpus control, not a one-shot script.** That it reproduced 49 rows
  of the *previous* corpus is a stronger guarantee than the implementer claimed for it, and
  it will keep paying off at every future renumber.
- **The unrepresentable-illegal-state work in `receipt.rs`** is the right kind of answer to a
  normative rule. The spec says a zeroed placeholder identity is non-conformant;
  `LivingPendingEventV2` has no field to zero. That converts a rule into a type, and the
  zeroed-id mutation failing five tests confirms it is guarded on the outside too.
- **`78072b9` is a model corrective commit**: it identified three records whose declared
  field order was pinned by nothing, said so in the message, and I measured its exact claim
  (green before, exactly one failure after) and reproduced it.
- **Per-row *and* totality pins on both registries.** Row pins name what moved; the
  gap-free test catches a skip or duplicate the row pins would accept. Both were needed.

## Minor reporting note

The implementer's own control is described as reproducing "all 52 stored digests". I could
not reproduce that figure: `verify` prints 65 checks at HEAD, of which 49 are digests (the
other 16 being domain-byte, optional-tag, canonical-byte and winner rows), and 49 checks on
the pre-change corpus. No count in the artifacts is 52. The substantive claim — the control
reproduces the unmodified corpus — is verified and then some; only the number is off.
Informational, no action beyond not quoting it.

---

## Recommendation

Approve. Nothing here justifies holding the commits: no identity is wrong, no vector is
tainted, the gate is green at 637/46, and the freeze this task performs is the one it was
authorized to perform.

Before Task 5 starts, in priority order: close **F1** (it is a live coverage hole that a
surviving mutation demonstrates), get an owner reading on **F2** (because publishing the
operation schema after more work depends on it costs more than publishing it now), and
carry **F7** forward explicitly so it is not resolved by editing a vector.

---

Reviewed by Claude Opus 5, 2026-09-08. Gate re-run and mutation evidence produced during
this review; all review worktrees removed.
