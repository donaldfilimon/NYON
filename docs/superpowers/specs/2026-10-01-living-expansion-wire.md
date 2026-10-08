# Living V3 wire and identity design

Date: 2026-10-03. Status: **PROPOSED coordinated review submission2; independent integrated review, fresh complete vectors and human approval pending**.
Parent: `2026-10-01-living-expansion-architecture.md`.
Existing Living V2 bytes, domains, identities, vectors and archive readers remain
unchanged. This document defines a new contract; it does not migrate any document.

## Encoding and trust boundaries

Canonical decoding and first-error ownership are exactly complete-schema §V3 decoder stages and exact error selection: rawcap/UTF8/syntax-depth/decoded duplicate members/entry-specific headers/structural fields/canonical bytes/installed IDs plus local semantics/linkage/framed integrity/integrated historical replay. Issued ID equality is deferred to installed identity, unlike nonissued duplicate keys. This keeps V1/V2 routing distinct and adds no State transport or Receipt header. Fraction/quantity domains and finite Catalog metadata live in schema, not codec defaults.


Use UTF-8 canonical JSON with prescribed field order, no insignificant whitespace,
unknown fields, duplicate keys, floats or noncanonical integer spellings. Record
maps are arrays sorted by their typed stable key. Reject duplicates and
unsorted input under schema decoder stages: nonissued repetitions at structural5,
issued equal IDs at installed identity7, historical origin/collision at integrated10. Re-encoding accepted bytes must
produce identical bytes. Reject excessive depth before recursive decoding.

Entity/branch IDs are 16 bytes encoded as exactly 32 lowercase hexadecimal digits;
digests and seeds are 32 bytes encoded as exactly 64 lowercase hexadecimal digits.
Ticks, sequence numbers and other u64 values use canonical decimal strings:
`"0"` or a nonzero digit followed by digits, without sign or leading zero.
Bounded u32 counts use JSON integers. Signed scores use canonical signed decimal
strings, reject `-0`, and range-check before conversion. Text is bounded by UTF-8
bytes and scalar count; do not silently normalize identities or display names.

Recognize explicit kinds `NYON_LIVING_PACK_V3`, `NYON_LIVING_GENESIS_V3`,
`NYON_LIVING_ARCHIVE_V3`, `NYON_LIVING_NOTEBOOK_V3` and
`NYON_LIVING_TRANSFER_V3`. Every top-level document carries format_version=3 and
rules_version=3. Kind/version mismatch rejects visibly while preserving the active
session. A V2 document remains routed to V2, including when its field widths match.

Initial byte caps: catalog 1 MiB, state/genesis 8 MiB, archive 64 MiB, Notebook
8 MiB, receipt8 MiB, transfer96 MiB and coordinator504 bytes (derived maximal
canonical schema width; null-revision maximum442). Maximum canonical nesting depth is 64.
Check raw UTF-8 byte length limits before decoding/parsing/allocation. Coordinator
505+ bytes returns byte_capacity before malformed/kind/version checks and preserves
resident without fallback; complete-schema correction I6 derives the exact bound. Capacity/byte overflow is
an explicit fault or rejected submission, never truncated canonical state.

## Authenticated operation contexts and reporting

Complete-schema owns exact closed FleetDispatchBlock/MovementPermissionFailure,
DispatchReservation/holders, ID-free Defense attempt/consideration, realized combat
Action/Defense credit and full Hazard carrier declarations. Required nulls and
field positions affect V3 canonical State/command/Actor32/candidate/receipt bytes;
no historical V1/V2 or old pinned-helper interpretation is changed. Capture.phase7
is retained while actual producer8/9 remains separate. No omitted binding/duration/
origin defaults, identity conversion or read-time migration exists.

Stage7's single creator-empty-Fleet cardinality deferral applies only genuine
initialized operation-indexed Records and registered unfinished prefix sources;
stage10 derives same-batch E, exact capture/effects and final E empty. Standalone
Genesis/completed State/selected HistoryView still rejects empty Fleet. Every
temporary source is authenticated at its original operation context, never a future
Receipt or eventual root. Terminal objective historical target/endpoint references
and retained SupplySuccess/Attempt World positions likewise use their exact owning
original producers; no general dangling-reference waiver exists.

Logical mixed mutation order is P/R/B/O/H/K/M/F/A, not receipt phase sort.
Authenticated replay reconstructs real W/ledger/guards/issuance at each operation,
then stable8/9 buffers and final global ordinals/Receipt/Event IDs. Successful AUTO
counters are actual producer-phase success subsequences, not Event ordinals; denied
attempts allocate no IDs. Existing scanner first live/batch comparisons and retained
branch chronology remain; source/derivation fault stops incomplete collision scan.
No outer source includes current Receipt/Revision self hash. Complete original
paid/creator ancestry must remain verifiable after cache retirement/save/fork;
a schema field retains duration/count origin, not a fabricated payment proof.

Canonical private-carrier access is whole-record authority/replay or explicitly
own actor; presentation can derive bounded safe notices but never export altered
canonical Events/Receipts. Holder lists and declaration-candidate identity in
Defense consideration must not reach requester/opponent views. Known public target
with hidden current owner can show entry unavailable without leaking owner or
creating an Observation. Needs guards follow complete-schema §Exhaustive Needs
guard subject, producer and privacy mapping: all five operation variants resolve
typed subjects at authenticated producer prefixes. Stage8 checks only supplied
typed graph/history-locator evidence available without producer replay, preserving
preliminary missing_reference/wrong_reference_kind; opaque IDs have no kind bits.
Stage9 integrity precedes historical reconstruction. Stage10 verifies original
membership/kinds/local-account/ownership links/selected Hull/amounts/cardinality:
missing prerequisite replay references use missing_reference, reproduced fact
mismatches use replay_mismatch, and Arithmetic faults retain invalid_range under
the existing replay order. No causal fault is backdated to8 and no current State
lookup substitutes for the original prefix. Own actor access requires
historical reason.world ownership plus verified owned Facility/Fleet links.
Missing historical proof removes the whole guard; present ownership, sight and
cargo receipt do not authorize private stock/hold quantities. No canonical Event
redaction or Actor32 quantity addition is permitted. Exact event/byte/depth/retirement limits fault atomically,
never truncate witnesses. Fresh independent vectors for the amended source freeze
precede complete human-package approval; no old helper pin bump is proof.

## Canonical state layout

State field order is:

`tick, branch_sequence, next_sequence, systems, stars, worlds, lanes,
civilizations, deposits, colonies, facilities, construction_jobs, hull_jobs,
fleets, routes, shipments, relations, agreements, wars, observations, hazards,
settlement_claims, occupations, offers, demand_requests, commitments, defense_requests,
shared_intelligence, stellar_events, counters`.

The ordinary topology, paid assets, fixed clocks and freight records retain their
specified semantics in V3-owned types. They do not alias V2 types or identifiers.
The new/extended surfaces are:

| Record | Required additions and canonical ordering |
| --- | --- |
| Star | next_activity_tick after name; checked 1,000-tick schedule |
| World | Environment after local inventory: base_habitability, stress, fertility, maximum_fertility, next_environment_tick |
| Inventory | energy, ore, alloy, food; each 0..10000 |
| Colony | Society after hub: population, specialization, next_needs_tick, last_needs, workforce assignments; Food remains solely in the world inventory |
| Civilization | Research after policy: target, per-technology progress, completed technologies |
| Facility | Workforce availability and effective recipe facts remain canonical, with exact scheduled clocks |
| Hull | passengers after next_repair, full Passengers{count,voyage_origin}; creator CreateHull submits count-only PassengerCountInput |
| Fleet | supply then blockade after hulls; complete order appends objective then defense_request |
| Shipment | original_travel_ticks after arrival_tick before disposition; full original provenance and creator override after returning_to_source |
| War | exact objectives and Peace; WarObjective inserts required secure_endpoints after target before count; SecureEndpoints{world_a,world_b} is directional Lane endpoint pair |
| Offer | id, proposer, recipient, kind, typed terms, created_tick, expires_tick |
| Commitment | id, parties, kind, typed terms, created_tick, expires_tick, obligation status |
| Defense request | id, commitment, requester, defender, exact target world, created_tick, expires_tick, disposition |
| Shared intelligence | recipient, originating actor, world, observed_tick, received_tick, bounded observed facts |
| Stellar event | id, star, warning_tick, start_tick, end_tick, habitability impact, freight factor, provenance |

Technology and specialization keys are catalog slugs, sorted by UTF-8 bytes;
technology prerequisite graphs must be acyclic. Workforce assignments are sorted
facility IDs, unique and bounded by available workers. Population, inventories,
research, environment and obligations are actual state, not presentation caches.
All optional fields encode explicitly as null unless a reviewed field schema
prescribes otherwise. The complete declarations, mandatory V3 overrides, keys, caps and variant
registries are in `2026-10-01-living-v3-complete-schema.md`. They replace this
introductory addition table wherever more specific; neither document approves
a codec before independent review and full-vector verification.

Actor OBSERVATION framing commits exactly32 fields in schema order, including
own_deposits, known_world_geometry{id,system}, own_settlement_claims and
own_occupations; source keys/caps/predicates are schema Actor32 source registry.
No State/Record/entity tag or ActorInputContext/SnapshotReference field is added.
The permitted hash, trace/context/AUTO-dependent frames change with these bytes.
Initial Shipment SnapshotReference/root circularity is handled by the optional
minimal B0–B7 profile, not a new genesis provenance variant or global richer-State
ban. Historical pre-amendment28-field fragments remain diagnostic under their
preserved source mirrors; new admission needs fresh current-release derivation.

## Clocks, limits and phases

Population/needs, ecology, research and treaty clocks use the society, ecology and
diplomacy numerical specifications. Use checked arithmetic throughout. No catch-up
loop may execute unbounded missed periods in one record visit. Normal simulation
advances one fixed boundary and applies the prescribed due-clock rule.

The authority phase order is creator interventions; lifecycle/ecology; arrivals;
conflict settlement; production/repair/research; needs/demographics/workforce;
observations; diplomacy; intents; dispatch; validation; sealing. Only complete
validated results publish. Shared-snapshot proposals and central reservations must
cover food/energy, worker/industry slots, research funding, routes and fleet assets.
Phase8/9 use the same immutable post-phase7 permitted captures; Decision.phase
records actual admission while snapshot.phase=7. Guard due phase5 discretionary
debits with transient actual Food/energy holds before phase6 needs. The exact four
tiers, Candidate/AdmissionTerms/ForceWitness/TieKey/audits and NeedsReserved facts
are mandatory closed declarations in complete-schema's integrated sections.

Initial history limits: 128 retained branches, 10,000 accepted revisions, 1,024
bookmarks and 256 Notebook experiments. Timed evolution is reconstructed from
exact genesis/catalog, revisions, fork inputs and tips; do not require retaining
100,000 full state snapshots or every unreferenced receipt in RAM. Authenticated
receipt anchors are validated by replay. Exhausting authored history capacity
rejects the new authored operation without stopping ordinary fixed-tick evolution.
The supplement now fixes proposed checkpoint/receipt/reclamation limits and
replay-authenticated Anchor reconstruction. Historical next_sequence is last
applied creator ordinal, branch_sequence producing fork ordinal; document high
waters stay separate. Running capture authenticates exact phase1 creator prefix,
not completed HistoryView; final Receipt waits for complete queue/phases. Fork
framing pins parent digest while child changes only branch_sequence. Notebook
Bookmark/Experiment/Evidence share one positive never-reused ordinal and Evidence
requires its actual Experiment-side receipt/witness. These Proposed contracts
require integrated review and fresh literal vectors.

## Identity framing

All domains below are ASCII bytes terminated by one NUL:

- `NYON_LIVING_V3_PACK`
- `NYON_LIVING_V3_STATE`
- `NYON_LIVING_V3_GENESIS`
- `NYON_LIVING_V3_ROOT_BRANCH`
- `NYON_LIVING_V3_REVISION`
- `NYON_LIVING_V3_RECEIPT`
- `NYON_LIVING_V3_EVENT`
- `NYON_LIVING_V3_ARCHIVE`
- `NYON_LIVING_V3_NOTEBOOK`

Pack/state/genesis/archive/Notebook digests are SHA256(domain || canonical payload
without its own integrity field). The payload must include every interpreted
input and its explicit version. Never hash an object containing its own digest.

Root branch digest is SHA256(root-domain || catalog_hash[32] || seed[32] ||
genesis_digest[32] || generator_id_length:u32le || generator_id:UTF8 ||
generator_version:u32le); branch ID is its first 16 bytes. Generator IDs use
validated catalog slug grammar. All lengths and widths are checked before framing.

Ordinary receipt payload field order is tick, applied_revisions, events,
state_digest. Applied revision IDs sort strictly; events preserve authority
emission order and carry phase, ordinal, provenance, kind and typed facts.
Receipt digest hashes that payload. An event digest hashes event-domain ||
receipt_digest[32] || tick:u64le || phase:u16le || ordinal:u32le; ID is its first
16 bytes. Maximum events per boundary is 65,536, with contiguous ordinal 0..65535.
Creator provenance includes its exact revision. Actor decisions include actor,
snapshot digest/tick, observed fact provenance, candidate score and disposition.

Autonomous/creator/genesis entity, fork and revision framing, kind/namespace
registries, allocator consumption and collision semantics are declared in the
supplement. Revision IDs use full32-byte digests; entity/branch/event IDs use16.
Independent allocation/collision vectors remain required before implementation.
The stellar schedule deliberately uses the ecology document's exact SHA256 input
formula; do not substitute one of these NUL-terminated identity domains.

## Store, transfer and negative acceptance

New coordinator records name explicit authority/version, exact slot, catalog,
branch and selected history view. A present invalid newer coordinator cannot fall
back silently to a different old selection. Generation CAS applies to every
head-dependent mutation. Pack/name equality never substitutes for byte identity.

Transfer binds exact catalog, genesis, history, selected view and Notebook evidence
in one coherent capture. Validate bounded bytes, schemas, graph references,
integrity and replay before replacing/creating a resident. Unsupported rules,
wrong catalog, stale capture, missing branch, forged witness and incomplete
artifacts have distinct visible outcomes. Import creates a separate slot.

Independent vector derivation must precede codec code. Verify full canonical bytes
and digests, not just parsed equality, including unknown/duplicate fields, signed
scores, integer boundaries, Unicode text, truncated frames, domain mismatch,
unsorted/duplicate identities, capacity rejection, cancellation and cross-platform
replay. The initial domain/empty-state vector proposal is a subset of this required
corpus; it cannot qualify the complete schema or mechanics.


## Complete schema supplement and integration precedence

`2026-10-01-living-v3-complete-schema.md` is the proposed normative declaration
for every state/catalog/command/event/history record, new request/peace state,
identity framing and bounded history work. It freezes inherited source fields
as copied V3 declarations, not aliases, imports or a live dependence on V2.
The companion's explicit overrides supersede the introductory table here.
Society/ecology/diplomacy numerical formulas and the approved V2 contracts remain
unchanged. Numerical within-priority economy/defender ranking and the independent
full vector corpus are still prerequisite work.

In particular, the supplement removes receipt/event identity circularity by
hashing ID-free EventPayloads before deriving public IDs. State hashes explicitly
bind the V3 version and exact catalog. Creator allocation excludes revision hash
so a reviewed before/after cascade can contain created IDs without circularity.
Genesis allocation excludes genesis/root digests for the same reason. Full32-byte
RevisionIDs and16-byte entity/branch/event IDs are distinct wire types.

A source inventory and consistency review are documentation evidence only. The
October1 review's open items1/2/4/5 now have concrete Proposed declarations to
review; this does not retroactively label that review complete or independent,
approve numerical mechanics, or implement V3 codecs.

## New Proposed technical and compatibility decisions requiring written review

1. Required fields with explicit null/false/empty values and exact canonical JSON
   escaping; Unicode Name limits256 UTF-8 bytes/64 scalars versus V2 ASCII.
2. Full32-byte RevisionIDs,18 stable entity kind tags, three separate allocation
   formulas, exact actor/lifecycle namespaces and explicit typed snapshot binding.
   Collision detection covers candidate/live and every retained replay issuance
   through bounded16 MiB caching plus mandatory streaming rescans, including
   removed transient entities; no lifetime issuance index is retained.
3. Mandatory typed reviewed_effects for all creator commands; whole-record
   before/after snapshots make hidden cascades independently inspectable. Stellar
   record provenance uses pre-hash CreatorCapture; exact RevisionID stays in its
   sealed receipt. Creation/edit/genesis framing has no command self-reference. There
   is no generic arbitrary-JSON patch operation.
4. Demand state roster, request visibility limited to current direct observation
   or a direct bilateral commitment,1024 request cap, deterministic offer
   consideration/admission, same-terms negotiated renewal retaining commitment ID,
   pending offer consideration/wait facts, exact observed enemy/threat payloads,
   and explicit responder objectives plus next-boundary peace consideration.
   These visibility/admission choices are mechanics proposals requiring review.
5. Paid supply fulfilled at actual delivery using immutable dispatch route/treaty/
   payment provenance; fabricated or creator-overridden manifests cannot fulfill
   negotiated commitments. Supply separates last successful payment and latest
   attempt, so first failure and expired supply are valid. Authenticated prior-receipt Anchors
   retained in state, current-boundary outcomes in typed facts. Anchors to their
   own sealed receipt would produce a state/receipt hash cycle and are forbidden.
6. Construction/hull-job caps4096/512 with separate industrial versus hull/fleet
   reservations (hull jobs consume no extra industrial slot); separately preserved128 hazard and128
   stellar-event caps;
   demand/intelligence/technology/catalog/queue capacities; agreements and new
   commitments share the proposed256 typed union with derived future-Truce keys of
   retained Wars. Removing a War generically cancels that key; peace/expiry converts it. These bounds are
   explicit technical proposals, not measured capacity qualification.
7. Checkpoints8/64 MiB, recent receipts128/16 MiB, anchors16384, evidence4096,
   reclamation16 owners/128 MiB, 64 MiB collision-validation workspace with16 MiB bounded cache/streamed rescans;
   resumable replay segments1000 boundaries/
   16777216 visits, comparison16 MiB and64 KiB/256-row pages, export buffer1 MiB.
   No receipt omission or authored-history exhaustion stops normal ticks; receipt
   overflow faults atomically, and export/comparison limits give explicit errors.
8. A separate V3 coordinator recognizes only living_v3; V1/V2 remain in their
   existing records and appear through version-specific routing. A present bad
   newer coordinator cannot silently select an older document. No migration,
   alias, implicit defaults, root dependency or V2-vector rewrite is introduced.
9. State digest wraps explicit version/catalog plus ordered State. Proposed
   domain/empty-state vectors must be rederived; original V2 vectors stay frozen.
10. Retained Industrialist policy receives Neutral's research order pending
    review; society/scoring now records that explicit order. Exact within-priority
    utilities, four categories, deterministic ties and offer/objective fallbacks
    are in scoring §§1–8, integrated here as Proposed pending full package review.

## Required independent written review before codec work

Review all six expansion proposals together with the complete schema supplement,
trace every interpreted input through canonical records and hash frames, and
check every event-kind/facts mapping, cross-phase clock and actor-visibility
rule. Independently derive full canonical bytes for genesis, populated state,
creator cascade/revision/fork, diplomacy requests/commitments/objectives/peace,
receipt/event identity, archive, Notebook, transfer and coordinator.

Negative corpus must cover every enum/null/default/sort boundary, unknown or
wrong-kind references, counter overflow, duplicate/truncated-ID collisions,
malformed/Unicode/escaped bytes, false history/receipt anchors, schema/capacity
rejection, cancelled/stale preparations and cross-platform replay. Mechanical
text checks and source inventory matching cannot replace that review or corpus.


## Written correction round1 (W1–W8)

The independent review required concrete corrections, now declared in the
supplement: pre-hash record CreatorCapture/genesis provenance; last-success versus
last-attempt Supply; immutable paid/creator shipment provenance; pending-offer
consideration facts; explicit observed hostile actors; closed collection/Record/
TypedKey encodings and metric/bookmark validation; bounded exact retained-history
collision rescans; and distinct hull/building reservations. All remain PROPOSED
and require scoped independent rereview. No utility formulas, rules balance or
codec implementation were added by these corrections.


Round2 narrows the remaining validity/hash issues: unavailable supply attempts
create no new payment and never clear/extend the prior successful payment;
readiness is derived from its exact deadline independently of the attempt result.
Observed-threat provenance now hashes a separate explicit version/catalog/
observer/world/tick/enemy fact payload under THREAT_OBSERVATION plus final NUL,
excluding the digest and every enclosing snapshot. Sharing/forks preserve that
original fact capture. These corrections remain PROPOSED and require narrow
independent rereview; they add no balance values.


## Integrated scoring compatibility and corpus applicability, 2026-10-02

Canonical Decision adds candidate:Option<Candidate> after action. Material
decisions require exact nonnull full action+admission terms identity; Tier includes
obligation. Actor observation payload adds incoming_shipments and party-visible
agreements/wars/offers in the schema-prescribed order. New audit/guard/hold event
tags and Facts are closed; all other State/catalog/entity/revision/Receipt framing
formulas remain as prescribed. Additional interpreted snapshot/receipt fields
change their canonical bytes/digests; no omitted defaults or old-draft V3 fallback
may fabricate them. No V2 format/hash behavior changes or migration is introduced.

Autonomous War action remains peer-only; complete declarer objective/force is in
AdmissionTerms, excluding future defender selection. Central private admission
attaches its same-tick defender objective and records exact audit bindings; only
successful War creation exposes the public choice. Private audits do not enter
actor snapshots, own identity or enclosing self-hash inputs.

Existing independent living-v3-wire-proposed corpus stays frozen to the reviewed
round2 source hashes saved in v3-integration-baseline. Its32 frames/14 documents
remain independently checked baseline examples, not integrated source acceptance.
Its source-freeze verifier intentionally rejects changed source text. Do not edit
its generated bytes to follow this integration. New/rederived snapshot/decision/
audit/guard/admission vectors and full gap6 need a separate reviewed corpus against
this package's new hashes; unchanged empty/catalog/genesis/root/entity formulas
can be carried forward only after an explicit new-source comparison. The older
living-v3-proposed mechanics subset is likewise partial historical evidence.


## Integrated round1 correction applicability

Permitted snapshots now contain bounded timestamped lane-hazard and stellar target/
interval/factor facts; only authorized observations disclose creator-edited factors.
ActorInputContext excludes hidden authority State/revision digests from trace hash
context; full SnapshotReference stays in outer authenticated CandidateAudit/Receipt.
Equal permitted inputs have equal eligible trace digests, not necessarily equal
outer receipts. ScoreComponents has economy/offer/war tagged exact fields/zero
rules; traces hash eligible complete candidates only, with ordered central-attempt
reasons separately. SetFleetOrder explicitly appends objective and defines clear/
replace/cancel/ForcePeace semantics. Coordinator cap is504 before parse. The schema
correction sections are the mandatory exact field/visibility/framing registry.
All choices remain Proposed; prior baseline corpora remain unchanged/stale for
this new source freeze and full integrated vectors remain required.


## Integrated round2 environmental scope correction (Proposed)

Complete-schema §Integrated correction I1 is the normative environmental knowledge
registry. Complete timestamped system/lane assertions include empty negative
coverage; choose newest scope across all permitted source worlds before flattening
positive event IDs. Select newest per-event child over retained winning positives
INCLUDING expired children, then filter the chosen child by expiry, then check128-
fact capture limits; never resurrect a losing older copy on expiry. Scope history
is unchanged by filtering; source refresh/eviction is separate knowledge loss. Equal ticks use lowest
original observer then canonical scope bytes, without own-source priority. Missing,
partial or unrelated scope data grants no absence evidence. Shared/request copies
preserve scope bytes and timestamps. Forecasts use only winning permitted scopes;
hidden cancellations retain stale facts until authorized supersession or expiry.
Fresh schedule observation requires an incident directly visible world; own fleet
traversal alone does not refresh schedules. This explicitly Proposed visibility
refinement replaces round1's fleet-only read and changes observation/capture wire
bytes, not balance tables, combat sight or movement. Empty winners enter actor
snapshot hashes; complete observation carriers and scopes obey schema bounds.
Old-baseline corpora remain historical, and full integrated gap6 remains Partial.
