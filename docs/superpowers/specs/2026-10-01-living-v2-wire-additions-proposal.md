# Living V2 additive wire contracts

Date: 2026-10-01

Status: **Proposed implementation contract; independent authority review pending.**
The user approved the scope of additive V2 shapes at 03:30 EDT on 2026-10-01.
That approval authorizes design/implementation review; this draft is not evidence
that its exact encodings have been implemented or qualified. No wire source or
existing vectors are changed by this document.

This closes expressiveness gaps in the accepted rules, qualification capability
matrix, and creator operations addendum. It adds no gameplay system. Existing
V2 tags, field orders, hash domains, entity kind ordinals, command bytes, receipt
bytes, archive bytes and independent golden corpus remain unchanged. WorkshopV1
is unaffected. New variants get new tags; old records must round-trip exactly.
Unknown tags reject explicitly. The authority reviewer approved the decision-receipt portion after clarifying
phase snapshot tick normalization. Remaining sections retain their review
boundaries before changing authority emission.

## 1. Shared encoding and compatibility

All records use the existing canonical V2 JSON rules: declared field order,
strict unknown-field rejection, lowercase fixed-width hexadecimal identities,
existing base-10 integer encoding for ticks/u64, no floating-point numbers, sorted keyed
collections, and canonical tagged-option/reference forms already used by V2.
Every shape below lists fields in wire order. An `EntityRef` uses the existing
`{type:existing,id}` or `{type:local,local}` shape. Noncreating operations never
consume a batch-local entity ID. Existing operation tags retain their schemas.

New production wire code must have independent specification-derived vectors,
including byte order and hash framing. It must not regenerate old expectations
from Rust output. Old vectors are verified before and after implementation.

## 2. Authoritative intent explanations

Introduce a separate canonical decision receipt record. Its association with a
tick receipt is explicit; it does not insert fields into an existing event or
change existing receipt/event identities.

`DecisionReceipt = {kind:"NYON_LIVING_DECISION_RECEIPT",format_version:2,
rules_version:2,catalog_hash,branch,tick,committed_state_before,state_after,
receipt_digest,intents,integrity_sha256}`.

`intents` is ordered by `(phase,actor,ordinal)`; ordinal is a u16 zero-based within an
actor/phase emitted attempt list, capped at 65536 emitted attempts; overflow is an atomic
deterministic fault, never wraparound. Action registry ordinals are fixed in the
order construction=0, hull=1, route=2, explore=3, move=4, settle=5, defend=6,
attack=7, return=8, agreement=9, war=10. Candidate emission order is action
ordinal then canonical target ID then complete canonical action payload. Ranking
continues to use the accepted score/target tie rules; registry order only resolves
an otherwise exact tie. This registry is for additive decision disclosure; it
never replaces the existing autonomous entity intent-ordinal framing or registry.

Disclosure records actual reservation attempts, including rejected conflicts and
the accepted proposal. The immutable logical candidate search still applies the
complete accepted score, tier, target and action ordering against one shared
snapshot. After an actor/category accepts an intent, lower-ranked alternatives
are not attempted and need not become portable LowerRank rows. A failed
reservation must retain ranked fallback alternatives and its truthful reason.
Emitted attempts are densely numbered in their canonical emission order; this
disclosure numbering never changes autonomous entity identity inputs. It is not
a claim that every possible logical alternative fits the emitted-attempt cap or
that all-rejected attempts fit the receipt byte cap. Those capacity and bounded
search obligations remain separately qualified.

`receipt_digest` references the ordinary tick receipt.
`committed_state_before` is the published completed T-1 boundary;
`state_after` is completed T. Every intent also binds its own phase snapshot
digest: diplomacy uses the accepted post-update snapshot and phase-8 decisions
use the post-phase-7 visible snapshot. These snapshots are immutable within each
collection/reservation pass. Integrity is
SHA256 of `"NYON-LIVING-DECISION-RECEIPT-V2\0"` followed by canonical payload
bytes omitting only `integrity_sha256`; the domain terminates in one NUL byte,
not literal backslash characters. This record is authority-produced and replay
validated, never a presentation-authored explanation. It does not enter the
state digest or allocator counters.

`Intent = {phase,actor,ordinal,action,target,score,tier,snapshot_tick,snapshot_digest,observed_tick,
outcome,reasons}`.

- `phase`: existing phase integer, restricted to the phase that evaluated it.
- `actor`: stable civilization ID; `target`: nullable existing entity ID.
- `action`: tagged enum `{type:construction,world,definition}`, `{type:hull,
  world,definition}`, `{type:route,source,destination,resource}`, `{type:explore,
  fleet,destination}`, `{type:move,fleet,destination}`, `{type:settle,fleet,
  destination}`, `{type:defend,fleet,world}`, `{type:attack,fleet,world}`,
  `{type:return,fleet,world}`, `{type:agreement,peer,agreement_kind}`, or
  `{type:war,peer}`. Payload identifies the candidate without creating identity.
- `score`: canonical signed decimal string bounded to i64, no leading plus,
  leading zero or negative zero. Score is the exact authority ranking score.
- `tier`: `power_emergency`, `ore_emergency`, `alloy_emergency`, `ordinary`, or
  `recovery`; tiers expose the accepted rules' priority, not a new bonus.
- `snapshot_tick` is T. `snapshot_digest` hashes the immutable phase-visible
  snapshot with its private state.tick normalized to T, matching candidate
  validation. Although internal phase state retains T-1 until commit, phase-6
  observations already carry T; never label a T-1 digest as a T snapshot.
- `observed_tick`: nullable target intelligence observation tick under rules
  section 7/decision disclosure; null means no target observation is used. It is
  not the actor snapshot clock and cannot be later than the snapshot boundary.
- `outcome`: `accepted` or `rejected`. Rejected candidates spend no resources.
- `reasons`: nonempty ordered structured reason list, at most 64 reasons per
  candidate, never translated free text. Decision sidecar has a 2 MiB byte bound.
  Agreement/war action tags belong only to phase 7; other listed tags to phase 8.

`Reason` tags and ordered fields:

| Tag | Fields after type | Meaning |
| --- | --- | --- |
| `insufficient_resource` | world, resource, required, available, reserve | Snapshot/reservation shortage |
| `output_capacity` | world, resource, amount, available_capacity | Storage prevents action |
| `queue_capacity` | world, queue, limit, reserved | Queue admission unavailable |
| `target_reserved` | target, winner | Another candidate won central reservation |
| `lower_rank` | winning_actor, winning_ordinal | Legal candidate lost allowed intent slot |
| `missing_intelligence` | world | Actor has no required observation |
| `stale_observation` | world, observed_tick, maximum_age | Accepted freshness rule fails |
| `invalid_target` | target, rule | Structured invariant slug |
| `dormant_restriction` | actor | Candidate prohibited for dormant actor |
| `bilateral_threshold` | peer, forward, reverse, required | Agreement/war eligibility |
| `truce_active` | peer, end_tick | End-exclusive truce blocks action |
| `policy_score` | policy, component, value | Exact scored contribution |
| `forecast` | world, horizon, resource, predicted, demand | Existing 200-tick forecast witness |
| `accepted` | rule | Named accepted rule witness |

All quantities use existing resource/counter bounds. `rule`, `component`, policy
and definition identifiers are validated lowercase catalog/rules slugs, not
arbitrary narrative. Reasons must be sufficient to reproduce why the published
winner won; no fabricated score for heuristics not implemented. UI localizes
these records and separately labels creator interventions and scheduled events.

**Transport review decision:** the recommended additive transport is this
separate record returned alongside the existing tick receipt by a new API, with
bounded retention and deterministic replay reconstruction. Do not expand the
old `LivingEventPayloadV2` fields or reinterpret its string kinds. Exported
histories reconstruct explanations from authoritative inputs; caches are not
canonical archive inputs. Native/wasm must reproduce decision receipt bytes.

## 3. Missing creator operations

Add these snake-case operation variants to `LivingCreatorOperationV2`. Each uses
the existing atomic envelope, tail checks, revision identity and creator
provenance. Nullable options use existing V2 option encoding. Editing preserves
target identity. The creator pays no resources; structural, clocks, ownership,
capacity and reference invariants still apply. Unchanged values are accepted as
ordinary no-op revisions. All schema fields below are mandatory.

| Tag | Ordered fields after type |
| --- | --- |
| `edit_system` | target, name |
| `edit_star` | target, system, archetype, name |
| `edit_world` | target, system, archetype, name |
| `edit_lane` | target, system_a, system_b, distance_units |
| `edit_deposit` | target, resource, remaining_units |
| `edit_civilization` | target, name, policy |
| `set_facility_enabled` | target, enabled |
| `scrap_facility` | target, cascade |
| `cancel_construction_job` | target |
| `cancel_hull_job` | target |
| `edit_fleet` | target, owner, world |
| `edit_hull` | target, definition, hit_points, return_credit |
| `split_fleet` | local, source, hulls |
| `merge_fleets` | target, source |
| `edit_route` | target, kind, source_world, destination_world, resource, batch_size, cadence_ticks, source_reserve, source_owner, receiver_owner |
| `set_route_suspended` | target, suspended |
| `edit_hazard` | target, lane, definition, start_tick, end_tick |
| `cancel_hazard` | target |

Use the corresponding existing create operation/model field types for name,
archetype, definition, hit points, return credit, route and hazard fields.
Systems/worlds have no authoritative position and fleets have no name; these
operations do not add either. Civilization color is presentation metadata,
requiring a separate approved presentation-settings shape; it cannot be silently
inserted into existing civilization state bytes. Accepted rules permit one queued building job and one hull queue/world. This
draft adds no multiple-job scheduling; reorder tags are deferred until a
separate accepted queue ordering contract gives them a meaningful operation.
Cancellation
uses the accepted refund and overflow-receipt rule. Moving a fleet is an explicit
creator relocation: reject transit or incompatible hull-job reservations;
retire/revalidate its orders atomically. Split/merge require idle co-located
fleets of one owner; sorted nonempty hull IDs, exact membership, capacity and
never-reused IDs are validated. Preserve hull IDs, HP and credits. Split creates
only the new fleet local ID; merge preserves target ID and removes source ID.

Edits may not silently destroy references. Lane/world/route edits validate every
in-flight dependent; reject until an explicit valid disposition is submitted.
Route edits retain next_due unless an accepted adjustment requires rescheduling
to application tick plus the normative cadence (10); reject arbitrary cadence.
Suspension retains route identity and canonical manifest; reactivation rechecks
endpoints, agreement/owner eligibility, reserves and clocks. Hazard intervals
remain end-exclusive and must obey accepted creator boundary constraints.
Ordinary model validation and creator projection must agree before commit.

### Persistent facility enabled state

Add `disabled:true` as the final field of `LivingFacilityV2` when disabled.
Absent means false; explicit false is noncanonical and rejected. Existing
facility fields and their order remain unchanged. Canonical encoding omits false,
so every existing facility/state vector round-trips byte-for-byte. True is part of
canonical state bytes and therefore contributes to the existing state-digest
formula. Never overload Operational/Blocked, which describes the last recipe.
`set_facility_enabled {target,enabled}` sets disabled to !enabled. Disabled
facilities preserve identity, hit points and due clocks; a due disabled attempt
advances its clock using the accepted due-attempt rule, consumes/produces nothing,
and records a structured disabled reason. Re-enable never creates catch-up.
All newly built/genesis facilities default enabled. Existing decoder behavior
for an unknown disabled field stays strict; updated decoder accepts the additive
canonical true form. Golden tests pin absent and true forms independently.

## 4. Explicit derived-only cascade

Keep `remove_listed` and `reject_if_referenced` encodings unchanged. Add:

`Cascade = {type:"remove_reviewed",dependents,derived}`.

`dependents` contains the exact sorted stable entity references from the removal
preview, possibly empty. `derived` is the exact sorted keyed-record disposition:
`{type:colony,world,owner}`, `{type:observation,civilization,world}`,
`{type:relation,from,to}`, `{type:agreement,a,b,agreement_kind}`,
`{type:war,a,b}`, `{type:occupation,world,claimant}`,
`{type:order,fleet}`, or `{type:settlement_claim,world,civilization}`. Colony and occupation are keyed by world alone; owner and claimant are
verification values, not extra key dimensions. Relations key by (from,to),
observations by (civilization,world), claims by (world,civilization). Existing model
key fields determine sorting; duplicate,
missing, extra or mismatched keys reject before any effect. Review must refine
this key list to actual model records without inventing records. Empty both is
allowed only when preview reports no references. This supplies explicit consent
for derived-only removal without pretending keyed records have entity IDs.

Preview carries target, stable dependents, derived keys and deterministic effects
(refunds, discarded cargo, retired orders). Apply recomputes against the private
pending-tail projection and requires exact equality. The accepted stable-entity closure remains fixed: if it includes a fleet it
includes every hull. An order-only derived disposition preserves its fleet only
when no stable dependency rule requires that fleet to retire. A creator cannot
shrink the computed closure by naming an order instead of the required fleet.
No preview capability token
can bypass authority. Failure preserves queues, resources, IDs and history.

## 5. Genesis-only fork identity

Existing revision-based fork formula and bytes are frozen. A child fork at a
null revision cursor receives a separate domain:

`branch_digest = SHA256("NYON-LIVING-GENESIS-FORK-V2\0" || parent_branch_16 ||
genesis_digest_32 || fork_tick_u64_LE || branch_ordinal_u64_LE)`;
branch ID is the first 16 bytes. Genesis digest is the exact 32 bytes of validated embedded genesis
`manifest_digest()`, equal to its canonical tick-zero state digest; no new genesis
serialization or hash formula is introduced. Parent
branch can itself be a genesis-only fork. Ordinal comes from the same global
never-reused branch allocator. `fork_revision:null` plus nonnull `parent` selects
this formula; root remains parent:null, ordinal zero and existing root formula.
No sentinel fake revision is inserted. Retained parent future is immutable;
archive graph validation distinguishes root from genesis-only child and replays
parent history to the exact fork boundary. Test tick-zero and nonzero no-edit
forks, repeated forks, parent null cursor and capacity refusal atomically.

## 6. Portable bookmarks and experiments notebook

Preserve existing archive format_version 2 bytes. Use a separate optional file:

`Notebook = {kind:"NYON_LIVING_NOTEBOOK",format_version:2,rules_version:2,
catalog_hash,genesis_digest,archive_integrity_sha256,bookmarks,experiments,
integrity_sha256}`.

Integrity domain is `"NYON-LIVING-NOTEBOOK-V2\0"` followed by canonical payload
omitting integrity field. Max UTF-8 bytes 1 MiB; max 64 bookmarks and 64
experiments. This accompanies the catalog/archive pair without changing their
existing bytes. Exact archive integrity prevents attaching a notebook to another
history. Re-export regenerates that attachment against the exported archive.

`Bookmark = {ordinal,label,view,digest}` with existing exact view and state digest;
label is bounded 64 UTF-8 bytes, normalized with existing display-name rules.
Bookmark ordinals are unique increasing u64, never reused within the bookmark namespace. Experiment ordinals independently use a unique increasing u64 namespace; left/right always resolve only bookmark ordinals. Import validates
all references and replays every bookmark before installation. No checkpoint
state is serialized. An invalid notebook cannot change the imported authority or
an already resident session; user may explicitly import the valid galaxy pair
without notebook. Dropping notebook never changes gameplay identity.

`Experiment = {ordinal,label,left,right,comparison_tick,observations}`. Left and
right reference bookmark ordinals. Comparison tick is identical for both views;
if stored bookmark ticks differ, materialize both at comparison_tick under their
own histories. `Observation = {metric,entity,left,right}`; metric is a validated
allowlist of inventory resource, owner, colony count, facility count, hull count,
agreement state, war state or completed tick. Values use `{type:integer,value}` (canonical signed decimal string),
`{type:entity,value}` (nullable entity ID), `{type:boolean,value}` or
`{type:absent}` for an entity absent from that view. `metric` tags are
`inventory_energy`, `inventory_ore`, `inventory_alloy`, `owner`, `colony_count`,
`facility_count`, `hull_count`, `agreement_active`, `war_active`, `tick`.
Reviewed initial scopes: inventory and owner require a world ID; counts use null for the galaxy total or a civilization ID for owner scope; tick requires null. A missing scoped entity measures absent. Integer values are signed i64 decimal strings; a quantity outside that range rejects explicitly. Observations sort by metric registry order above then nullable entity ID;
duplicates reject. Agreement/war metrics additionally require their complete
canonical pair/kind key, so their final payload schema remains a review item;
they are withheld from initial source rather than represented by ambiguous
single entity IDs. No unrestricted JSON is accepted. Observations are measured replay outputs; decoder
recomputes equality. UI can display a conclusion as user-authored annotation,
separately labeled, but this proposal does not persist such text as evidence.

## 7. Review and qualification boundary

Before implementation, the authority reviewer confirms exact existing model
field/key mappings, signed score encoding, decision-record emission/retention,
all edit conflict semantics, genesis digest framing and notebook relationship.
Resolve ambiguities in this file, then pin independent new vectors. Approval of
scope is not approval of an accidental encoding from first implementation.

For each new operation test valid/invalid/capacity/cascade/paused/running,
projection/replay, undo/redo and canonical reload. Test stale tail and arithmetic
faults preserving all authority state. Test standalone decision/no-op creator
records without rewriting old receipt bytes. Test notebook corruption, wrong
catalog/archive, stale views, cancellation and exact matching-tick observations.
Run untouched old corpus and new full-byte native/wasm corpus. Live forms,
semantic operation, import/export pairs and notebook transfer need separate
journeys. Proposed schemas alone satisfy no LG completion requirement.

## Reviewed observation defense extension

`LivingObservationV2` appends optional `operational_defense_batteries: u16` after
`stationed_hulls`. Absent means the legacy visible battery aggregate remains a
conservative defense bound; it is not an exact operational-count witness. The
field is present only when the measured operational count is strictly less than
the observation's aggregate `defense_battery` count. Equal or greater values
reject state validation. Explicit null is noncanonical because absence is the
canonical encoding of None. A refresh measures batteries visible at that world
with positive HP and `disabled == false`; a difference publishes the exact count,
including zero. Hidden changes never refresh this field. The war heuristic uses
the exact count when present and the conservative aggregate otherwise. Normal
observations retain their existing bytes. Historical V2 replay containing
previously unrepresented disabled/nonoperational batteries can change under this
additive behavior; Classic and WorkshopV1 identity and replay remain frozen.
Independent new vectors qualify this field separately from the existing corpus.

## Reviewed construction replacement witness

The `construction` decision action gains a final optional `scrap` field containing
the exact `LivingEntityIdV2` of a producer removed in the same atomic construction
reservation/commit. The order is `type`, `world`, `definition`, then `scrap` when
present. Absent/None preserves the existing action bytes; explicit null is
noncanonical because canonical serialization omits None. This does not add a
creator command or alter old autonomous identity framing. The extra payload
participates in sidecar integrity and canonical-payload tie ordering.

Eligible producers are only `solar_array`, `extractor`, and `foundry`; a candidate
must be explicitly disabled or have no successful recipe in the full scheduled
200-tick forecast. The removed producer must duplicate its definition with at
least one enabled producer remaining afterward. Hubs and unique shipyards/defense
batteries are never eligible, and an in-progress repair target is protected.
Paid construction and hull jobs retain their existing world references. Payment,
queue reservation, removal and remaining enabled producer protection use the
same authoritative snapshot and commit atomically; losing candidates remove
nothing and spend nothing. This witness makes the approved full-slot replacement
behavior inspectable without adding gameplay.

### Reviewed immutable shipment dispatch provenance

The approved additive scope adds final optional `dispatch_kind` to
`LivingShipmentV2`, after `disposition`, with the existing route-kind strings
`internal`, `trade`, or `aid`. `None` is omitted; explicit null is noncanonical.
Existing shipment bytes remain unchanged. Autonomous dispatch snapshots the actual
route kind so deleting, suspending or editing that route cannot reinterpret
in-flight cargo. Unknown legacy provenance earns no invented Aid/Trade counter
credit. Credit applies once only to successfully unloaded outward cargo at its
intended receiver; capture, return and capacity waits earn none. This field adds
no new gameplay and preserves the accepted directed relation counter formulas.
Independent fixture covers omitted, Aid and Trade full bytes and ordinary SHA256.

A second final sparse shipment field, `returning_to_source: bool`, follows
`dispatch_kind`. False is omitted. True is valid only with
`disposition: "waiting_for_storage"` and preserves a cargo's already-selected
return destination while source storage is full. It earns no outward delivery
credit and cannot be reinterpreted as waiting at the destination on the next tick.
Independent fixture includes a Trade shipment waiting at its source with true.

## 10. Approved additive ordinary-event facts

Status: approved scope and schema by implementation coordinator on 2026-10-01;
source and focused regression work are in progress. This explains accepted
ordinary events without changing their payload, receipt digest, event identities,
or existing independent vectors. It introduces no additional gameplay.

The public `LivingEventFactsReceiptV2` field order is exactly
`tick`, `state_digest`, `receipt_digest`, `events`, `integrity_sha256`.
Its integrity payload contains the first four fields in that order. The digest is
`SHA256("NYON-LIVING-EVENT-FACTS-V2\0" || canonical_payload_bytes)`; the final
integrity field is lowercase 64-hex. Tick uses the existing canonical tick scalar;
state and receipt identities retain their existing 32-byte lowercase hex encodings.
Each event entry is `{ordinal, event_id, facts}`, with an existing emission `u16`
ordinal and the exact ordinary receipt-derived 16-byte event identity. Entries
are sparse and strictly increasing by ordinal; missing explanations mean unknown.

Facts field order is `{actor, entity, resources, blocked_reasons, affected}`.
Actor/entity are optional 16-byte identities, encoded as existing optional values
including explicit null when absent. They are emitted by the producer from the
actual operation, not looked up from subsequent presentation state. Resources
are `{role, resource, quantity}`; roles in canonical wire/sort order are
`discarded`, `input`, `output`, `paid`, `refund`, `transferred`. Resource is the
existing `alloy`, `energy`, or `ore` enum; quantity is a canonical `u32` in
0..10000. Up to 18 resource entries are strictly ordered and unique by
(role,resource); quantities describe each event, not a fabricated aggregate total.

`blocked_reasons` reuses actual `LivingBlockedReasonV2` records and their existing
field order. Up to eight entries are strictly sorted and unique by reason kind
and resource: disabled, missing_deposit, missing_input by resource, output_full
by resource. Missing-input/deposit required is at most 10000 and available is
strictly less than required. Explained `facility_blocked` requires at least one
actual reason; other event kinds cannot carry blocked reasons. Up to 64 affected
IDs are strictly ascending and unique. No freeform motive or guessed actor is
permitted. Construction/hull completion may name the actual created facility/hull
and paid job owner; hull completion may identify its affected fleet.

The complete sidecar is capped at 2 MiB. Collector pre-admission serializes one
bounded entry, accounts exact entry bytes/commas and reserves 300 bytes for the
maximum tick, digest strings and document framing. Rejection occurs before the
ordinary event append; existing unexplained `record` semantics remain unchanged.
Sealing checks the entire collector against ordinary ordinals/kinds/provenances,
then binds final event IDs. Verification checks ordinary integrity, tick, state,
receipt, every ordinal/ID, facts ordering/bounds and sidecar integrity before
presentation publishes any cache change. Sidecar failure must preserve the last
valid authority boundary. This is integrity binding, not proof of a third-party
writer's truthfulness; trusted authority producers supply the actual facts.

The independent Python fixture `living-v2-additions/event-facts.json` frames an
ordinary receipt and its event IDs from the existing domains, then independently
computes complete explanation bytes and the new-domain digest. Existing corpus
bytes remain unchanged. Chronicle's existing `record` remains compatible;
`record_with_facts` validates the entire pair before publication, provides honest
actor/entity follow only for explained events, and aggregates only equal facts.
Summary labels remain bounded to 2048 printable ASCII bytes; exact receipt/event
references remain available for detailed replay inspection.

### 10.1 Approved successful facility-recipe event

The additive ordinary event kind is `FacilityProduced`, canonical tag
`facility_produced`. Its provenance is `Autonomous { phase: RunProduction }`
(phase ordinal 5). It is emitted once for an actual successful enabled facility
recipe, after all input/deposit/storage checks and the complete resource
application succeed. Blocked/disabled attempts retain `facility_blocked`; no
successful event or output quantity may be fabricated for them. Hub production
is not covered by this tag and must not be attributed to a facility.

Its facts may name the actual owner and facility, actual input/output resource
amounts, and a final sparse `deposit_input` field. The optional field is omitted
when absent, preserving all previously approved event-facts bytes. When present
it is exactly `{world, units}`: the actual world-local aggregate deposit draw,
with an existing entity identity and canonical u32 units in 1..10000. It is
permitted only for `facility_produced`. No individual deposit identity is inferred
from an aggregate draw; future richer provenance requires its own reviewed shape.
Explicit null is noncanonical because the absent field must be omitted.
`production-facts.json` independently pins complete new-tag ordinary framing,
event identity and the resulting sidecar/draw bytes and integrity.

Existing ordinary kind tags, state bytes, state digest domains, and all original
independent corpus entries remain unchanged. **Historical receipt compatibility
is narrower:** adding successful events changes receipt payloads, event ordinals,
receipt/event identities and decision sidecar bindings at boundaries where the
new event is emitted. An earlier stored ordinary receipt remains self-verifiable,
but replay under the new producer cannot claim identical historical receipt bytes
at those boundaries. Canonical state archives retain their state/history identity
because receipt/event identities do not feed authoritative state. Native/wasm
receipt parity requires both runtimes to use the same new producer. This approved
additive distinction must be kept explicit in qualification evidence.

### Approved route adjustment decision witness

The existing `LivingIntentActionV2::Route` remains tag `route`, action ordinal 2, with ordered fields `source`, `destination`, `resource`. Append `existing_route: Option<LivingEntityIdV2>` as its final field, omitted when absent. `None` retains every previous establishment encoding. `Some(id)` names the exact existing route adjusted by this proposal; source/destination/resource describe its resulting configuration. Explicit JSON null is noncanonical. This additive witness changes only decisions that record an adjustment and contributes normally to the existing decision-receipt integrity domain.

A validated current source owner may adjust a captured/suspended route even when the stored route's old source owner differs; acceptance explicitly refreshes participants and permissions. Current receiver ownership, Internal/Trade/Aid permission, positive batch, source reserve and fixed cadence ten are revalidated from the shared snapshot. Adjustment preserves the existing `next_due` clock. Already dispatched shipments retain immutable dispatch owner, recipient, kind and cargo. Merely observing a captured route never resumes it. This implements approved rules sections 5/7; it adds no new gameplay or distance rule.

Independent witnesses `decision-route.json` and `decision-route-adjust.json` derive ordered payload bytes and SHA-256 from Python specification formulas. Original corpus verification ran before and after this addition without rewriting that corpus. The Rust golden verifies omitted/Some encodings, unchanged ordinal, explicit-null refusal and modified-ID integrity refusal.
