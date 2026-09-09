# Review: `f569437` — feat(living): add the genesis manifest and validated genesis

Status: **APPROVE WITH FINDINGS**

Task: Living Galaxy authority 3b. Binding contract:
`docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md`.

Files (4, +1033/−3): `crates/nyon-workshop-core/src/living/genesis.rs` (new, 389),
`crates/nyon-workshop-core/tests/living_genesis.rs` (new, 630, 24 tests),
`crates/nyon-workshop-core/src/living/mod.rs` (+7),
`crates/nyon-workshop-core/tests/living_wire.rs` (+6/−3).

The commit was amended once; the reviewed SHA is `f569437`. `git diff 743dab9 f569437` is
**+6/−1 in the test file only**: it renames
`an_undeclared_facility_definition_is_a_dangling_reference_not_a_clock_defect` to
`…_reports_a_dangling_reference` and adds the doc comment recording the M3 measurement. The
amend is exactly the correction the implementer's report describes.

Nothing in this review was fixed. Two source mutations were applied as evidence and reverted;
`git status --porcelain` was empty before and after each, and is empty apart from this file.

---

## Verdict summary

The two prohibitions hold. `LivingGenesisGeneratorV2` is a two-field record that nothing
consults, and no generation code exists anywhere in the module. Validation **composes**
`LivingGalaxyStateV2::validate` (genesis.rs:202) rather than restating it: no capacity, range or
referential rule from that ~520-line method reappears here, and neither of the two things it
deliberately excludes — per-colony industrial slots and `construction_jobs`/`hull_jobs`
reservation accounting — was added. The digest is the state digest, computed from the state's
own canonical bytes, and `root_branch_id_v2` is reused rather than re-derived. The three new hub
constants match the spec and duplicate nothing. Both halves of the two-place guard edit are
present and correct, and the gate is green.

What holds it back from a clean APPROVE is not a code defect. It is that the specification
publishes **no wire schema for the genesis manifest at all**, so the envelope this commit
freezes is the implementer's invention (F1), and — measured, not assumed — its field order is
pinned by **no test in the crate** (F2). Together those are the reason Task 3c cannot yet author
vectors from spec text. F3–F7 are small and none of them is a byte-format error.

---

## What I verified by running

From the repo root, `--workspace` throughout, exit codes read out of log files rather than
through a pipe or a trailing `echo`.

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | `FMT_EXIT=0` |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | `CLIPPY_EXIT=0`, zero warnings |
| `cargo test --workspace --all-targets` | `EXIT_CODE=0`, **590 passed across 44 `test result: ok` lines**, 0 failed |

Arithmetic against the stated `60db100` baseline of 566/43: 566 + 24 = **590**, 43 + 1 = **44**.
The `living_genesis.rs` binary reports `running 24 tests` / `24 passed`. The delta is exactly
this commit's suite and nothing was dropped. The build itself was fully cached (`grep -c Compiling`
on the log is **0**), so that run alone is not evidence against a stale binary; what is evidence
is that the log carries 590 individual `... ok` lines across 44 result lines, so every test binary
executed, and that mutations M3 and M4 each forced a real recompile (`Compiling` present in both
logs) with M3 producing a different, failing result from the same harness.

The two mechanically enforced guards passed in that run and exercise the two-place edit for me:
`crate_module_scan_covers_every_file` (log line 810) and
`no_crate_source_file_names_both_a_workshop_v1_and_a_living_v2_identity` (line 811).

### Mutation experiments

| # | Mutation | Outcome |
| --- | --- | --- |
| M3 | Delete `document.state.validate(catalog)?;` (genesis.rs:202) | **Exactly 1 failure**, `a_state_the_authority_contract_refuses_is_not_valid_genesis`; 23 passed. `an_undeclared_facility_definition_reports_a_dangling_reference` stayed green. |
| M4 | Move `kind` from first to third position in `LivingGenesisManifestV2` (genesis.rs:142–151) | **0 failures.** `cargo test -p nyon-workshop-core` green across all 12 result lines, including the 24-test genesis suite. |

M3 **confirms the implementer's account exactly**, including the reason: the `ok_or` at
genesis.rs:307 raises a byte-identical `DanglingReference` when the fallback runs. The implementer
reported this weaker-than-expected result rather than hiding it, and reworded the affected test
to say what it actually pins. That is the right handling.

M4 is mine and is the evidence for F2.

---

## What I verified by reading

- **Provenance is provenance.** `LivingGenesisGeneratorV2` (genesis.rs:128–135) is
  `generator_id: LivingSlugV2` + `generator_version: u32`, `deny_unknown_fields`, and appears in
  no other type, no function signature, and no digest input. It is not a field of
  `LivingGenesisManifestV2`. This is **spec-backed rather than invented**: rules line 305 lists
  `genesis_generator` and `genesis_manifest` as two sibling fields of the *archive*, so
  provenance belongs beside the manifest, not inside it. `genesis_seed` likewise enters this
  module at exactly one place, genesis.rs:386, as a passthrough argument to `root_branch_id_v2`.
- **No generator is rerun, and none exists.** The module has no construction path at all; the
  only inputs are bytes or an already-built document.
- **Composition, not duplication.** genesis.rs adds precisely three things `validate` does not do:
  the envelope (`check_envelope`, :221), the tick-zero conditions (`check_genesis_boundary`,
  :246), and the clock rule (`check_clocks`, :280). `LIVING_COLONY_INDUSTRY_SLOTS_V2` and
  `LIVING_RESOURCE_STORAGE_LIMIT_V2` are neither imported nor redefined; slot counting and job
  reservation are absent, matching the exclusions model.rs:1203–1229 documents.
- **The digest hashes the state.** genesis.rs:210 is `document.state.digest()`, which is
  `state_digest_v2` over `LivingGalaxyStateV2::canonical_bytes` (model.rs:1181–1193) — the
  `NYON-LIVING-STATE-V2\0` domain over state bytes. That is rules line 343 literally.
  `root_branch_id(seed)` (genesis.rs:386) calls the published `root_branch_id_v2` from `ids.rs`
  and re-implements no part of the formula.
- **The catalog hash cannot be spoofed.** `ValidatedLivingGenesisV2::catalog_hash` is taken from
  the `ValidatedLivingCatalogPackV2` argument (genesis.rs:217), not from the document, so
  `root_branch_id` cannot pair a genesis with a catalog it was never checked under. This is a
  strength worth naming.
- **Hub constants.** Rules section 3, "Hub baseline": "Each hub produces 2 energy every 10 ticks
  and 1 ore every 100 ticks. Its explicit fallback fabricator may consume 4 energy + 4 ore for 1
  alloy every 100 ticks." 10/100/100 at genesis.rs:64/66/68 is correct. `grep` over
  `crates/nyon-workshop-core/src` and `src` finds no other declaration of any of the three;
  the hub is not in `assets/living/core-pack-v2.json`, so they cannot come from catalog data.
- **The two-place edit is complete.** `CRATE_SOURCES` 13 → 14 with the label
  `"living/genesis.rs"` carrying the required `living/` prefix, and the declaration assertion
  12 → 13 with its message updated to "five living submodules".
- **Guards clear by construction.** `genesis.rs` contains no occurrence of `unsafe`, and its only
  V1-vocabulary-shaped substrings are inside `LivingCatalogHashV2`, `LivingStateDigestV2`,
  `LivingBranchIdV2`, which `without_living_identifiers` (living_wire.rs:543) strips before
  scanning. Every serialized struct in the file carries `#[serde(deny_unknown_fields)]`.
- **`tests/living_model.rs` is untouched** — it is not in the diff — so the 200-key canonical
  field-order sequence is unchanged by construction, and the state encoding did not move.
- **The `generator` byte assertion is real.** living_genesis.rs asserts
  `!manifest_text.contains("generator")` over the fixture's canonical bytes. Adding a
  `genesis_generator: LivingGenesisGeneratorV2` field would emit `"generator_id"` into those
  bytes and fail it. See F6 for its limits.
- **`LivingGenesisErrorV2::Kind` carrying no `found` is not an inconsistency.**
  `LivingCatalogErrorV2` (catalog.rs:49–67) has the identical shape: bare `Kind`, `FormatVersion
  { found }`, `RulesVersion { found }`. The genesis error enum mirrors the established precedent
  variant for variant.

---

## Judgments the task asked for

### Materialization as the identity projection — permitted, not forced, and the right call

The implementer argues the reading is forced: a manifest needing an expansion algorithm "would
make that algorithm part of replay truth, which is exactly what 'generators are never rerun when
decoding' forbids". I do not think that is forced. Line 305 forbids rerunning *generators*; a
fixed, published, generator-free expansion (say, a compact topology declaration expanded by a
normative algorithm in the spec) would not violate that sentence on its face — it would be part
of the decoder, not a rerun generator. So the argument is about consequences, not a quotation,
and it is presented in the module header as slightly stronger than it is.

But it is nonetheless the correct choice, for the reason the implementer gives second: identity
is the minimal reading, and it is the only one with no unpublished algorithm standing behind it.
Any expansion would have to be authored in the spec before it could be implemented, and none is.

One consequence for the gap proposal: under this reading the word "materialized" at rules line
343 is doing no work. It should either be changed to "declared by" or be given a definition that
says materialization is the identity, so a future reader cannot revive the expansion reading.

### The clock generalization — sound, and it reproduces the named cases exactly

Section 2 states the general rule first ("Genesis at boundary G initializes each clock to G plus
its period") and then illustrates with three slugs. Checked against
`assets/living/core-pack-v2.json`: `solar_array` `period_ticks: 10`, `extractor` `10`, `foundry`
`20`. The general rule therefore reproduces "solar and extractor at G+10; foundry at G+20"
exactly, and hard-coding the slugs would have added nothing but a divergence risk if a catalog
ever changed a period. The generalization is correct.

`expected: None` for a non-recurring effect is an **inference** and is labelled as one in the
code. It rests on section 2's "Due *recipes* use stored next-due boundaries": `shipyard`
(`hull_assembly`) and `defense_battery` (`defense`) run no recipe, so no clause initializes a
clock for them. I agree with the inference. It is also the only reading that does not invent a
period the catalog does not supply.

The implementer correctly kept section 2's "unless a validated import supplies canonical clocks"
exception out of genesis: that clause's subject is creator-created already-built facilities, not
the genesis boundary.

### `accepted_sequence == 0` and `branch_sequence == 0` — the inference holds

`design.md:148` — "The root branch begins at that manifest, before creator revision zero" —
gives `accepted_sequence == 0`: nothing has been accepted. Rules line 345 — "The root branch does
not consume the fork sequence" — gives `branch_sequence == 0`. Section 2 closes it by making both
in-state values **historical**, frozen at the moment the state was produced, so neither can carry
a live allocator mark forward into a tick-zero state. Labelling this inference rather than
quotation, and naming in the doc comment which check moves if a later ruling admits a resumed
sequence, is the right treatment.

### Keeping the M3 fallback — right, with a wording problem (F4)

The alternative to `ok_or` at genesis.rs:307 is an `expect`/panic on a path a caller controls,
which this crate does not do anywhere else. Keeping it is correct. See F4 for the comment.

---

## Findings

### F1 — MAJOR (specification gap, not a code defect; blocks Task 3c)
**`crates/nyon-workshop-core/src/living/genesis.rs:142–151`; rules spec §10, line 305.**

The spec publishes the top-level field order of the catalog pack and of the archive, and (since
2026-09-08) the authoritative state schema. It publishes **nothing** for `genesis_manifest`. I
confirmed this independently: `genesis_manifest` appears in the spec only as an archive field
name and in the `genesis_manifest_digest_32` sentence. There is no field list, no `kind` string,
no size bound.

So `kind="NYON_LIVING_GALAXY_GENESIS"`, `format_version`, `rules_version`, `state` is the
implementer's envelope. **Assessment: as a standalone-document reading it is faithful and
minimal** — it copies the pack's leading three fields verbatim, adds exactly one payload field,
invents no optionality, and the module header says plainly that it is the implementation's and
needs promoting. That is the correct disposition for an implementer who cannot edit the contract.

Three parts of it are nonetheless **owner decisions, not implementer decisions**, and the gap
proposal should put them to the owner rather than ratify the code:

1. **Standalone document or nested record?** The pack is *never* embedded in an archive (line
   305: "The archive references its catalog hash and never embeds a pack"), but the manifest
   **is** an archive field, sitting beside the archive's own `kind`, `format_version` and
   `rules_version`. Mirroring the pack envelope therefore puts a second `kind`/`format_version`/
   `rules_version` triple inside every archive. That may be wanted — a manifest handed around on
   its own is self-describing — but it is a choice with a cost, and the spec should state it.
2. **The unstated cross-check.** If both levels carry `rules_version`, must they agree, and what
   error fires when they do not? Nothing in the spec or this commit answers that. A Living V2
   archive decoder (not yet written) will have to decide, and deciding it there rather than in
   the spec is how two implementations drift.
3. **No published size bound.** genesis.rs:176 and :207 bound the manifest by
   `LIVING_MAX_ARCHIVE_BYTES_V2` (32 MiB), with the reasoning that a curated starter is
   legitimately large. Reasonable, but it means a manifest at the bound can never fit inside any
   archive, since the archive that must contain it shares the same 32 MiB bound. The manifest
   needs its own stated bound.

**Effect on Task 3c, which is the point.** 3c must derive frozen vectors from spec text by an
independent non-Rust computation. For the genesis manifest there is no text to derive from, so
vectors authored now would be derived from this module's field order — i.e. a recording of the
implementation, exactly what §10's re-derivation paragraph forbids ("a corpus regenerated from
the implementation's own output is a recording of the code rather than evidence about it"). 3c
is blocked on a normative edit, not on effort. Recommend: publish a `Genesis manifest schema`
subsection in §10 with the field order, the `kind` literal, the size bound, and a ruling on
nesting, then unblock 3c.

### F2 — MEDIUM (measured coverage hole)
**`crates/nyon-workshop-core/src/living/genesis.rs:139` ("The four fields are the wire order.
Reordering one is a format break.")**

That statement is true and load-bearing: `decode_canonical_v2` (wire.rs:108–111) re-encodes and
compares byte-for-byte, so serde declaration order *is* the wire order and any reorder rejects
every previously written manifest. **It is pinned by nothing.** Mutation M4 moved `kind` from
first to third and the entire crate suite stayed green — all 12 result lines, including all 24
genesis tests.

The existing tests cannot catch it by design: the round-trip encodes and decodes with the same
mutated struct, the unknown-field test injects at the front regardless of order, and the
`generator` assertion is order-independent. This is the same class of hole F1 of the
2026-09-08 `living_model` review recorded for record-level state field order.

The proper fix is the 3c frozen vectors, which are blocked by F1. Until then, a cheap interim
pin costs four lines: assert the canonical bytes of the fixture manifest start with
`{"kind":"NYON_LIVING_GALAXY_GENESIS","format_version":2,"rules_version":2,"state":{`. Not this
commit's obligation to have written, but it should not stay unpinned indefinitely, and the
interim assertion should be removed when the vectors land rather than left as a second authority.

### F3 — LOW (real gap, correctly not fixed here; belongs to `model.rs`)
**`crates/nyon-workshop-core/src/living/model.rs:464–473` and `:485–490`; fixture at
`tests/living_genesis.rs` `facility()`.**

The implementer's second finding is real. `LivingFacilityStatusV2` has only `Operational` and
`Blocked`, documented at model.rs:485 as "Outcome of the most recent due attempt", and at genesis
there has been no attempt. `Operational` is the right stand-in — it is the only value that does
not assert a false negative outcome, `Blocked` would additionally require a `blocked_reasons` set
that section 3 does not supply, and inventing a third variant here would be a schema change
inside a task scoped to genesis. But the label is a lie the type forces, and it will be read as
evidence by any consumer that switches on status. Not fixing it was correct; recording it is
correct; it needs an owner ruling (either a `Fresh`/`Idle` variant, or an explicit sentence that
`Operational` means "not blocked" rather than "ran").

**A second instance of the same asymmetry, which the report does not name.** `check_fleets`
(model.rs:1503–1508) rejects `hull.hit_points == 0`, but `check_colonies_and_industry`
(model.rs:1432–1437) checks a facility's `definition` and nothing else — no `hit_points` range at
all. That is why the fixture's `hit_points: 0` on every facility validates. For a solar array it
is harmless (model.rs:489: "only a defense facility can be damaged"), but the same path accepts a
genesis `defense_battery` at 0 HP — a destroyed battery declared as starting content — and
equally at 4 billion. Genesis checks no HP either, correctly, since section 3 states no genesis HP
rule. Same home as the status gap, same disposition: a `model.rs` item for the gap proposal, not
a change to this commit.

### F4 — LOW (comment contradicts the code it annotates)
**`crates/nyon-workshop-core/src/living/genesis.rs:306` — "`validate` already proved this
definition resolves."**

If it is proved, the `ok_or` branch below is dead. If it is not, the branch does work — and M3
demonstrated that it does exactly that when the composed call is absent. Keeping the fallback is
right; the comment should say it is deliberately redundant defence in depth, and that it is
reached only if `build`'s ordering is broken, rather than asserting a proof that would make it
removable. The test doc comment added by the amend already says this correctly; the source
comment is the one out of step.

### F5 — LOW (doc claim slightly stronger than the code)
**`crates/nyon-workshop-core/src/living/genesis.rs:183–184** — "it never bypasses a check that
[`decode_living_genesis_manifest_v2`] performs, it only skips the decode".

`validate_living_genesis_manifest_v2` skips `decode_canonical_v2` entirely, so it also skips the
BOM check, the UTF-8 check, `scan_canonical_structure`'s depth bound (wire.rs:139–146), and the
re-encode identity comparison. In practice none is reachable for a well-typed in-memory value —
they are all properties of *bytes*, and the schema bounds nesting depth — and the size bound is
still applied via `encode_canonical_v2`. So the behaviour is fine and the wording is what needs a
qualifier: "skips the byte-level checks, which a typed value cannot violate".

### F6 — NIT (a proxy presented as a guarantee)
**`crates/nyon-workshop-core/tests/living_genesis.rs`, `generator_provenance_is_an_identifier_and_a_version`.**

`assert!(!manifest_text.contains("generator"))` is a substring scan over one fixture's bytes. It
would spuriously fail if a future fixture named a world "Generator", and it would pass if a
provenance field were added under a different name. It is good evidence and it does fire for the
obvious regression, but it is a proxy for a type-level property, not the property. The
neighbouring assertion that `manifest_digest != state_digest_v2(canonical_bytes())` is the
stronger guarantee and carries most of the weight.

### F7 — NIT (unreachable generality)
**`crates/nyon-workshop-core/src/living/genesis.rs:284, 291–301, 314`.**

`boundary` is `state.tick.0`, and `check_genesis_boundary` (:198) has already forced it to `0`
before `check_clocks` runs (:203), so every `boundary + period` is provably `0 + period`. The
`G + period` generality is dead in its general form and the additions cannot overflow. Harmless
and arguably good documentation of the rule's shape, but worth a word so a later reader does not
assume the function is reusable at a non-zero boundary — it would need its own bound check first.

---

## What I could not check

- **The `clippy::err_expect` claim.** The implementer reports clippy caught three `.err().expect()`
  calls while the suite was green. `git grep` over the pre-amend commit `743dab9` finds **no**
  `.err()` in either new file, and the amend diff is the test rename only, so the fix predates
  the first commit and left no trace in git. I can confirm only the end state: clippy with
  `--all-targets --all-features -D warnings` exits 0 with zero warnings, and no `.err().expect()`
  survives. The claim is consistent with the evidence but is not independently verified.
- **Whether the invented envelope matches any intent the owner has not written down.** F1 is
  posed as a question, not resolved.
- **Behaviour of a Living V2 archive containing this manifest.** No such decoder exists yet, so
  the nesting concern in F1.1/F1.2 is prospective rather than observed.
- **Vector-level byte agreement.** By design: no frozen vectors exist for genesis, and the test
  file deliberately writes down no expected hash. Every digest assertion in the suite is a
  structural identity between two crate-derived values, so the suite is honest about proving
  self-consistency rather than spec conformance — but that also means **nothing in this commit is
  evidence that these bytes are the bytes the spec intends**. That is Task 3c's job, and F1 is
  why it cannot start.
