# NYON Living Galaxy: three normative gaps to resolve before A3/A4 vector freeze

Date: 2026-09-06

Status: **APPLIED 2026-09-08 in `438551e`.** All three gaps below were resolved as
spec text on the owner's decision; the rules spec's own amendment block (lines 6-12)
names this file as the source and records what was adopted. This file is therefore a
historical record of the analysis, not an open proposal.

**Two resolutions were adopted in a different form than proposed here, and the spec
wins on both:** phases are numbered **from 1, not 0**, and the `LivingEventKindV2`
ordinal is **deliberately left unassigned**. Read the spec, not this file, for the
normative form of any resolution below.

A fourth gap, not raised here, was found later the same day while preparing the state
schema and closed in `7200e8a`: the spec named `LivingGalaxyStateV2` without ever
listing its fields.

Original status, retained for provenance: "Proposal only. Nothing in this file has been
applied to the rules spec or the authority plan. Every 'smallest resolution' below is a
candidate formulation an implementer or reviewer should adopt, amend, or reject before
`docs/superpowers/plans/2026-09-04-nyon-living-galaxy-authority.md` Tasks 3 and 4 freeze
their canonical vectors."

## Inputs and how they were used

- `~/Documents/Codex/2026-09-04/verify-donaldfilimon-gama-pr-79-at/outputs/nyon-authority-contract-preflight.md`
  ("the preflight report"), read in full (57 lines). Highest-authority input:
  it names the three omissions this file resolves. Its own line-number
  citations were re-verified against the current checkout below and all of
  them still land on the sentence the preflight quotes, confirming the file
  contents did not change when the repository moved from `~/Public/NYON` to
  `~/dev/active/NYON` on 2026-09-05.
- `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` ("the rules
  spec"), read in full (332 lines) and re-searched with `grep -n` for
  `receipt`, `high-water`/`allocator`, `batch`, and `ordinal`/`u16` to confirm
  no existing text resolves any of the three gaps elsewhere in the document.
- `docs/superpowers/plans/2026-09-04-nyon-living-galaxy-authority.md` ("the
  authority plan"), read in full (223 lines).
- For grounding only, not as an input this file resolves anything against:
  `docs/superpowers/plans/2026-09-04-nyon-living-galaxy-civilizations.md`
  lines 60-110, which show the `LivingTickReceiptV2`/`LivingEventV2`/
  `LivingStepContextV2` shapes and the frozen `LivingGalaxyStateV2` field
  order that the rules spec's formulas and the authority plan's Tasks 3-4
  will produce. It is a downstream consumer plan, not a source of authority,
  and I did not check whether it needs edits.

**Independent verification result: all three omissions the preflight report
names are still open in the current rules spec and authority plan text.** A
targeted re-search found no sentence anywhere in either document that already
supplies a receipt payload exclusion rule, a historical/allocator counter
distinction, or a creator-batch/queue limit and complete ordinal registry
assignment. Where the preflight report's own wording and my independent
reading diverge, that is called out explicitly in each section below.

---

## Gap 1 — the receipt-hashing formula names its input but never defines it, and the natural reading is circular

### What the rules spec states

Rules spec, line 291 (part of the "normative formulas" block, section 10):

> ```
> receipt_digest = SHA256("NYON-LIVING-RECEIPT-V2\0" || canonical_receipt_bytes)
> ```

Rules spec, lines 312-313, in the same block:

> ```
> event_digest = SHA256("NYON-LIVING-EVENT-V2\0" || receipt_digest_32 || event_ordinal_u16)
> event_id = first 16 bytes of event_digest
> ```

The rules spec never defines what `canonical_receipt_bytes` is a canonical
encoding *of*. Every other hash formula in the same block names or clearly
implies its source record (`canonical_catalog_bytes`, `canonical_state_bytes`,
`canonical_payload_bytes` for archive integrity, `canonical_command_bytes` for
revisions). Receipts have no equivalent phrase, and section 10 has no prose
paragraph describing a distinct "receipt payload" record the way it has one
for archives.

Compare rules spec line 281, which *does* state an explicit hash-input
exclusion for a structurally similar problem:

> Integrity hashes the same ordered payload without the final integrity
> field.

That sentence is the rules spec's own precedent for "hash this canonical
record, but not the field that stores the hash of it." No equivalent sentence
exists anywhere in the rules spec for `receipt_digest`.

### What it fails to say

The only public receipt type visible from the authority/civilization plans
(`LivingTickReceiptV2`, civilizations plan lines 81-88) has fields `tick`,
`applied_revisions`, `events: Vec<LivingEventV2>`, `state_digest`, and
`receipt_digest`; each `LivingEventV2` (lines 89-93) has fields `id`,
`ordinal`, `provenance`, `kind`. Both `receipt_digest` on the receipt and
`id` on every one of its events are themselves *outputs* of the two formulas
above, which both take `receipt_digest` (directly, or via `receipt_digest_32`
for events) as an *input*. If `canonical_receipt_bytes` is read as "the
canonical encoding of the full `LivingTickReceiptV2` record," computing
`receipt_digest` requires already knowing `receipt_digest` (to fill the
receipt's own field and to compute every contained event's `id`) before it
has been computed. The rules spec states no exclusion that breaks this cycle,
so as written, the formula cannot be executed by a literal reading of the
record it names.

### Ambiguity vs. bug

This is a specification gap, not an observed implementation failure — no
code implementing `receipt_digest` exists yet to fail. The preflight report
frames it the same way ("The full-record interpretation is circular; a
separately defined payload could avoid it... this is a decision required
before implementation, not proof every implementation must cycle"), and my
independent reading agrees: the rules spec is silent, not self-contradictory,
because it never actually says the hash input is the full public record.

### Proposed resolution

Add a new normative subsection to rules spec section 10, adjacent to the
formulas block, defining a receipt payload record distinct from the public
`LivingTickReceiptV2`:

> `canonical_receipt_bytes` is the canonical encoding of a `LivingReceiptPayloadV2`
> record, in this field order: `tick_u64`, `applied_revisions` (the sorted
> array of 32-byte revision IDs), `event_payloads` (an array, in emission
> order, of `{ordinal_u16, provenance, kind}` — the same provenance/kind
> encoding `LivingEventV2` uses, but omitting `id`), and `state_digest_32`.
> `LivingReceiptPayloadV2` excludes `receipt_digest` and every event's `id`
> field: both are derived from this record's hash and cannot be inputs to it.

And a companion sentence pinning derivation order, matching the style of the
existing `entity_digest` note at line 319:

> Compute `receipt_digest` from `canonical_receipt_bytes` first. Derive each
> event's `id` from `event_digest` (which already consumes `receipt_digest_32`
> and that event's `event_ordinal_u16`, per the existing formula) second.
> Only then assemble and publish the full `LivingTickReceiptV2`, whose
> `events` field carries the now-derived `id` on every entry and whose own
> `receipt_digest` field carries the value computed in the first step. Pending
> (not-yet-committed) events inside `LivingStepContextV2::emit` carry no `id`
> and no placeholder value until this derivation runs; subsystem code must not
> invent one.

Concretely: freeze one additional exact vector alongside the existing
"receipt/event" vector the rules spec already commits to checking in (line
325 — "creator/autonomous entity..., root/fork branch, receipt/event, claim,
and archive vectors"), showing the payload bytes, the digest, and the final
event IDs together, so an implementer can byte-match all three without
inferring the exclusion independently.

### Owning task and why

**A4** (Task 4, "Freeze creator commands, revisions, receipts, and events,"
authority plan lines 98-109). Task 4's own checklist already names the exact
objects this gap concerns: line 105 ("Implement `LivingAcceptedCommandV2`,
`LivingRevisionV2`, `LivingTickReceiptV2`, provenance, and the frozen event
kinds from the rules spec with explicit `u16` ordinals") and line 106
("Centralize event and autonomous identity allocation in
`LivingStepContextV2`; subsystem code cannot fabricate IDs"). Task 3 (state
schema/genesis) never touches receipts or events at all — its file list and
bullets (lines 85-96) are scoped to `LivingGalaxyStateV2` and genesis. This
gap must close inside Task 4's own commit ("`feat(living): freeze V2
commands and receipts`," line 109), before civilization tasks are handed a
"frozen" receipt/event contract per the Cross-Plan Dependency Order (lines
201-208: Authority Tasks 1-4 precede Civilization Tasks 1-10).

### What breaks if frozen unresolved

If Task 4 freezes vectors by serializing the literal `LivingTickReceiptV2`/
`LivingEventV2` structs (the only structs currently named in either document)
as `canonical_receipt_bytes`, the hash cannot be computed without first
choosing an undocumented workaround — e.g., zero-filling `receipt_digest` and
every event `id` before hashing, or hashing in two undocumented passes. Two
implementations (or the two halves of the authority/civilization split, or
native vs. wasm) that each pick a different undocumented workaround will
produce different `receipt_digest` and `event_id` values from the same
logical tick, which breaks: the shared exact-vector corpus every
implementation is required to consume identically (rules line 325); the
"Legacy"/"Replay" fixtures' implicit assumption of one canonical outcome per
tick (rules line 259, "Uninterrupted 36000 boundaries equals save/reload at
18000 then continue"); and `event_id`, which downstream state (observations,
diplomacy event citations, the chronicle at rules line 214) can reference and
persist — a wrong or non-reproducible `event_id` corrupts every historical
reference to that event on reload.

---

## Gap 2 — historical per-branch counters and the durable non-rewinding allocator are never distinguished

### What the rules spec states

Rules spec, line 40 (section 2, tick and command contract):

> An accepted running envelope receives the next document-global
> monotonically increasing `accepted_sequence` and is validated against a
> private projection containing all earlier accepted envelopes for that
> boundary. The counter is persisted and never reused after queue clearing, a
> fault, Undo, or branching; a rejected submission consumes no sequence... If
> any supposedly accepted envelope cannot reproduce its validated result or an
> invariant fails, commit neither the tick nor any of its queued revisions,
> retain the queue for diagnosis, and pause with a typed fault.

Rules spec, line 42:

> `ordinal_u64` is that envelope's document-global `accepted_sequence`. A
> paused envelope also receives the next sequence. These meanings do not
> change after replay or branch creation.

Rules spec, line 321 (section 10):

> `branch_ordinal_u64` is allocated from a separate document-global
> monotonically increasing `branch_sequence` only when a fork command
> validates and is accepted. Rejected or stale forks consume no ordinal.
> Accepted ordinals remain reserved and are never reused after queue
> clearing, a fault, Undo, later branch navigation, or archival.

Authority plan, line 21 (Global Constraints), restates the same two counters
as one property: "Accepted and branch sequences are document-global,
monotonic, persisted, and never reused."

The civilizations plan (grounding only, not resolved here), line 97, places
`accepted_sequence` and `branch_sequence` as canonical fields of
`LivingGalaxyStateV2` itself, directly after `tick` in the frozen wire order
— the same struct rules spec line 289 hashes into `state_digest`, and the
same struct authority Task 6 (line 129) must restore byte-for-byte on Undo/
Redo ("redo restores its exact digest; historical edits preserve both
futures").

### What it fails to say

The rules spec repeatedly calls these counters "document-global" and "never
reused," which is a statement about a single, currently-live allocator. But
the same two field names also live *inside* `LivingGalaxyStateV2`, which is a
per-revision, historically-frozen snapshot that Task 6 must reproduce
byte-for-byte when navigating back to an earlier point in history — including
after other, later document-global allocations happened on a sibling branch.
The rules spec never states which of two incompatible things
`accepted_sequence`/`branch_sequence`, as they appear inside a given
historical `LivingGalaxyStateV2`, actually are:

1. The value that was live at the moment that historical state was produced
   (needed so an old branch's `state_digest` stays exactly reproducible no
   matter what happens later on a sibling branch), or
2. The current document-wide high-water mark (needed so the *next* command,
   submitted after navigating back to that old branch, allocates a genuinely
   unused sequence/ordinal and never collides with one already spent on a
   sibling).

A single field cannot be both without an explicit rule connecting them,
because (1) must stay frozen when other branches advance, while (2) must
keep advancing when other branches advance. The rules spec's "never reused"
language is exactly the kind of statement that requires (2) to exist
*somewhere*, but never says where, nor states that (1) and (2) are different
things at all.

The preflight report gives a concrete case that exercises exactly this:
record branch A at a cursor/tick/digest, accept later revisions on sibling
B, return to A's recorded view, then submit again. My independent reading of
rules lines 40, 42, and 321 confirms the rules spec gives no rule that would
let an implementer answer that case today: nothing says A's redisplayed
`state_digest` must stay frozen at its original counters while the fresh
submission on A still receives a document-wide-unique sequence higher than
anything used on B.

A secondary point the preflight report raises, and which my read of line 40
independently confirms: a retained-but-faulted queue ("retain the queue for
diagnosis, and pause with a typed fault") has already consumed
`accepted_sequence` values that never became a committed revision. Any
resolution that tries to recompute the high-water mark purely by scanning
the committed revision graph would under-count and could then reissue one of
those already-spent, never-committed sequence numbers.

### Proposed resolution

State explicitly, next to rules spec lines 40-42 and 321, that
`accepted_sequence` and `branch_sequence` play two distinct roles that must
not be collapsed into one persisted value:

> The `accepted_sequence` and `branch_sequence` values stored inside a given
> `LivingGalaxyStateV2` (and therefore hashed into that state's
> `state_digest`) are **historical**: they are frozen at whatever the
> document-global counters read at the moment that revision/branch was
> produced, and they never change afterward — including when other branches
> later advance the live counters. Restoring, viewing, or replaying an older
> revision or branch must reproduce these frozen values exactly and must
> never mutate them to reflect subsequent document-wide allocations.
>
> Separately, the engine maintains one pair of durable, monotonically
> non-decreasing **allocator high-water marks** for `accepted_sequence` and
> `branch_sequence`. These are never part of any single `LivingGalaxyStateV2`
> and are never inputs to `state_digest`. Every new submission or fork,
> regardless of which branch or historical tick is currently being viewed,
> allocates its next value from these document-wide marks, never from the
> counter fields of the currently displayed historical state. The marks
> persist across save/reopen at least as high as the maximum
> `accepted_sequence`/`branch_sequence` recorded anywhere in the full
> revision/branch graph, *and* at least as high as any value already spent on
> a retained-but-uncommitted queue after a fault (rules line 40), even though
> that queue produced no committed revision. Implementations must therefore
> persist the high-water marks independently of the committed revision graph
> — deriving them solely by scanning committed revisions on reload
> undercounts a fault-retained queue's already-spent values and risks
> reissuing them.

And, to keep this from silently widening an already-normative frozen list,
an explicit instruction on where the marks live in the archive format:

> The durable allocator high-water marks are carried as two additional
> top-level archive fields, ordered immediately after `final_digest` and
> before `integrity_sha256` in the list at rules line 281
> (`accepted_sequence_high_water`, `branch_sequence_high_water`, both
> `u64`), and participate in `archive_integrity` like every other listed
> field. They are not part of `canonical_state_bytes` for any individual
> state and never contribute to any `state_digest`.

### Owning task and why

**A3** (Task 3, "Freeze the complete state schema and validated genesis,"
authority plan lines 85-96). Task 3's own scope is exactly "what fields does
`LivingGalaxyStateV2` contain and what do they mean" (line 90: "Implement
`LivingGalaxyStateV2` with ordered systems... and counters") and "what gets
validated/how is it derived" (line 92). Whether `accepted_sequence`/
`branch_sequence` are historical-snapshot values or live allocator state is a
question about the *meaning of a state-schema field*, not about a command,
revision, or event — Task 4 owns none of those two field names. This must
also resolve before Task 3's own listed commit
("`feat(living): validate V2 genesis and state`," line 96), because Task 5
(queue orchestration, which allocates `accepted_sequence`) and Task 6
(branch/history/archive, which allocates `branch_sequence` and must
reproduce historical digests exactly) both consume Task 3's frozen schema
per the Cross-Plan Dependency Order — Task 3 sits upstream of both in the
plan's own task list, and the archive-field change above also needs to land
before Task 6's archive encoding (line 132) references a fixed top-level
field list.

### What breaks if frozen unresolved

Absent this ruling, an implementer has to guess, and the two plausible
guesses each break a different named fixture. Treating the in-state fields
as live-allocator values (no historical freeze) breaks Task 6's own required
test at line 129 ("redo restores its exact digest") the moment any sibling
branch has advanced in between, because the redisplayed state's counters
would reflect activity that happened elsewhere, silently changing
`state_digest` for state that is supposed to be immutable history. Treating
the in-state fields as the *only* allocator (no separate high-water mark)
breaks the explicit "never reused" requirement at rules lines 40 and 321
directly, because resuming work on an old branch would resume allocating
from that branch's old counter value, and can collide with sequence/ordinal
numbers already spent on a sibling branch — the exact case the preflight
report's concrete scenario describes. Either failure mode is silent (no
crash, just wrong digests or duplicate identities) and only surfaces later,
in save/reload parity (rules line 259) or in a multi-branch fixture that does
not exist yet — i.e., after the vectors are already frozen and expensive to
change.

---

## Gap 3 — A4 is told to "freeze" an event/ordinal registry and batch/queue limits that no cited document actually supplies

### What the rules spec and authority plan state

Authority plan, line 105 (Task 4): "the frozen event kinds from the rules
spec with explicit `u16` ordinals" — phrased as consumption of a table that
already exists in the rules spec.

Rules spec, line 319: "`phase_u16`, `intent_ordinal_u16`, entity kinds, and
local IDs come from explicit never-reordered schema tables, not Rust enum
layout." This states that such tables must exist and must be stable, but is
itself prose about a table, not the table.

Rules spec, line 277 (canonical encoding): "the schema-declared field order
is normative... use the exact lowercase-snake-case discriminants declared by
the schema" — again referring to a schema that must declare these things,
without itself declaring the full enumeration anywhere in the document.

Rules spec section 2 (lines 46-59) lists the ten ordered simulation phases in
prose, and section 9's `LivingEventKindV2`-relevant fixtures (e.g., line 246
"Simultaneous battle," line 252 "Captured freight") describe behavior in
prose, but at no point does the rules spec publish a numbered table
assigning explicit `u16` values to phases, event kinds, intents, or entity
kinds.

Independent re-grep of both documents for `batch` and for entity/queue
capacity language confirms: rules spec section 1's limits table (lines
19-28) and section 10's byte/nesting/work-unit caps (line 329, "V2 archive
input is capped at 32 MiB, a catalog at 2 MiB, nesting at 32 levels, and all
entity/queue/history counts at the declared limits... Bound a decode/replay
poll to 1024 work units") name every entity-collection maximum and a
*replay-poll* work-unit budget, but neither document states a maximum number
of operations inside one `LivingCommandV2::CreatorBatch { operations:
Vec<LivingCreatorOpV2> }` (authority plan line 103), nor a maximum depth for
the pending/running command queue between boundary T and T+1 (rules spec
line 40's queue). Authority plan line 107 requires a test for "oversized
batches" without any document declaring what size is oversized.

### What it fails to say

Two related but separable prerequisites are missing, and the authority
plan's own wording treats both as if a source document already supplies
them when none of the three inputs to this review does:

- A **complete, explicit, numbered registry**: every `LivingEventKindV2`
  variant with its assigned `u16` ordinal (not just the category list in
  rules section 2 and the fixture prose in section 9); the phase table
  behind `phase_u16` (ten entries, matching rules lines 46-59's ordered
  list, each needing an explicit number); and the `intent_ordinal_u16` /
  entity-kind / local-ID tables rules line 319 requires to exist but never
  publishes.
- **Numeric limits** for creator-batch operation count and running/pending
  queue depth per boundary, parallel to the limits already tabulated in
  rules spec section 1 (lines 19-28) but absent from that table, plus the
  boundary/one-over rejection behavior the rest of the rules spec applies
  to every other declared maximum (e.g., section 1's capacity-check
  paragraph, or the "Fleet/ship capacity" and "Capacity reservation"
  fixtures at rules lines 260-261).

The civilizations plan (grounding only) already shows what happens without
this: it names concrete event variants and payload fields (e.g.,
`RecipeProduced`/`RecipeBlocked`, `Refunded`) that appear nowhere in the
rules spec's own text, while simultaneously asserting the schema is
"schema-frozen" and forbidding itself from introducing new discriminants —
i.e., a downstream, consumption-only plan is already the closest thing to a
registry that exists, which is backwards from the authority plan's own
Global Constraint (line 19: "Domain literals and little-endian framing are
exactly those in the rules spec") and from rules line 325's requirement that
"every implementation consumes the same corpus" of checked-in exact vectors.

### Proposed resolution

Before Task 4's commit, add to the rules spec (as new material near section
9/10, or as a new appendix section) three explicit, numbered tables and two
new limit rows:

> **Event kind ordinals.** A table of every `LivingEventKindV2` discriminant
> in lowercase-snake-case, one row per variant, each with an assigned
> decimal `u16` value starting at 0, grouped by the categories rules section
> 2's phase list already implies (creator/autonomous intent; construction/
> hull; recipe/block; repair; observation; agreement/war/truce; fleet;
> retreat; combat/destruction; occupation/capture; claim; shipment/route;
> dormancy/reactivation; hazard; refund; capacity-rejection). Ordinals are
> assigned once and never reordered or reused for a different variant.
>
> **Phase ordinals.** A ten-row table assigning `phase_u16` values 0-9, one
> per step already enumerated at rules spec lines 48-57, in that fixed
> order.
>
> **Intent/entity-kind/local-ID tables.** A table for `intent_ordinal_u16`
> covering every economic and fleet intent named in rules section 7 (tables
> at lines 194-205 and the fleet-priority list at line 210), and a table for
> every entity kind that can appear in `entity_kind_u16` (matching whichever
> entity types Task 3 has already fixed in `LivingGalaxyStateV2`).
>
> **New limits table rows**, added to the table at rules spec lines 19-28:
> `Creator-batch operations` with an explicit maximum (e.g., 256, chosen to
> exceed any single legitimate cascade while staying well under the 1,024
> decode/replay work-unit budget at line 329, which governs a different
> operation); and `Pending running-queue depth per boundary` with an
> explicit maximum, with the same atomic-rejection behavior rules section 1
> already specifies for every other capacity check ("Validate and reject
> such an order before changing its existing order, job, credits, or
> energy"). An over-limit batch or an over-limit enqueue attempt is rejected
> at submission before any sequence, ordinal, or resource is consumed —
> consistent with authority plan line 21.

### Owning task and why

**A4** (Task 4). This is where the preflight report's own citations already
point — every citation in its P2 section is to "Authority A4," never A3 —
and my independent reading agrees for a structural reason: every object that
needs one of these tables (`LivingEventKindV2`, `phase_u16` as consumed by
the autonomous-entity digest formula at rules lines 299-302, the
`CreatorBatch`/`batch_local_id` limits) is explicitly Task 4's declared
scope (authority plan lines 100-107), and Task 4 is the task whose own
checklist already claims these are "frozen" (line 105) without having
produced them. The one caveat worth recording: the *entity-kind* component
of the registry describes entities Task 3 defines first (Task 3 precedes
Task 4 in the plan's own ordering), so Task 4 should assign entity-kind
ordinals to Task 3's already-fixed entity set rather than redefine which
entities exist — Task 4 owns the numbering, not the entity list itself. This
placement also matches the Cross-Plan Dependency Order (authority plan lines
201-208): Task 4 is the last Authority task before Civilization Tasks 1-10
branch off to consume "the frozen event kinds," so it is the last point at
which this can be supplied centrally rather than invented piecemeal
downstream.

### What breaks if frozen unresolved

The Cross-Plan Dependency Order gates all ten Civilization tasks on
Authority Tasks 1-4 completing first specifically so civilization code can
"consume" a frozen wire format rather than define one. If Task 4 is marked
complete without an actual numbered registry, civilization implementers (and
the native/persistence/web split rules line 325 anticipates) will each have
to invent ordinals and limits ad hoc while writing phase modules — exactly
the "unreviewed wire-format decision" the preflight report warns about. Two
authors assigning different `u16` values to the same event-kind spelling, or
choosing different undeclared batch/queue caps, would silently break byte
compatibility between implementations that are each individually
self-consistent and pass their own tests, and would not surface until save
files or replay corpora produced by one implementation are decoded by
another — which is precisely the failure mode the shared exact-vector corpus
at rules line 325 exists to prevent.

---

## Where the preflight report and my reading agree, and where they add nothing to check

The preflight report's "Bounded coverage and no-finding areas" section (its
own lines 51-57) states it found no conflicting domain literals, wrapper
names, version constants, entity caps, or canonical-JSON conventions across
the inspected documents, and explicitly did not run any test, arithmetic, or
hash verification — "textual agreement only." I did not re-verify every one
of those items independently (out of scope for this review, and it would
require touching `crates/`), but nothing in my own reading of the rules spec
or authority plan contradicts that statement, and I found no fourth omission
of the same kind while reading both documents end to end.
