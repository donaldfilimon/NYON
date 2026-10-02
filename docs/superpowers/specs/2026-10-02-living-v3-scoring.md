# Living V3 autonomous scoring and reservation proposal

Date: 2026-10-02. Status: **PROPOSED numerical design; independent review and vectors required**.
Scope: October review gap3, integrated into the eight-document Proposed package
on 2026-10-02. No V2 rule, codec, vector or product source is changed.

**User-selected constraint:** imminent Food/energy recovery has strict priority
over discretionary activity. Every other weight, horizon interpretation, tie,
category, policy fallback and coordination rule below is a new Proposed choice.
Numerical examples are design arithmetic, not executed simulation witnesses.

Source basis: society §§Needs, growth and workforce, Research, Autonomous economy
and settlement; ecology §§Boundary order, World environment, Autonomous adaptation;
diplomacy §§Offers, Alliance intelligence, War objectives, Fleet logistics;
complete-schema §§Capacities, Command additions, Diplomacy lifecycle, Events,
Identity. Those documents remain Proposed. The October review explicitly says
“recovery outranks” does not rank competing recovery choices (review lines32–34).

## 1. Inputs, clocks and counterfactual forecasts

At a decision boundary T divisible by50, capture one immutable information-limited
planning snapshot after phase7 observations and before any phase8 negotiation.
At boundaries also divisible by100, phase8 and phase9 use that SAME capture.
No actor scores against another actor's later accepted resource reservations.
The capture's exact SnapshotReference identifies phase7 and the committed creator
cursor/catalog/branch/tick. This common capture is a Proposed integration choice;
phase8 negotiation still executes before phase9 intents and phase10 dispatch.
New accepted access/supply/defense is unavailable to this boundary's actor scoring
until a subsequent capture; phase10 still obeys actual newly committed permissions.

Each actor receives exactly the permitted snapshot declared in the schema.
Own colonies/worlds/facilities/jobs/fleets, own research/policy, known lanes,
observations/demands/commitments and exact known paid manifests may contribute.
A known manifest means one selected by Actor32 own_shipments/incoming_shipments:
dispatch_owner=A, or intended_receiver=A OR current destination World.owner=A.
Deduplicate the own/incoming union by immutable ID; no unrelated rival root scan. Missing/stale enemy inventory, research,
HP, supply or routes contributes NO inferred value, never fabricated zero.
A copied observation retains its original tick; attack eligibility requires age
<=300 as diplomacy specifies. No actor predicts an unannounced seeded flare by
reading the genesis seed; only already-known warning/start/end records apply.

For actor A, B(A) is a deterministic200-boundary counterfactual from T+1 through
T+200 inclusive, with no new autonomous actions. Retain already-paid jobs, standing
orders and known immutable manifests. Apply exact society/ecology phase order,
clocks, integer recipe/growth/capacity/staff rules, known scheduled hazards and
current observations and permitted known_hazards/known_stellar_events; future
unknown rival interventions/actions are absent. Exact target/interval/factor/
source selection is complete-schema §Integrated correction I1. A previously
observed event is a counterfactual fact even after an unseen edit; do not consult
live root events or hard-code creator-editable freight factors.
Supported own transitions are simulated by the conditional operator below.
Unknown claim/combat/occupation/storage outcomes hold dependent transitions while
retaining exact own state under its explicit scenario; no promise of full unknown
interaction simulation is made. For foreign territory use authorized observation
predicates, never unknown stock, HP, activity, population or future diplomacy. No projection assumes a
proposed offer will be accepted or a standing contract will deliver unmanifested
cargo. Known actual manifests may arrive under their recorded quantities, timings
and observed permission/disposition; an unknown future capture is not predicted.
Every projected result is labelled a counterfactual, not a guarantee.

For candidate C, F(A,C) repeats the SAME forecast with only C inserted at its
prescribed acceptance/departure boundary and its real costs/reservations. A new
standing order cannot depart at T; use T+1 earliest. Jobs complete at T+duration,
and first recipes at completion+period. Treaty proposals create an offer only;
any future acceptance remains unknown. Research points require actual eligible
staffed labs, local inputs and their due clocks. Ark passengers are transferred,
not created; boarding cannot evade the same100-person/200-origin rules.
Forecast copying, every nested entity/recipe/period and every hash use the same
1..1024 visit budget. Never count one200-boundary forecast as one visit.

For each colony's due Needs result, resource exposure is
ceil(population_before * shortage / max(1,resource_need)). Zero need gives exposure0.
Sum Food exposure and energy exposure separately, then together. Exposure can
count the same population twice; it is a ranking measure, not casualty evidence.
FirstExposure is the sum at each colony's FIRST due needs boundary in (T,T+100].
AllExposure sums every due result in (T,T+200]. A first boundary's shortage is
imminent iff either resource shortage>0; its deadline is that boundary's tick.
D(A) is the earliest such baseline tick, or absent. Loss is the actual number of
people lost to Food/energy shortage causes in this owned-state counterfactual;
capacity-only decline is excluded. ImminentLoss sums those same shortage-caused
losses only at due needs results in (T,T+100]; losses after that interval cannot
qualify imminent recovery. Attribute a loss to the exact colony, needs tick and
shortage cause emitted by the demographic rule, never to an earlier warning.
Generate totals only for A's own colonies.

Projection deltas are B minus F for exposure/loss and F minus B for positive
outputs/capacity/research. Deltas may be negative; never clamp away harm.
Output counters count actual produced units, not inventory changes or freight
movement, so transporting the same Food cannot earn production credit twice.
All quantities derive from permitted facts; omniscient actor scores are invalid.

## Conditional private B/F operator (Proposed)

This section defines supported projection, held outcomes and accounting; all existing numerical score/category/tie tables remain unchanged. Farm evaluated-prefix, amount domains and nominal-before-stock zero freight are exactly complete-schema §§ResourceAmount domains and empty-list meanings, Exact zero-freight transition, Arithmetic funding and domain-check ownership. No copied zero modifier legalizes an earlier guard. Public0 is invalid; internal allowance0 is continuation.

## Captured evidence and projection boundary

| Meaning | Permitted use |
| --- | --- |
| Exact own fact | Copy original inventory, population, hull HP/passengers/credits, jobs, orders, manifests and clocks; evaluate guards against supported own projected changes |
| Timestamped observation | Retain source tick and source identity; use only the predicate authorized for that operation; age-valid attack information does not become fresh freight ownership |
| Unresolved outcome | Hold the dependent transition; absence is never zero stock/HP, no hostile force or no competing claimant |
| Authority result | Establish actual accepted transfer, damage, claimant, capture or fulfillment only through authority/replay |

Capture at T once and project T+1 through T+200 inclusive. Use existing integer
arithmetic, phase order, recipe/needs/supply clocks, public intervals and geometry.
Do not refresh observations, generate unknown rival activity, discover new facts,
read the seed for claim ranks/events, or synthesize treaty/peace acceptance. Known
scheduled effects expire by their selected end-exclusive intervals. Event selection
retains the reviewed scope/identity/newest-before-expiry ordering. B and F share all
source records, event selection and this operator, even when their hold sets differ.

Paid provenance is immutable input. Projected location, order, dispatch due clocks,
claim eligibility and cargo return/retry state are private working records, stored
separately. Never mutate source manifests to represent a predicted return/wait, or
export a working projection as canonical State/history. Paid cost is incurred once;
a withheld unpaid operation pays zero, creates no refund and reserves no new asset.

For counterfactual supported allocations retain the frozen AUTO_ENTITY byte formula,
namespace/actor/kind/subject fields and successful-allocation ordinals. Use this
run's captured phase7 permitted_snapshot_digest for future allocations, with their
actual projected tick/phase; use already-public root/active branch identifiers from
the document identity context, never State/revision digest or a future root scan.
This constant-capture choice is Proposed forecast behavior, not a prediction of
actual future issued IDs. Order working entities by their resulting decoded IDs,
as existing local rules do; never use a guessed ID0, root-wide rival ordinal or
unordered insertion. No canonical command/entity identity formula changes. Future
projection allocations cannot be published into authority history.

## F1: capacity knowledge, legal formation and reservations

### Knowledge account

For each required global capacity account g, use a private record
`CapacityKnowledge { account:g, limit:L, known_tokens:Sorted<Token>, hidden_upper:H }`.
A Token is one of the exact tagged actual/reservation keys in the finite registry
below, never an untyped ID standing in for a composite record key. It is not a public global count. Initialize
K as the number of UNIQUE known tokens and H=L-K, checked; the possible occupancy
interval is [K,K+H]. If K=L, the roster proves exact fullness. If K<L, omitted
rivals are not zero and the upper occupancy is still L. An input K>L faults this
projection; never truncate or reinterpret duplicate identities as spare capacity.

For shipments, merge own/incoming sorted arrays by immutable ID before counting or
stepping. The same ID in both roles is one token, one arrival, one stock transfer
and one release; inconsistent bytes for a repeated ID fault the unpublished
projection. Retain role flags for visibility/provenance only. Do the analogous
identity dedup for any actual row referenced by a reservation: one hull/job target
cannot be counted twice merely through two relations.

Within this scenario hidden rows/reservations do not create new future activity;
their occupancy may decline in reality, but H is never lowered for an unseen
release. For a supported removal of one known token, remove it and decrement K;
H stays unchanged. Hence upper occupancy falls by exactly one. For a supported new
allocation of n unreserved tokens, require checked K+H+n<=L before any debit;
insert n tokens and increase K. Retained cargo, a return leg, an unknown death,
an unknown settlement award and a capacity hold remove no token. A conversion
between reserved and actual forms in the SAME account replaces its key atomically,
keeping K unchanged. A reservation cannot coexist with its consumed actual token.

Examples: 4096 unique known shipments and one own unload prove 4095 afterward.
A route may allocate one shipment and restores upper occupancy to4096. Two selected
shipments prove only lower2/upper4096, not two globally occupied slots. If one of
those two independently unloads, lower1/upper4095 proves one slot even though the
hidden contribution remains bounded by4094; this is conditional on no new hidden
future activity, not live global availability. A foreign held arrival proves no
release. This makes the sufficiency predicate independent of fixture root counts.

### Account and conversion table

Use the EXISTING numerical limits; no new slot/batch cap is chosen here.

| Account | Known tokens at capture / admitted C | Conversion and new-slot obligation |
| --- | --- | --- |
| shipments,4096 | Unique actual own/incoming manifests; any explicitly already-paid manifest added by C | Return/retry keeps its token. Successful own unload removes one. Standing future dispatch always needs one new token; a route or temporary funding hold promises no future manifest slot |
| facilities,2048 | Selected completed own facilities PLUS one promised facility token per paid own construction job | Accepted Living rules §Capacities reserves global entity capacity through completion. Completion converts its existing promise to a facility, with no new global slot. Supported scrap releases exactly one actual token; replacement consumes that released token as its named job promise. Cancellation releases only its promised token; unrelated jobs cannot spend it |
| construction_jobs,4096 | Own paid construction rows | Job admission needs a new job token; supported completion/cancellation removes it. Payment and local industrial reservation do not grant additional job/facility slots to unrelated future work |
| hulls,2048 | Actual live own hull IDs PLUS one promised hull token per paid own hull job | The schema explicitly reserves global hull capacity for a paid hull job. Completion converts that promise to its actual hull, with no new hull slot. Cancellation releases its promise; held combat/ark award never releases an actual hull |
| hull_jobs,512 | Paid own job IDs | Admission consumes a new job token; supported completion/cancellation releases it |
| fleets,256 | Actual own fleet IDs PLUS each distinct new-fleet reservation of paid hull jobs | A new-fleet job consumes its promised fleet token on first fleet creation, without a second global slot. Later hulls targeting that SAME fleet share its fleet token but retain separate hull promises. Existing target fleets add no new global fleet reservation. Supported merge/removal releases only an actually emptied fleet |
| routes,2048 | Actual own route IDs | Adjusting an existing route consumes no new global route token. New-route C needs one conditional admission token; it promises no recurring shipment capacity |
| treaties,256 combined agreements+commitments | Selected Agreement keys UNION derived future Truce keys of selected Wars, plus Commitment IDs with DISTINCT tags | They share one256 account. Exact expiry/removal releases only its own tagged key. A conditionally admitted Agreement C inserts/replaces only its exact key; no unaccepted Offer creates a treaty/commitment. A known mandatory war-expiry Truce is handled below, without granting Trade/access/supply |
| offers,64 globally / one outgoing per actor | Selected Offer IDs; admitted Offer C's exact issued ID | New C consumes one actual Offer token until recorded expiry or supported removal; replacing/renewing the same ID adds no slot. Future acceptance/refusal is unknown and does not release it merely because a response boundary passed |
| wars,120 | Selected ascending public War pair keys; admitted War C's exact pair | A new admitted War consumes one War token; same existing pair is not a second token. Keep it through recorded end_tick, regardless of declaration force-hold release. Expiry removes the War token at phase2 and cancels its future bound operations; unknown victory/peace never removes it early |
| settlement_claims,8192 | Own selected (world,civilization) keys | Existing own claim token stays through held close/award and departure. Supported phase4 armed/owned-world clearing releases exactly its key. Timing-unknown new arrivals create metadata only, not a claim row/allocation. Multiple own arks at the world do not add claim tokens |
| occupations,512 | Own selected world keys | Held progress keeps its one world token. Proven own claimant reset removes that key at phase4. No foreign progress row or new occupation success/allocation is synthesized |
| defense_requests,256 / one per commitment | Selected exact DefenseRequest IDs | Retain captured dispositions/observed threat; recorded expiry or a supported recorded termination releases its ID. No new defense request, rival response, fulfillment or threat refresh is forecast. C Defend/War uses existing records, not a new request token |
| demand_requests,1024 / one per(actor,world,resource) | Selected exact DemandKey triples | Existing positive requests support opportunity ranking through their own interval; expiry releases their triple at the ordinary lifecycle phase. No new/refresh request row is projected. Actual free request_supply posting remains outside B/F; its ordinary authority behavior is unchanged |

### Finite Account and tagged Token registry (R1)

Account is the PRIVATE closed enum in this EXACT declaration/comparison order:
`shipments, facilities, construction_jobs, hulls, hull_jobs, fleets, routes,
treaties, offers, wars, settlement_claims, occupations, defense_requests,
demand_requests`. Limits in that order are4096,2048,4096,2048,512,256,2048,256,64,
120,8192,512,256,1024. `treaties` is the single combined agreements+commitments
account, not either collection's independent allowance. Local industrial/shipyard/
fleet-hull/passenger/funding accounts retain their already-stated exact limits and
are not new global Account variants. There is no `other`/string/open account tag.

Actual Token variants, in declaration/comparison order, are:
`shipment{id}, facility{id}, construction_job{id}, hull{id}, hull_job{id},
fleet{id}, route{id}, agreement{participant_a,participant_b,kind}, commitment{id},
offer{id}, war{participant_a,participant_b}, settlement_claim{world,civilization},
occupation{world}, defense_request{id}, demand_request{actor,world,resource}`.
Each ID is the decoded existing typed ID; pair IDs use the frozen ascending pair
order; Agreement.kind and Demand.resource use their frozen enum declaration order.
Compare tag first, then listed fields in order, never display text or an ID cast.
A Commitment ID equal in raw bytes to a party ID does not alias an Agreement key;
War pair does not alias Agreement(pair,kind); an occupation world does not alias a
settlement claim(world,A). Registry mappings are one-to-one: all actual variants
map to their named Account, except agreement/commitment both map to treaties.

Reservation Token variants follow ALL actual variants, in this order:
`promised_facility{construction_job}, promised_hull{hull_job},
promised_fleet{hull_job}`. The payload is the exact paid job's typed issued ID.
Each target_fleet=null hull job has its own promised_fleet keyed by that job;
targeting an already actual fleet adds no fleet token and uses its existing exact
local hull reservation. Conversion replaces the promise with its actual entity key
in the same account. All jobs/promise binding bytes must agree with capture/C.
A conditional C's new actual row uses its exact successful prospective issued ID
or composite key under the existing identity rules; during its unpublished atomic
admission the token is staged, not an additional generic C token counted beside
that row. No same-account key is consumed twice, no tag is inferred from raw bytes,
and declaration funding/force locks are LOCAL account locks, not global War tokens.

Known War expiry at actual phase2 converts its derived reserved agreement{participant_a,participant_b,kind=truce} key atomically. Initialize the treaties account's known tokens as selected AgreementKeys UNION future Truce keys from selected own-party Wars, plus distinct CommitmentIDs; identical agreement keys count once. A valid captured surviving War proves its key, so natural expiry never waits for hidden treaty capacity or fabricates a pending Truce. Remove one War token at end_tick, retain the treaty unit as an actual Truce on [end_tick,checked(end_tick+300)), then release that treaty key at ordinary expiry. Apply the exact same-key interval and creator/cancellation distinctions in complete-schema Derived future Truce capacity. Unknown early peace, new admissions or unseen creator cancellation remain unsupported; no rival future actions are predicted.

Other frozen top-level collections acquire NO new projected rows in this operator:
systems/stars/worlds/lanes/civilizations/deposits/colonies/relations/observations/
hazards/stellar_events/shared_intelligence and document branches/revisions/bookmarks/
experiments. They therefore have no global allocation Account here. Existing exact
own scalar stock/population/research/workforce and eligible known relation counters
may update by already-supported rules; captured observations/schedules retain source
bytes/expiry. No colony/foreign relation/observation is created for held settlement,
capture, combat or guessed delivery. Nested research sets/points use their existing
catalog-bound64/local validation and counted visits, not an omitted global row slot.
Creator/history publication is outside the private operator. No free request/defense
posting or rival negotiation is added to B/F simply to exercise an unused account.


Accepting a building OR hull job reserves its global entity capacity and destination
slot through completion (accepted Living rules §Capacities, line66). A paid job is
therefore evidence of its OWN promise, not evidence that other global slots are
free. Joint actual+reserved counts use each promised entity account's existing
limit. A known job token is separate in the construction/hull-job collection, so
conversion consumes its promised facility/hull token and releases its job token;
it does not free the converted facility/hull token. New-fleet promises follow the
same rule. Reservations/capacities remain unpublished. Missing required target or
inconsistent token binding faults the unpublished projection; no root lookup,
second allocation, job delay or invented refund repairs an invalid selected job.
This preserves paid completion promises rather than withholding a valid job merely
because an unrelated current root count is hidden.

Local accounts are exact from complete own records: completed facilities plus paid
construction reservations use the eight industrial slots; a hull job uses shipyard
concurrency, its hull/job promises and target fleet's actual+reserved hull count
(max16), not an industrial slot. Completion transfers the local reservation to
occupancy. Cancellation removes only that job's promises and grants its actual
half-refund subject to exact storage. A known scrap/refund is atomic with replacement
and its earmarked released facility token. No cancel/scrap beyond C or an already
captured scheduled operation is inserted just to free capacity.

### Actor formation versus conditional F admission

Before scoring/trace insertion, apply exact catalog, structural, path, ownership,
local funds/reserve, local capacity, intelligence and treaty eligibility. A global
account excludes a candidate only if its required tokens are **provably impossible**
from permitted known tokens AFTER its exact legal atomic scrap/releases
(K_after_release+n>L), or an exact selected reservation conflicts. Legal replacement
therefore tests its actual released token, not a full-slot no-scrap alternative.
Unknown H is not a zero and does not exclude that identity from one implementation's
trace while another scores it. This explicit Proposed interpretation refines frozen
scoring's generic “capacity before scoring”: actor eligibility is information-limited;
actual central global scarcity remains a later authority admission check/reason.

F conditions on C's successful admission. For each account/reservation C explicitly
needs at admission, apply its exact permitted atomic releases/refunds, then set
H'=min(H,L-K_after_release-n) and insert its n reserved tokens atomically;
if K_after_release+n>L, C was not eligible and no F is publishable. This tighter bound expresses
only C's assumed successful allocation. It does not disclose an actual root count
or tell B that rivals were absent. K_after_release+n+H' still may equal L, so no unrelated free
future slot is inferred. Local paid debits and reservation conflicts apply once.

A C reservation lives until its specified conversion, cancellation, rejection or
boundary publication. Consumable reservations convert atomically into exactly the
promised actual row; rejected/unused C reservations release only their own tokens.
No successful-F rejection path manufactures a refund: an unsuccessful candidate
has no F admission and central rejection remains outside that hypothetical run.
A declaration force/account hold releases at publication, pays0, grants no return
credit and never funds a subsequent departure. Route next-batch funding escrow also
ends at publication, before T+1: it promises neither T+1 stock nor a shipment slot.
Subsequent dispatch needs fresh projected source funding and a new proven global
shipment slot. A paid job promise lasts until supported conversion/cancellation;
an unrelated standing operation cannot consume it. Stable phase/subject-ID order
selects the first consumer of a newly proven free token.

## F2: endpoint knowledge and complete cargo disposition

At capture, define OwnerKnowledge(world) in this exact precedence:

1. `own_exact(A)`: world and account are in consistent exact own-world/colony input.
   Retain owner A throughout this projection except a supported own operation.
2. `direct_at_capture(owner:Option<ID>)`: the actor's retained DIRECT Observation
   for this world has observed_tick==T. Retain this owner, including null, throughout
   the projection as a declared no-unmodeled-ownership-change condition.
3. `unresolved`: every other source, including a direct observation at T-1, any
   older direct observation, shared intelligence alone, or dispatch/claim metadata.

Own exact records win over stale observations. Conflicting exact own/current direct
records fault the projection. Direct observations have the existing unique actor/
world key; no world-level merge from another observer supplies freshness. A shared
record can still rank/authorize an attack by its published age/source rules, but
cannot become freight current-owner proof. “Fresh” means case2 exactly, not age<=300
attack eligibility (or a new freight age threshold). No new observation is acquired
inside B/F, so case3 does not become fresh by waiting. Case2 does not become stale
inside this explicitly retained scenario simply because projected time advances.

Public permission uses only captured agreements/wars to which A is party and their
exact start_tick<=t<end_tick intervals. Known expiry/truce and explicit captured
order cancellations apply at their ordinary phases; no renewal/acceptance or
objective-success peace is predicted. A newly hypothetical C War supplies its own
conditionally admitted public relationship only, no rival combat ledger. Treaties
alone grant no recipient inventory access. The intended receiver/dispatch owner
in a manifest supplies provenance, not the current endpoint owner.

At EACH nominal arrival and subsequent retry, choose disposition before storage:

| Cargo state / supported endpoint | Disposition and account choice |
| --- | --- |
| Not yet arrived on recorded/projected leg | Keep travelling; no hold/retry/unload |
| Already returning, including returning_to_source=true on a canonical waiting source return | Use ORIGINAL source endpoint. Ignore A/B war for bouncing; choose its supported current owner for capture/delivery classification, then storage. Never initiate a second return |
| Outbound endpoint owner unresolved | Hold endpoint_disposition; do not infer capture, delivery, return or storage, even if an intended receiver or old claim is named |
| Outbound supported owner differs from intended receiver, including null | Choose captured-owner/unowned delivery before war-return. If that owner is A and exact own account exists, atomically unload there; otherwise hold recipient_storage. No A inventory/aid credit from a foreign/unowned account |
| Outbound supported owner==intended receiver and captured public war active at this arrivals phase | Begin ONE paid return immediately, regardless of destination storage. Keep the same manifest token and quantity; paid cost0. Projected return duration d=checked(original arrival_tick-original departure_tick), require d>0; set return_departure=t and return_arrival=checked(t+d). Original source/destination/ticks/provenance remain immutable |
| Outbound supported owner==intended receiver, no active public war | Choose normal delivery; exact own account permits atomic unload or exact space wait; otherwise hold recipient_storage. Treaty expiry alone is not a return guard |
| Returning source owner unresolved | Hold endpoint_disposition at source; retain cargo/token, with return_used=true. No bounce |
| Returning source supported but account foreign/unowned | Hold recipient_storage, disclose selected owner/disposition; retain token, no bounce/credit |
| Returning source owner==A with exact own account | Atomically unload iff checked stock+units<=the existing resource storage limit, else known storage wait/retry. Release one unique token only on transfer, never credit produced units |

A supported owner change to A does not create an account: exact own account access
is separately required. In particular, merely observing a captured foreign world
or selecting an occupation does not supply its inventory. Conversely an A-owned
OCCUPIED colony already has its exact account: under retained-own-ownership/no-damage
conditions, incoming Food DOES unload when storage fits, before its same-boundary
Needs result. A combat/occupation hold does not override that conditional local
account. Its dependency is disclosed; there is no predicted new colony/capture
resource award. Changing hidden rival HP changes neither this unload nor its actor
trace, although actual authority can capture/damage the colony before later needs.

Use private CargoProjection keyed by unique manifest ID:
`leg={outbound|returning}`, `return_used:bool`, `effective_arrival:Tick`,
`arrived:bool`, `storage_wait:bool`, plus optional checked return_departure/arrival.
Initialize from exact captured disposition/returning_to_source; source capture can
already be WaitingForStorage. The first due scan sets arrived=true once. A return
sets leg=returning, return_used=true, arrived=false, storage_wait=false and its new
private effective arrival; it schedules no second outbound arrival. A held delivery
sets arrived=true and retains its current leg/arrival; retry EVERY later arrivals
phase independent of a one-time arrival notification. Exact own full storage sets
storage_wait=true; foreign unknown storage holds are not canonical WaitingForStorage.
On successful unload remove cargo, its token and all its holds once. A retry never
spends again or emits a second arrival/authority event. The source record bytes do
not change. Invalid duration or checked-clock overflow faults unpublished output.

## F3: held claim windows, order linkage and conflict dependencies

### Private claim state

Use one `ClaimWindowProjection` per world, with original captured close if present,
otherwise `close={unknown}`; `status={before_close|award_held|timing_unknown|cleared}`;
known OWN captured claim keys; and members bound to real own fleet/hull IDs. Each member
has its supported actual arrival tick, qualifying Settle order linkage, current
world and `eligible_here:bool`. Passenger quantity/HP/credit remain in the hull,
not copied into a second resource pool. Multiple own arks share one civilization
claim key; they are linked individually so departure of one is not departure of all.

At capture, a selected OWN claim establishes its original close; the unique
(world,A) key permits only one such row. Never recompute arrival+50. Associate that
existing claim with every live own catalog-role ark hull Docked at its world,
regardless of its current order: existing claimant presence is not the new-claim
Settle-order guard under the existing claimant-presence predicate. Retain a separate order linkage
for NEW participation. No competitor row/absence or rank is selected. If no selected claim
exists, an own supported settling arrival creates ONLY a timing_unknown private
window. **Do not invent arrival+50 as its actual close:** unseen earlier paid rivals
may already have opened a different window. Record that ark's arrival and hold
`claim_outcome/claim_window_timing`; close remains unknown even if own arrival is
T+5. This explicitly resolves the unobserved-target case without asserting an empty
universe or pretending every known ark supplies a canonical new claim row.

For a known close c, a new supported own Settle arrival at t<c joins membership
and the existing own civilization key, requiring supported unowned endpoint,
Docked fleet at that world, live catalog-role ark hull, and current Settle order
with target_world equal to this world. Link all such own hulls; passenger quantities
remain exact and are never assigned100 merely because an ark is eligible. The
known-close own-only case already has its one (world,A) reservation; joining a
second ark consumes no additional claim token. No foreign row supplies a new
known-close claim for A. The timing_unknown case never issues an assumed canonical
claim token/window simply because an ark arrived. At t==c or t>c, do not join the closing/held award: retain
an ineligible late-arrival link and no new claim token/window. At t>=c set status
award_held; never arbitrate, consume an ark, grant20energy/20ore, create hub/colony,
start colony Needs or free hull/fleet tokens. No seed/rank or a single known own
claimant releases that hold. Timing_unknown windows retain arrival links without
issuing claim tokens; they never pick a close or score settlement completion.

An independently supported own departure is allowed despite an award hold, under
all ordinary order/path/funding/credit/supply/treaty guards. It flips each departing
member's eligible_here=false and updates location/order normally; it neither
consumes passengers nor declares Claim lost. Keep the captured claim key/capacity
reservation until a supported clearing predicate; an eligible-ark link alone is
not a claim award/removal proof. Before-close arrival of a previously linked own
ark restores its existing claimant-presence eligibility while Docked/live, without
inventing a new Settle order; NEW links still require the arrival Settle guard.
At/past-close arrivals do not join/reopen the held award. A Move/Return order edit
clears its OLD Settle-order linkage but does not erase physical existing-claim
presence while the ark remains Docked; actual departure clears eligible_here. No hidden future order edit is synthesized. A captured Docked claimant can already
have a Return/Move order with literal not_before_tick AFTER its captured close;
that order waits through the held award and then departs when supported. C is
inserted only at its prescribed acceptance time, never as an extra future action.

A known own armed presence is a sufficient settlement-clearing predicate even
without knowing rival forces. A supported owned-world predicate is also sufficient.
At that conflict phase clear the private window/members, release ONLY its known
claim reservation tokens, and retire its holds incrementally. Neither predicate
consumes an ark or grants a colony. A later eligible arrival can create a fresh
private timing_unknown entry; no new exact50-tick window is asserted without a
new captured claim-close operand. Absence of observed rivals, own departure alone,
or passage past close is not a clearing proof. At horizon end retire all windows,
including closed sentinels with no eligible members; maximum one per public world.

### Combat and occupation dependencies

Unresolved combat affects every own hull/fleet/battery/colony at an interaction
world, not just the attacking fleet. Mark a world uncertain if a selected observed
hostile force exists there, a captured OWN occupation/claim interaction names it,
or an own current Attack/Blockade/Settle operation reaches a foreign/unowned endpoint
whose required combat/absence predicates are unsupported. Bind combat holds to EACH
affected own fleet and owned colony at that world; bind unresolved new enemy-world
ownership gain to its fleet operation/window. A captured pre-existing damaged own
battery/hull remains exact; unknown enemy HP/supply are neither zero nor full HP.
A known escort count/strength can rank offensive objectives but cannot compute a
combat damage ledger, future destroyed row, occupation win or safe-dock witness.

Retain known own HP, population and owner under the declared scenario. Continue
own-account recipes, repair, Needs, research and supply when their ordinary OWN
location, staffing, clock, reserve and physical-input guards are exact. A successful
supply attempt records exact payment/deadline in working state; failure keeps the
last successful deadline. Being an endpoint of a held attack grants no enemy refuel,
repair, return credit or inventory access. An operation whose guard requires absence
of foreign defenders, a claim winner, new ownership, safe military fulfillment or
unbound public permission stays held. No ordinary operation gains a missing guard
merely because own HP was preserved conditionally.

Paid travel arrives at its recorded tick; mutable projected location changes to
Docked and the actual paid leg completes. Preserve HP as the scenario condition,
not a claim that combat allowed safe arrival. Apply inherited order completion:
Move/Return clear on arrival; Attack/Settle clear iff their target differs from
this arrived endpoint, otherwise remain pending; Explore/Defend clear iff target
is absent or equals this endpoint. Preserve other order fields/objective linkage
when retained. Blockade retains its separately bound standing operation and never
completes an objective from arrival alone. No new order completion clock/witness
is invented. Known war expiry cancels future bound orders/Blockade at the specified
diplomacy phase but never teleports/refunds an already-paid travelling leg.

For automatic noncombatant retreat, known hostile-force presence plus complete own
unescorted roster, paid credits, exact owned return destinations, permitted path
and all ordinary retreat guards support the already-defined phase4 immediate
return. Choose minimum duration, then world ID, then lexicographic lane path. If
hostile presence/escort guard is unsupported, hold combat_outcome/retreat_guard;
never manufacture a retreat by setting enemy forces to zero. Ordinary Move/Return
at phase10 requires Docked, a nonnull existing order with not_before_tick<=t,
nonnull target different from current world, no incompatible paid hull-delivery
reservation, and a supported permitted path. Return requires an exact owned target
and a credit on every actual hull, consumed once. A new non-Return launch from an
owned account pays the unchanged per-hull launch cost (including missing-credit
surcharge and published future-cost technology factor); away from owned accounts
it cannot start a fresh excursion unless the explicit dormant-ark exception's
known guards apply. A missing predicate holds and changes no credits/location;
a proven ordinary failure consumes nothing. Original not_before_tick is retained
on blocked attempts, not shifted to t+1; successful dispatch stores its actual
paid projected leg/selected path. A new C order obeys the original
no-earlier-than-next-boundary rule; there is no added safety guard or a special early escape. Neither grants survival
or fulfillment credit. Reorganizations whose anti-combat guard is unresolved hold.

Never increment occupation through an unproven foreign defense/contest condition.
Retain selected OWN claimant progress and add occupation_transfer hold. **Proven own-claimant
reset:** for claimant A, full own roster can prove no live own escort remains
Docked at its occupation world after a supported departure/removal; then, at the ordinary occupation phase4, remove that private progress row,
release ONLY its known occupation token and its progress hold. A phase10 departure
does not rerun earlier phase4: the reset occurs at NEXT boundary phase4. A phase4
supported removal before occupation is visible to that same phase4 reset. This is a necessary failed claimant-presence guard, not a predicted battle.
A surviving own escort leaves progress held at its last captured value. For a
foreign claimant, there is NO selected exact progress row in the chosen input:
retain an interaction hold on the affected own colony/fleet, not a fabricated or
root-read foreign progress value. Own departure cannot prove that claimant absent.
A departing own defender does not automatically advance enemy progress. Known treaty/truce incompatibility is another exact reset, if every
required public operand is selected. An unknown guard never resets/increments.

## F4: literal clocks, suspension and retry order

At each dispatch phase traverse current own routes by increasing route ID. A route
is due iff old_next_due<=t. Process it ONCE this boundary, even if overdue by several
cadences; no while-loop catch-up. Before any source debit, compute
`new_next_due=checked_add(old_next_due,u64(cadence_ticks))`. Set next_due to that
value for EVERY due attempt, including suspended, unknown endpoint, expired trade,
no path, no funds/above-reserve batch, zero modifier batch or capacity failure.
Never use t+cadence. Overflow is a sticky projection Arithmetic failure, not a
hold, wrap, saturating tick or successful partial forecast. Undue routes keep both
next_due and suspended byte-identical.

After the checked due-clock computation, test **old_suspended FIRST**. If true,
keep suspended=true, complete this due attempt with zero debit/new cargo, and stop:
DO NOT read/check endpoint participants, path, permission, funding, modifiers or
capacity. No later uncertainty hold is created/refreshed. A previous endpoint or
capacity hold owned by this same route-attempt dependency is no longer live and is
retired under the ordinary bounded cursor; unrelated subject dependencies remain.
Do not preserve it “for explanation.” Undue routes do not attempt these guards.

Only when old_suspended=false, check source/destination participant proof in that
order. Source knowledge comes from F2 as well (a recorded source_owner alone does
not imply current exact ownership); destination follows F2. A proven mismatch
sets suspended=true and blocks dispatch; no later guard is visited. An unresolved
participant preserves old_suspended=false and records endpoint_disposition. A proven
match leaves suspension unchanged and proceeds. Expiration/re-formed agreement
never clears suspension. Only an explicit supported C adjustment with validated
current participants clears it before subsequent attempts.

Literal route guard order is: due check/checked clock; existing suspension;
source participant; destination participant; known path/route kind; public dispatch
permission; exact source reserve/funding; batch/selected modifier order; global
shipment slot. A proven earlier failure stops later reads/holds. Trade uses its
known end-exclusive agreement; Aid does not gain a Trade-agreement requirement.
Treaty/war expiration changes its ordinary permission guard, not ownership proof
or paid cargo disposition. Suspension check, any actual prior-hold retirement and
all other visited guards consume visits; skipped participant/other guards consume
NONE. Clock and failure/hold edits publish together only after that attempt's
bounded work finishes; polling/cancellation exposes no partially committed B/F.

Arrivals scan unique shipments in ID order. Nominal arrival due is
`effective_arrival<=t`; already-arrived or captured WaitingForStorage also retries
once at every arrivals phase. Re-evaluate F2 disposition before space on every
retry. Storage full does not shift arrival_tick. A just initiated return has
arrived=false and cannot unload until its new effective arrival. Fleet arrivals
are ID ordered, happen once, and update orders as F3 states; held combat/claims
cannot schedule duplicate arrivals. Job completion uses recorded completion_tick<=t and consumes its guaranteed paid
reservation once, with first recipe at checked(actual supported completion boundary+period); no second
charge or global allocation. Recipe/repair/supply/needs due attempts retain their
source-defined checked expressions and ordinary blocked-clock policy, never borrow
route's expression for a different clock. A completion whose required own target
guard is unresolved retains its original completion tick and paid promises, retries
once each production phase, and starts no recipe before supported completion. A
proven invalid target follows exact cancellation/refund/release; unknown future
combat alone never destroys that target under the declared no-damage condition.

## F5: closed private holds, bounds and scores

### Typed metadata

Use private, non-wire types:

- `HoldKind={claim_outcome,combat_outcome,occupation_transfer,recipient_storage,
  endpoint_disposition,global_capacity}` in that declaration order.
- `Subject={world(ID),colony(world),claim_window(world),occupation(world),war(pair),
  construction_job(ID),hull_job(ID),fleet(ID),route(ID),shipment(ID),
  candidate(identity_bytes)}` in that declaration order. Candidate bytes are the
  existing bounded identity, not a new arbitrary string. A held hull/battery
  dependency is recorded via its containing fleet/colony; their bounded live ID
  sets are the exact affected records, not a copied second HP ledger.
- `Missing={endpoint_owner,recipient_account,recipient_space,claim_window_timing,
  claim_rank_or_future_eligibility,combat_ledger,retreat_guard,occupation_guard,
  global_slot(account)}` in that declaration order. `account` uses exactly F1's
  closed global account tags, including inherited settlement_claims/occupations.
- `Binding={none,endpoint(world),window(world,close:Option<Tick>),
  occupation(world,claimant),account(account,subject)}`. Binding cannot name an
  unselected private row; claim close null means timing unknown, not zero.
- `Hold { kind,subject,phase,binding,missing,first_unresolved_tick,last_checked_tick }`.
  Key is (subject,kind), independent of retry tick/category. phase is deterministic
  from the subject/operation: cargo3, claim/combat/occupation/retreat4, completion5,
  order/route10, war-expiry2, candidate at its prescribed admission
  phase8 or9. Sorting for
  inspection is (phase,Subject tag/listed decoded key fields-or-identity bytes,HoldKind).

At each operation check predicates in literal phase/guard order. The first missing
predicate determines the primary HoldKind/Missing/Binding, so an outbound stale
owner plus unknown foreign stock records endpoint_disposition/endpoint_owner first;
it never guesses the later recipient. If an earlier guard is proven false, report
that known private failure and perform its ordinary clock update, without appending
holds for unvisited later guards. If guards become complete, release this operation's
hold and evaluate once. Proven unsupported combat/occupation dependencies can
coexist on different affected subjects. Never use arbitrary missing-category text.

Insertion creates one entry for its key with first_unresolved_tick=t. Retry retains
that first tick, updates last_checked_tick=t and deterministically replaces current
binding/missing/phase; it appends no history. Changing primary kind atomically
retires the old operation hold and inserts/reuses the new key; the two missing
conditions are not accumulated as two primary cargo explanations. When a key has
other still-live operation dependency of the same subject/kind, keep it, choosing
minimum (phase,Missing declaration ordinal,Binding tag/fields) and earliest first
tick across those CURRENT dependencies. No operation is decided by whichever poll
runs first. Resolution of one dependency cannot erase another live dependency.
Canonical before/after facts and receipts are never generated from these records.

### Static storage/work closure

Derive subject storage slots before starting B/F from captured unique subjects,
existing public world keys and C's bounded identity/reservations, then reserve
bounded slots for supported allocations within the existing collection limits.
Do not allocate an unbounded per-tick log. A shipment/hull/facility/job/route's
storage slot may be reused only after its old subject/holds are retired; its issued
ID is never recycled. At most one claim-window/occupation subject per world and one War interaction
subject per known War pair (max120);
affected fleet/colony records use their original bounded child sets. Each CURRENT
subject supports at most six hold entries, one per kind. Candidate holds cover
only the one C under evaluation, not every candidate previously scored.

Precisely, a conservative private subject bound is captured unique world keys
(max512) for each of world/colony/claim_window/occupation, plus slots for
construction_jobs4096, hull_jobs512, fleets256, routes2048, shipments4096,
war-pair subjects120, and one C;
H_max=6*(4*512+4096+512+256+2048+4096+120+1)=79062 entries. This is a DERIVED worst-case
metadata allocation bound, not a new gameplay cap or a disclosed global count.
For B omit C; for an instance allocate only checked actual captured+C domain sizes
plus the relevant existing maximum slots for future supported allocations. A
world not in public geometry is a missing-reference fault, not a new capacity.
Dependencies per subject are bounded by the existing own child/source maxima;
scan them under a cursor and retain only the one winning explanation per key.
No arbitrary vector of blockers or per-boundary copied hold graph is introduced.

Every source copy, merge comparison, guard, account token change, child visit,
retry, score record and deletion is counted under the existing1..1024 visit/chunk
budget; public requested0/>1024 rejects invalid_range with zero work. Internally exhausted remaining0 yields continuation,
not a different count/prediction. Store pending deterministic edits until the
operation is complete; do not debit on a partially checked guard or publish a
partial B/F pair. Checked size/key inconsistency, Arithmetic, missing required
reference or private output-capacity overflow becomes a typed sticky projection
failure. No silent truncation/score0 or fallback authoritative read. Further polls
return the same failure while bounded retirement disposes all capacity owners;
cancelled projections similarly return no score and retire incrementally.
End-of-horizon/replaced/released holds, cargo, windows, token storage, nested own
state and partial builders have explicit bounded retirement; no capacity-sized
iterator destruction in a poll/cancel/failure fence. This proposes bounded work,
not a measured5ms acceptance claim. Serialization/hash inclusion of holds remains
an entirely separate closed schema review: append nothing to canonical State,
Receipt, CandidateAudit, Candidate identity or current trace bytes here.

### Exact feature stream and strict class0

For each B and F, collect ordinary supported private facts in (tick,phase,subject
ID,recipe/resource declaration order): actual produced units by resource; real
projected shortage-caused loss by colony/tick/cause; exact own carrying-capacity
deficit; research points/completion only at their true clocks; departure/material
payments once; and every colony's due Food/energy Need result with population_before,
required, paid, shortage and exposure. Output counts come from successful atomic
recipes, not stock movements/unloads/refunds. No held transition produces a fact.
Own conditional retained colonies still generate Needs/exposure; claim/capture
holds do not erase people. Boarding100 real people from an eligible origin>=200
moves them to the existing hull passenger pool; passengers are not destroyed or
new colony residents. Actual scenario deaths from supported own Needs use the
ordinary demographic formula, never unknown battle/capture losses.

Use scoring §1 formulas unchanged: exposure=ceil(population_before*shortage/
max(1,need)), zero need0; FirstExposure at each colony's first due result in
(T,T+100], AllExposure at all due results through T+200; shortage-caused ImminentLoss
within the first100 boundaries only. B establishes the threatened colony/result
roster and deadline D(A) once. Compare matching baseline world/tick results; a
candidate cannot evade comparison by moving people, withholding a clock, deleting
a threatened row or treating an absent expected own result as zero. Lost/missing
expected result without a supported demographic rule faults scoring. Existing
valid zero-population semantics still apply with real loss/passenger accounting.

A material C qualifies class0 by recovery ONLY if D(A) exists, at least one baseline
threatened result strictly improves FirstExposure or ImminentLoss, **NO baseline
currently threatened colony has increased combined FirstExposure**, and EVERY
baseline fully funded imminent Food AND energy payment stays fully funded in F.
A resource payment with paid==required in B must have paid==required for that
world/tick/resource in F (using F's real population-based required amount).
Protect all baseline funded payments, including those at other owned colonies.
A farm/boarding/repair payment that breaks one cannot qualify or be materially
admitted by slipping to class3. Reduction after T+100 only cannot qualify class0.
At central same-actor overlay recheck these exact guards with all earlier admitted
same-actor costs, while preserving the phase7 capture; other actors' output is not
spendable forecast money. Lower classes remain behind legal class0 reservations.

The public-request opportunity alternative requires positive feasible dispatch under the schema Exact zero-freight transition; a known restriction-zero quantity cannot qualify. Subject to that predicate it is exact: positive visible
Food/energy request with tick<=T<expires_tick, urgency max(T+1,min(expires_tick,
checked(tick+100))), feasible locally funded quantity capped by min(request.units,
feasible batch within200). It can rank class0 with predicted delivery/exposure saved0;
its own funded-needs protection and no harmful threatened-colony guard still apply.
Do not relabel that opportunity as actual recovery/delivery/fulfilled commitment.

Compute the SAME existing utility components and role/policy/observed-objective
terms in both projections. Exposure/loss deltas=B-F; output/capacity/research
benefits=F-B under published definitions; negative deltas remain signed. Hold sets
may differ because C's real payments/storage/people change a guard; they are
explanations, no risk penalty/bonus. Exact local farm/known own manifest/reserve/
research/boarding effects can produce nonzero recovery. Settlement and offense
retain published role/policy/objective components but get no speculative ownership,
colony grants, victorious loot/output or enemy disappearance credit. No favorable
hypothetical victory is inserted to restore competitiveness. Existing category,
urgency, score, lane, family and identity tie order stays unchanged.

## 2. Exact categories and hard priority

Assign each material candidate one category, ordered0<1<2<3:

0. **imminent_needs_recovery:** D(A) exists and C strictly reduces FirstExposure
   or ImminentLoss at one or more baseline threatened owned needs results in
   (T,T+100], without increasing FirstExposure for ANY currently threatened owned
   colony. A reduction only in AllExposure/Loss after T+100 does not qualify. It must also preserve every already-funded
   imminent needs payment under the protection rule below. A preparatory Food/
   energy offer/route may enter0 using an exact live public request even
   when its counterfactual predicts no delivery; its score counts request coverage
   as opportunity, never actual exposure saved. A supplier's own protected needs
   cannot be sacrificed to another actor's request.
1. **recovery:** C reduces owned shortage exposure outside100 ticks, reduces
   above-capacity deficit, returns paid stranded assets/passengers, or supplies a
   real Defend objective from a recorded current hostile threat. No fabricated
   evacuation/research completion qualifies. Defend remains behind category0.
2. **obligation:** fulfills an existing exact eligible commitment/request using
   actual known route/fleet/local funds, with no category0/1 benefit.
3. **ordinary:** every other legal investment, exploration, settlement, research,
   access/defense offer or offensive plan.

Class0 admission occurs before ALL class1/2/3 material admissions globally,
including different actors. Within an actor/lane, any legal class0 candidate
eliminates lower-class candidates from selection; rejection by central scarcity
may try another class0 candidate, never silently switch to discretionary spending
at the same boundary. If no legal class0 exists, lower classes can compete only
within protected needs reserves. This permits recovery planning without asserting
that every imminent shortage has an affordable rescue.

Public request opportunity: only positive Food/energy DemandRequest records
with tick<=T<expires_tick and permitted visibility qualify. Their priority deadline
is max(T+1,min(expires_tick,tick+100)). This is an explicit REQUEST URGENCY convention,
not a claim to know the recipient's private actual needs clock. A route/offer can
count at most min(request.units, feasible locally funded batch units within200
ticks); no rival output/inventory is assumed. Requests for ore/alloy remain class2
or3. Positive requests do not certify actual later arrival or recovery.

Free protocol maintenance is separate: expiry/termination, posting the actor's
own exact due demand records, objective progress and observations follow their
existing clocks. Research-target selection consumes no material; the exact policy
choice below is prepared after needs priority evaluation and cannot authorize an
unfunded/needs-consuming lab recipe. No free action can masquerade as paid relief.

## 3. Candidate roster and legal enumeration

Use every legal candidate available from the snapshot, not an arbitrary top-N.
The authenticated enumeration trace contains ELIGIBLE COMPLETE candidates only;
no ineligible/null-candidate record is emitted or counted. Complete-schema
§Integrated correction I4 defines deterministic term formation, exclusion,
deduplication and central-attempt reason emission.
Candidate arrays are traversed under a cursor; store only bounded best-so-far and
central loser/alternative cursors. Candidate identity bytes are canonical JSON
`{action:IntentAction,admission_terms:AdmissionTerms}` in that field order.
Deduplicate only byte-identical full identity bytes, excluding derived category,
score, tie fields and audit digests. Hull target and route batch alternatives
therefore remain distinct even when the partial IntentAction is identical. All structural/catalog/
capacity/path/ownership/intelligence/treaty eligibility applies BEFORE scoring.
Evaluate exact replacement candidates when colony slots are full; a construction
with scrap names one eligible facility and its actual half-refund/derived effects.
Never infer a free industrial slot from a hull job (reservations are separate).

| Action family | Enumerated candidates and scored contribution |
| --- | --- |
| Construction | each owned colony × each allowed definition × no scrap or each legal scrap; exact farm/solar/habitation/lab/extractor/foundry/shipyard/battery forecast and costs |
| Hull | each staffed owned shipyard × ark/escort/scout × each legal target/new-fleet reservation; actual cost, role, repair/return/supply constraints |
| Route | each observed eligible direct source/destination × four resources × new/each matching adjustable owned route; batches1..10 and the fixed cadence10/reserve>=20; choose exact terms by scoring, not hidden stock |
| Explore/Move/Return | each idle eligible owned fleet × each permitted reachable destination, including paid-return paths; path tie is duration then lexicographic lane-ID path |
| Board/Settle | every eligible owned ark/origin/observed unowned destination; boarding uses100 people and requires origin>=200, settlement transfers its real passenger count |
| Defend/Attack | each idle eligible supplied owned fleet × each valid observed objective target; current-world defense/paid travel/return retain unsupplied eligibility stated in diplomacy |
| Blockade | each supplied idle escort fleet × known enemy route endpoint under declared war; only endpoints evidenced by an owned/received exact route/manifest, never unseen rival route enumeration |
| Agreement/Offer | every eligible counterpart/target/exact terms under the three published offer kinds; terms must satisfy an exact visible request or known access/threat facts |
| War | each valid objective target/counterpart × each eligible whole idle owned fleet × each eligible objective arrival endpoint; bind the exact force witness below, then class/utility/ties |
| Select research | each incomplete eligible technology, using exact policy order below; no candidate if the desired target is already selected |

V3's partial IntentAction Construction/Hull/Route fields do not themselves encode
all catalog/target/batch/reserve choices above. This document proposes the exact
scoring Candidate payload required for independent vectors: it includes full
chosen admission terms and target-reservation IDs, not merely a partial intent
from which a codec invents terms. See complete-schema §Integrated scoring, admission and privacy payloads; these
required choices are encoded explicitly, never inferred by a codec.

One material economy action and one fleet action may be admitted per actor at a
50-tick boundary; one outgoing offer at a100-tick boundary obeys the existing
pending-offer cap. War is in the fleet lane; Route/Construction/Hull in economy;
Blockade/Board/Explore/Move/Settle/Defend/Attack/Return in fleet. No combined multi-action candidate is introduced here. A dependent boarding then
settlement, build then hull, or offer then route uses separate actual boundaries,
not a forecast that admits both for one payment. If an actor
has class0 in either lane, its ordinary candidates in the other lane are blocked
for that boundary; cost-free current-world Defend/Return may remain legal class1
only after all class0 reservations. These lane/category details are Proposed.

## 4. Numeric economy/recovery utility

For non-offer, non-war material candidates, define signed i64 Score:

U = 1000000 * DeltaFirstExposure + 1000 * DeltaAllExposure
    + 100 * DeltaShortageLoss + 10 * DeltaCapacityDeficit
    + 8 * DeltaFoodProduced + 4 * DeltaEnergyProduced
    + 2 * DeltaOreProduced + 4 * DeltaAlloyProduced
    + 25 * DeltaResearchPoints + 100 * PublicRequestCoverage
    + RoleValue + PolicyBonus - 2 * EnergyPaid - OrePaid - 4 * AlloyPaid
    - TravelTicks.

CapacityDeficit is sum max(0,population-effective_capacity) across owned colonies
at T+200; delta is deficit avoided. ResearchPoints counts real incremental lab
points across the200-tick forecast, without counting prior target progress twice.
Paid resources are the candidate's actual admission debit NET of its legal scrap
refund, limited to actual usable refund (discarded overflow earns no credit).
They exclude unchanged baseline upkeep already included in both forecasts.
A negative net debit is allowed only for a real reviewed refund. TravelTicks is
the chosen candidate departure itinerary duration,0 for stationary operations.
RequestCoverage is opportunity units under §2,0 otherwise; it cannot appear in
DeltaExposure/Loss unless an actual known manifest produces that result.
RoleValue: ark200, scout100, escort150 per newly admitted hull; other actions0
except the explicit Return and Blockade terms below.
Returning a hull earns10 per actual returned hull (not free hull creation).
No other omitted utility term is permitted. Record all components with their
literal numeric field names in DecisionReason.PolicyScore. Exact tagged component
representation and reason emission are complete-schema §§Integrated scoring and
correction I4; it preserves raw signed deltas without an optional extra Forecast
reason whose byte order or meaning a writer could choose.

| Policy | Farm | Solar | Habitation | Lab | Extractor | Foundry | Shipyard | Battery | Ark | Scout | Escort | Route |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| neutral |0|0|0|0|0|0|0|0|0|0|0|0|
| trader |200|200|100|100|100|100|0|0|100|100|0|1000|
| guardian |200|200|200|300|0|0|100|600|0|100|600|100|
| expansionist |200|200|400|100|200|200|300|100|600|400|100|200|
| industrialist |100|400|100|200|600|600|200|0|100|100|0|300|

Table columns map to these exact catalog slugs: Farm farm, Solar solar_array,
Habitation habitation, Lab research_lab, Extractor extractor, Foundry foundry,
Shipyard shipyard, Battery defense_battery, Ark colony_ark, Scout scout,
Escort escort; Route means the route action family, not an industry definition.
These new slug spellings must be frozen in the integrated V3 catalog review.

All other actions' PolicyBonus is0, except the separately specified offer/objective
utilities. Custom/unknown policy/definition is not scored by guessing a nearest
role; the finite initial catalog/version must supply the exact reviewed table.
Ordinary paid candidates require U>0; Wait has U0 and wins over nonpositive
ordinary spending. Recovery/obligation candidates may have U<=0 because urgency
and actual recorded obligation precede speculative productivity. A blocked recipe
without exact staff/inputs cannot earn nominal outputs or research points.

Research target order, first eligible wins, then UTF-8 slug for a future reviewed
equal priority: Trader efficient_logistics,agronomy,resilient_habitation; Guardian
resilient_habitation,agronomy,efficient_logistics; Neutral/Expansionist/Industrialist
agronomy,efficient_logistics,resilient_habitation. Guardian cannot skip Agronomy's
prerequisite. Existing per-technology progress persists. Selection's recorded
score is3000 minus1000 times the0-based listed rank; no phantom local funding.

## 5. Offer and objective utilities

Preserve diplomacy's explicit offer arithmetic unchanged:
Supply U=100*batch+min_relation; Access U=100*observed_unowned_reachable+min_relation;
Defense U=100*observed_threatened_owned_worlds+min_relation. Relation contribution
clamps0..100, counts clamp512. Bonuses: Trader Supply1000/Access500;
Guardian Defense1000; Neutral Access500; Expansionist Access1000/Supply500.
Industrialist uses Neutral's Access500, others0 as an explicit Proposed fallback.
Class/urgency gates apply before that U, so a Food/energy request can outrank an
otherwise larger discretionary Access score. Within a class retain published
kind tie Supply,Access,Defense, target world, counterpart ID, proposer ID,
then full identity bytes. Section7 supplies the one authoritative merge tuple. Published offers do not
receive the generic recovery utility's PublicRequestCoverage term a second time.

Every offer Candidate uses ScoreComponents.offer; every war Candidate and
DefenderObjectiveAudit uses ScoreComponents.war with exact zero/mapping rules in
the complete-schema registry. All published weights below remain unchanged.

Every valid war/defender objective ranks Defend before Secure access before Occupy,
then the following U within its class, then stable target ID, counterpart ID and
canonical objective bytes. Admission still requires the declarer's assembled
funded reachable force, known treaties and eligible intelligence. Defender's
objective needs valid permitted references but need not itself have an achievable
force, since peace explicitly evaluates its inability to achieve that objective.
No valid observed defender target rejects central admission without inventing
unseen targets. Declarer enumeration does not read that private selection; the
explicit two-stage attachment below is the integrated declaration contract.

Defend U = 100000*observed_hostile_escorts + 50000*observed_occupation
           + 1000*own_target_population - shortest_eligible_relief_travel_ticks
           + DefendPolicy.
Secure access U = 10000*observed_unowned_worlds_accessible_via_named_lane
                  + 1000*owned_shortage_worlds_with_known_route_using_lane
                  + 100*owned_routes_using_lane - eligible_force_travel_ticks
                  + AccessPolicy.
Occupy U = 10000*observed_completed_facility_count
           + 1000*known_own_route_endpoints_at_target
           - 100*observed_defense_strength - eligible_force_travel_ticks
           + OccupyPolicy.

Observed_occupation is0/1 for the named enemy; escort counts refer to that enemy,
not aggregated friendly/unknown fleets. Defense strength remains10*observed
defending escorts+20*operational batteries. Occupy/Access force eligibility still
requires strength>=ceil(3*observed_defense/2); Defend requires strength>=observed
hostile escort strength. These are unchanged diplomacy numerical thresholds.
Unreachable candidates are invalid, not assigned an invented travel penalty.
For defender ranking without an eligible force, the travel penalty is0 AND the
reason explicitly records force_unachievable; this does not make it achievable.

**Exact Proposed force witness.** Enumerate each whole idle owned fleet, never an
unrecorded split or invented reinforcement. Hull IDs are its complete sorted live
hull roster; strength counts only its actual live escort hulls. The existing
supply/offense/current-world-defense exceptions still apply. Defend/Occupy arrival
endpoint is their named world. Secure access enumerates each of its named lane's
two endpoints separately, only when the endpoint is known, reachable and legally
enterable under current permissions. Owning an endpoint does not grant the other.
For each fleet/endpoint choose the minimum-duration permitted simple itinerary,
then lexicographic lane-ID sequence among equal-duration paths. A current-world
witness has an empty path, travel0, no departure payment. Checked duration is the
sum of the inherited corridor travel formula for that actual fleet and lanes.
Unavailable paths/accounts/supply/strength reject that witness before scoring.

Each eligible fleet/endpoint is a distinct war candidate with its own TravelTicks
and exact local funding witness. Objective-class ordering remains first; equal-U
war variants compare objective target, peer, arrival endpoint, fleet ID, sorted
hull-ID bytes, then full identity bytes. Defender selection uses this same local
objective/force order. For an otherwise valid defender objective with no eligible
force, generate exactly one variant with null force and force_unachievable=true;
never generate this variant when any eligible force exists for that objective.
A declarer must always have a nonnull force. The defender's force witness is
information permitted to the defender, recorded in its own decision audit; the
declarer does not acquire private defender fleet/accounts through negotiation.

**Exact Proposed declaration/defender attachment stages.** At the shared
post-phase7 boundary, enumerate declarer candidates solely from its permitted
capture. War IntentAction remains `{type:war,peer:ID}`; War AdmissionTerms contains
only peer, declarer_objective and declarer_force. No prospective defender objective,
private defender audit/digest or availability boolean enters candidate identity,
U, category, queue, TieKey or CandidateAudit enumeration hash.

For each central attempted declarer candidate, first validate its own guards and
hold its exact force/account. Then ask the named defender authority to enumerate
and select its objective under the DEFENDER'S already-captured same post-phase7
snapshot, using §5's exact private objective/force ranking. This is a deterministic
central admission operation, not a new turn or a mutable recapture. Its result
may differ across hidden defender states; this affects admission outcome only,
never declarer candidate inputs/order. If no valid defender objective exists,
reject with `no_valid_defender_objective`, release all private declaration holds,
record the attempted candidate unchanged and proceed to its next candidate once.
Do not replace its objective/force/score or silently rebuild identity after reply.
A legal defender objective with null force is still installable and records the
explicit force_unachievable audit. No declaration is published without BOTH
validated objectives attached atomically from those two exact captured selections.

The installed War's public defender objective first becomes visible through that
successful admission's war-created fact and subsequently through permitted War
state at the next planning capture. It is absent from the original planning
capture. The full central audit binds unchanged declarer identity/capture to defender
capture/selection and installed pair and is creator/replay-only. Each actor sees
its separate own decision/selection audit, never the other actor's capture, force
or account through the combined record. Both captures have equal tick/branch/catalog
and phase7, but their permitted_snapshot_digest differs by actor. The defender's
force/account/score audit remains actor-private; disclosure exposes the selected
objective alone. A private stock change must leave declarer enumeration and scores
identical; it may change this later response or acceptance, which is explicitly
outside that invariant. This two-stage public attachment is an architectural Proposed amendment to the
pre-integration declaration contract. The integrated schema §Diplomacy lifecycle
now binds both objectives at central admission, not actor enumeration.
Creator ForceWar remains its explicitly supplied two-objective command.

The funding witness binds departure world, actual hull roster, path and departure
energy computed by the inherited per-hull departure rule with completed Efficient
logistics and any already-paid return credit. Required payments sort by resource;
zero payments use an empty list. It is an eligibility/reservation witness, not a
new war cost or a departure order. Central declaration admission transiently holds
that fleet and its departure-world unheld stock against competing same-boundary
material candidates. These holds release at boundary publication. A declaration
consumes no departure energy, does not set a travel clock and predicts no automatic
arrival in F. The war utility's travel penalty ranks the witness's potential trip;
EnergyPaid/OrePaid/AlloyPaid remain0 for the declaration. Any later Attack/Defend
movement revalidates its current fleet/path/account/needs/supply and pays once at
its actual departure, without reusing the released hold. Defender selection
reserves no resources and charges nothing. This makes the declaration eligibility
requirement concrete without double charging a trip.

| Policy | DefendPolicy | AccessPolicy | OccupyPolicy |
| --- | ---: | ---: | ---: |
| neutral |1000|5000|0|
| trader |0|5000|5000|
| guardian |10000|1000|1000|
| expansionist |1000|10000|10000|
| industrialist |1000|5000|5000|

Unknown rival population/stock/research has zero CONTRIBUTION, not a displayed
zero value. No conquest utility includes unobserved resources or future loot.
Blockade uses generic U plus an additional RoleValue1000 times observed enemy
completed facility count at its known endpoint; it cannot claim destruction of
already-dispatched cargo or predict unknown opponent shipments.

## 6. Central reservation and protected needs

Reservation is one unpublished deterministic ledger, shared by phase8/9 material
admission. Seed it from exact captured paid construction/hull reservations,
committed queue and already-paid departures. Offer acceptance validates its real
terms, reserves commitment/route capacity and any first dispatch funding against
this ledger; no future promised cargo is credited as inventory. Score one common
snapshot, then validate/admit against the ledger. A denial records the specific
resource/slot/hull/target winner and proceeds to that actor/lane's next best
same-class candidate. Scores never recompute using another actor's spent balances.

Ledger entries are exact typed accounts:

- (world,resource): current physical inventory minus actual accepted debit, plus
  only actual accepted usable refunds. Never spend forecast output at admission.
- (world,industrial_slot): completed facility occupancy plus paid building jobs;
  a scrap releases its actual slot only atomically with its replacement.
- (shipyard,hull_job), global hull count, (target_fleet,hull_slot), and new-fleet
  reservation: separate from industry slots. Reserved fleets cannot simultaneously
  depart/split/merge or accept two incompatible hull destinations.
- (colony,worker_assignment): use the committed assignment at operation time;
  no phantom workers from future growth. Completed jobs receive workforce only
  under the exact recomputation points in ecology.
- (ark,passenger_capacity) and (origin,population): boarding reserves100 real people
  once; no hull can be both reserved for another plan and separately evacuated.
- route/shipment capacity, exact route mutation ID, outgoing offer per actor,
  commitment/request capacities, treaty endpoint/party permissions and exact
  objective target: use actual paid/structural capacity, not a visual badge.
- research funding is (world,energy/alloy,lab clock,target technology), never a
  civilization-global stockpile. An eligible target is not prepaid points.

Needs protection has two distinct precise rules:

1. **Material admission:** rerun F(A,C) using all earlier accepted SAME-ACTOR
   candidate costs as a ledger overlay, but the same permitted capture. For every
   baseline owned first due Food/energy payment within100 ticks that was fully
   funded, F must still pay it fully. A class0 rescue must also not increase a
   threatened colony's combined first exposure. Other actors' forecast production
   is never overlaid. This rule protects funded outcomes without treating future
   output as current spendable money. Simultaneous own economy/fleet/diplomacy
   debits cannot each independently claim the same future funded needs payment.
2. **Actual due spending:** at a due population-needs boundary, before phase5
   discretionary industry/research/repair/military-supply debits, hold the actual
   local Food/energy required by phase-entry population. Missing units are not
   invented; available=min(stock,need). A non-life-support debit may spend only
   unheld stock. For a Food-producing operation that requires held energy, compare
   the exact local imminent Needs outcome without/with that atomic recipe: permit
   it only if combined exposure strictly decreases AND neither fully funded
   resource payment becomes short. Record both actual resource debit and the
   protected-needs reason. No fractional/partial recipe payment is allowed.

The second rule is the integrated Proposed precedence refinement: compared with
the pre-integration lab/supply eligibility, it adds needs-reserve denial before
phase6 without altering recipe costs/periods or population formulas. The integrated society/diplomacy and closed blocked-reason/facts schema
acknowledge this Proposed guard; independent integrated review remains required. No silent use of
MissingInput with a falsified physical stock quantity is conformant (§9).

Freight source reserve is max(the route's reviewed source_reserve, known next
Food/energy need for that resource); non-needs resources use source_reserve.
Fulfill only above this floor, with actual stock at each dispatch. Route/offer
admission may reserve at most its next exact batch in the current ledger, not
1200 ticks of promised stock. If a known upcoming eligible dispatch uses inventory
also requested by construction/repair/labs/fleet supply, the same typed account
can fund only one; strict recovery category wins, then the reservation order below.
The transient batch hold ends at boundary publication and never turns into hidden
permanent escrow. Future dispatch revalidates actual stock/needs and emits Wait.
Prior paid manifests remain immutable and retain original treaty provenance.

All automatic due costs also use exact local eligibility and central account
debits in source-defined phase/ID order. This proposal does not retroactively
rescore already-paid travel, cancel completed funding or refund consumed inputs.
Reservations undone by an invalid candidate release only its private holds before
admission; a rejected candidate consumes no IDs/resources/history.

## 7. Exact stable order, ties and arithmetic bounds

Global material order is the following single lexicographic tuple:
(category ascending, urgency_deadline ascending with absent last, U descending,
lane tag ascending, family rank ascending, offer_kind_rank ascending,
primary target ID ascending with absent last, counterpart ID ascending with absent
last, actor ID ascending, full identity bytes ascending).
Lane tags are economy0/fleet1/diplomacy2; protocol3 is free audit only. Family ranks
are their0-based roster rows in §3. offer_kind_rank is Supply0/Access1/Defense2 for
offers and0 otherwise; primary target is construction/hull/board world, route destination, movement
target or declarer War objective target; offer Supply uses destination_world,
Access/Defense use absent because their canonical Terms name parties only.
Agreement/research targets are absent IDs. counterpart is exact offer recipient
or war/agreement peer, absent otherwise. No phantom offer world is inferred. Offer peers with different kinds therefore compare kind before
actor ID as the published offer order requires. The final bytes are the complete
§3 identity, not AdmissionTerms alone and not derived Candidate score/audit fields.
This is a Proposed global merge choice: actor-ID priority applies only after lane,
family, kind, target and counterpart ties, rather than before those fields.

Within the War family's actor-local queue, order category/deadline, then objective
class0 Defend/1 Access/2 Occupy before U, then the exact objective/force tie in §5.
Offer only that queue's head to the global merge; rejection exposes its next
candidate. Every other family likewise exposes its tuple-ordered head. Thus
objective-class precedence is local to War and not a cross-family utility bonus.
Urgency deadline is earliest OWN helped imminent boundary or public request
convention, whichever applies to that candidate; a recovery of later needs cannot
pretend it improves an earlier unhelped boundary. For category1 use exact known
recovery/threat deadline or T+200 when no tighter clock exists; category2 use the
obligation expiry; category3 absent. All ID comparisons use decoded bytes.

An actor's selected material candidates are generated from its immutable capture
and ordered by this tuple. Central acceptance does not alter the order. Rejected
predecessors remain identified, so a next candidate cannot silently become a new
best through hindsight. An actor/lane is filled only by one successfully admitted
candidate; consuming a rejection does not fill it. Every candidate gets one
central attempt, no oscillating retry after the queue changes. Existing offer
response consideration retains the wire's recipient-ID/offer-ID order; first
reserve the class0 joint-plan needs/capacity, then consume leftover admission
capacity in that consideration order. A denied offer remains pending with the
exact capacity/needs reason. This explicitly resolves cross-phase competition.

All products, sums, deltas and sort-score negation compute in checked i128,
then range-check i64 for Decision.score. Never wrap, saturate a hidden score or
use floating point. Input caps bound forecasts: at most512 colonies, population
10000, next_needs_tick>T at the post-phase7 valid capture and therefore two due
needs per200 ticks, exposures <=20480000; first exposure<=10240000.
Output/research counters are checked u64; their observed maxima derive from
finite facility count, positive period and per-operation caps in the exact catalog (at most one due
attempt per facility per forecast boundary, including overdue operation clocks),
not guessed infinity. A schema-valid catalog whose published U exceeds i64 is a
visible scoring arithmetic fault before publication, not a fabricated lowest rank.
Counters/ticks also reject checked overflow. All score components/deltas and the
actual category/deadline/tie tuple are independently replayable.

Do not store a billion-candidate vector just to find a winner. For each actor/lane,
stream the exact enumerated candidate trace in the §3 family order then full identity
byte order through SHA256 domain
`NYON_LIVING_V3_CANDIDATE_AUDIT`+NUL. First append actor ID16, lane:u32le and
length-prefixed canonical ActorInputContext bytes; then append length-prefixed
canonical trace records in that order. Trace record is CandidateTrace with field order action_tag,action,admission_terms,
candidate; candidate is REQUIRED nonnull and includes every exact category/
component/score/tie field. Trace only structurally/guard-eligible complete terms
under the shared capture, deduplicated by full identity before hashing. No absent
path, unknown target or unfunded/unstaffed/full-slot no-replacement candidate emits
a record. No central admission outcome/reason or future defender response enters
the enumeration digest. Each record length uses u32le and contributes exact bytes.
Retain eligible emitted count:u64, trace digest, best candidate per family and
every actual central attempted candidate/disposition. Ordinary legal nonpositive
U remains in trace but loses to Wait0 without a central attempt.
An independent replay can reproduce the full trace without retaining it in RAM.
CandidateAudit is declared explicitly in the integrated schema; it does not
silently drop required events. Existing65536-event/8 MiB
receipt overflow still faults atomically rather than truncating attempted outcomes.
All enumeration, hash bytes and alternative rescans share the visit budget;
qualification must measure their real cost, including collision validation.

## 8. Complete worked arithmetic and edge cases

These examples fix all nonzero inputs; omitted components are0. They validate
ranking arithmetic only, not that a particular fixture produces those projections.
Each fixture must later establish the projected outputs through independent rules.

**Competing recovery, same deadline.** Neutral colony population1000 has next
Food need10 and energy need5. Food shortage1 means exposure100; energy shortage1
means exposure200. Candidate F saves the Food exposure100 in first/all periods,
produces10 additional Food and pays2 energy. UF=100000000+100000+80-4=100100076.
Candidate E saves exposure200, produces8 energy and pays20 alloy.
UE=200000000+200000+32-80=200199952. Both class0/deadline50; E wins. This Proposed
normalization prefers the larger population fraction restored, not the larger
nominal recipe number. Either candidate that makes a fully funded other need
short is invalid BEFORE this comparison.

**Hard gate beats a larger ordinary score.** Farm recovery at deadline50 has U10.
Ordinary foundry has U1000000000. The farm wins category0 versus3 despite its
smaller numeric score; no policy bonus can cross the category gate. If the farm
fails capacity, try the next class0 candidate; do not fund that foundry instead.

**Policy distinguishes equal productive alternatives.** Identical forecast value
2000 and AlloyPaid30 yield Neutral U1880. An Industrialist extractor adds600:
U2480; its Farm adds100: U1980. If Farm is class0 and extractor ordinary, Farm
still wins; same class/deadline puts extractor first. Equal category/deadline/U
uses the authoritative §7 tuple, with no insertion-order winner.

**Scarcity, shared snapshot.** Actors A=...01 and B=...02 see legal equal-score
routes requiring one remaining route slot and no conflicting private stock.
Both recorded scores remain identical. Global actor-ID tie admits A; B gets
capacity rejection naming that reservation, then considers another SAME-CLASS
candidate. It cannot rescore using A's new route or stock. A separate actor C's
class0 route reserves that slot before A/B's ordinary treaty admissions, even if
A's offer is considered earlier by recipient/offer ID.

**Cross-actor offer tie.** Supply from actor...02 and Access from actor...01
have identical category,deadline,U and diplomacy family. Supply's kind rank0 wins
before Access's rank1, irrespective of smaller actor...01. Same-kind ties compare
primary target, recipient, proposer, then complete action+terms identity bytes.

**Complete identity alternatives.** Route batches1 and10 share a partial Route
action but different admission_terms.batch_size; both survive enumeration. Two
Hull alternatives target fleets...01 and...02 and survive likewise. Identical
action AND complete terms deduplicate; score changes cannot manufacture identity.

**Imminent versus later benefit.** Baseline shortage at T+50 remains unchanged;
a candidate improves only a needs result at T+150. Its FirstExposure and
ImminentLoss deltas are0, so it is category1 despite positive AllExposure/Loss
utility. A candidate reducing a shortage-caused loss at T+100 qualifies0 only
with the exact improved needs result and no threatened first-exposure increase.

**Private defender change, invariant actor enumeration.** Two root states have
identical declarer permitted captures but different hidden defender energy. All
declarer candidate identity bytes, trace digest, scores and order are identical.
ActorInputContext excludes hidden State/revision digests; full outer audits and
Receipt/Event digests may differ and remain replay-authenticated. Central admission
may attach defender objective X in one and Y in the other;
WarAdmissionAudit binds that difference to each defender private phase7 capture.
No valid defender objective yields a rejection of the SAME attempted identity,
not a rehashed candidate. Only successful attachment publishes the public choice.

**War witness tie and payment.** Two eligible fleets...01/...02 at different
owned docks reach the same objective endpoint in20 ticks. Equal objective U ties
on endpoint then fleet ID, choosing...01. Its witness names that dock, complete
hull roster, exact20-tick path and inherited local departure energy. Declaration
holds that account/fleet until publication, pays0 and creates no departure. Later
movement pays its freshly validated departure once. For a Secure-access lane,
equal endpoint variants compare endpoint ID before fleet ID. With no eligible
defender force, a null witness records force_unachievable and pays/reserves0.

**One actor, two lanes.** Snapshot energy10 and fully funded imminent need5;
economy candidate costs4, fleet candidate costs4, with no known replacement.
Each alone leaves6 and passes. Economy admission leaves6; overlaying the fleet
would leave2 and fail protected needs. Fleet is denied instead of spending the
same protected5 twice. Pending lab/treaty next-batch funding uses that same account.

**Research target and progress.** Guardian has Agronomy incomplete, Logistics
eligible and Resilient habitation not yet eligible. Its listed ranks are1/2:
selection scores2000/1000, so select Agronomy. Completing Agronomy makes Resilient
rank0/score3000 eligible at the next target boundary. Industrialist uses Neutral
order; existing Logistics progress remains when switching. A lab with target cost
remaining1 cannot earn extra points for a second newly selected target that tick.

**Offers, exact published arithmetic.** Trader Supply batch4 with relation20 has
U400+20+1000=1420. Access reaches5 observed unowned worlds with relation20:
U500+20+500=1020. Supply wins within equal class. If the supply request is only ore,
and Access is ordinary while an unrelated exact Food recovery is class0, the
Food candidate wins its hard category before either offer. Capacity-denied offer
remains pending with offer_considered waiting fact, not a refusal/new creation.

**Exact utility-family mapping.** Supply example uses offer fields kind=
resource_supply,batch4,reachable_worlds0,threatened_worlds0,min_relation20,
policy_bonus1000: U1420. Access uses transit_access,batch0,reachable_worlds5,
threatened_worlds0,min_relation20,policy_bonus500: U1020. Guardian Defense uses
mutual_defense,batch0,reachable_worlds0,threatened_worlds3,min_relation20,
policy_bonus1000: U1320. All numeric reasons use these raw fields in that order.
Guardian Defend has hostile_escorts2,occupation=false,own_population500,travel30,
policy10000 and all other war counts0: U709970. Trader Secure-access has
accessible_worlds2,shortage_route_worlds1,own_routes3,travel40,policy5000 and all
other war fields0/false: U26260. Expansionist Occupy has completed_facilities4,
own_route_endpoints1,defense_strength30,travel20,policy10000 and all other war
fields0/false: U47980. Null-force Guardian Defend uses the same facts with travel0
and force_unachievable=true: U710000. These are exact raw field/weight examples,
not populated independent Candidate simulation vectors; those remain required.

**Defender objective.** Guardian sees two valid Defend worlds. X has hostile
escorts2, population500, relief duration30: U=200000+500000-30+10000=709970.
Y has escorts1, population700, duration20: U=100000+700000-20+10000=809980.
Y wins within Defend. A Secure-access U9000000 cannot outrank either Defend class.
Two known enemies at X are separate ObservedThreat records; only the chosen
enemy's escorts count. No available force makes its inability explicit, not a
reason to invent unseen attack targets; exact peace achievability still applies.

**Hidden and stale information.** Rival stock changes from0 to10000 out of view;
no candidate input changes, so scores/category/ties remain identical. An age301
attack observation is invalid even when apparent utility is high. A stale shared
record keeps its original observed tick and threat-fact digest; receive time
cannot make it age0. A supply request exposes units, not recipient inventory.
Request opportunity predicts neither acceptance nor paid cargo arrival.

**Boundary and invalid cases.** Deadline T+100 is imminent, T+101 is category1
unless another exact request/threat qualifies; T+200 is within forecast, T+201
outside. Needs0 contributes exposure0 and causes no division-by-zero. Complete
technology, unstaffed lab/shipyard, absent path, population199 ark origin, full
fleet/route/industry reservations and missing catalog definition reject before U.
Population200 boards100, retaining100 at origin and100 aboard; forecast tracks
both and cannot claim100 people were newly grown or transported twice.

**Eligible-only trace boundaries.** A construction option failing both staff
and physical funding is excluded once before trace; there is no chosen first
local diagnostic byte to hash. An absent path cannot form movement terms and
emits0 records. An unobserved target is never enumerated from root and emits0.
At full industrial slots, no-scrap construction emits0; every legal named scrap
replacement that passes actual refund/slot/funding/needs rules emits its distinct
full identity exactly once. The trace count measures those eligible records, not
all syntactic Cartesian products or diagnostic attempts. After central scarcity,
a previously eligible attempted candidate can be rejected with the exact separate
first-failure contract; enumeration count/hash/order are unchanged.

**Checked arithmetic.** ceil(1000*1/10)=100 and ceil(1000*1/5)=200 use checked
integer product and quotient/remainder, not (a+b-1) overflow. i64::MAX+1 projected
utility rejects before comparison. Signed negative ordinary U loses to Wait0;
negative recovery U remains behind a positive same-category same-deadline rescue,
while still ahead of all ordinary candidates by category. ID ties compare full
bytes; strings compare exact UTF-8 bytes without Unicode normalization.

## 9. Integrated Proposed decisions and compatibility effects

These concrete Proposed amendments are represented across the integrated package;
they remain subject to full independent written review:

1. **Snapshot/phase coordination:** make one post-phase7 capture serve phase8/9
   and reserve class0 plans before ordinary pending-offer acceptance. Existing
   phase order is preserved; the ecology/schema contracts now specify the shared
   capture and cross-phase strict-priority coordinator exactly. Decision provenance
   carries that shared capture and actual admission phase separately.
2. **Due needs guard:** add protected actual Food/energy eligibility before
   discretionary due lab/industry/repair/fleet-supply spending. Society/diplomacy eligibility now includes the guard, preserving numbers.
   The schema includes exact `needs_reserved { world,resource,physical_stock,held,required }`
   blocked reason/fact; do NOT falsify MissingInput.available to explain reserves.
3. **Complete candidate payload:** partial IntentAction omits some chosen
   route batches/reserves, hull-target reservations and exact dispatch/offer parameters. Automatic target
   completion/clearing remains the existing research phase, not a scored free
   unlock or a new candidate action. The schema defines Candidate as `{action:IntentAction,admission_terms:
   typed per-action terms,category,deadline,components,score,tie_key}` plus exact
   CandidateAudit fields and score records before vector derivation. The terms
   are precisely the listed catalog/job/path/route/offer/war/boarding parameters;
   no arbitrary JSON patch or implementer-selected omitted default is permitted.
4. **Opportunity versus evidence:** requested units can rank preparatory relief,
   but cannot produce delivered/paid/recovery witnesses. Exact immutable manifests,
   preserved dispatch provenance and later matched receipt facts remain required.
5. **Finite policy/catalog fallback:** Industrialist uses Neutral research order
   and Neutral offer Access bonus; its economy/objective bonuses are the explicit
   new tables here. Unknown policies/technologies/definitions require a reviewed
   new table/version, never “closest known policy.”
6. **Information scope:** incoming manifests needed for owned needs forecasts are
   explicitly present as incoming_shipments in the permitted snapshot. Enemy route endpoints need
   existing permitted evidence; absence is unknown, not legal enumeration from
   the omniscient root. Future seeded hazards are not known until warning.
7. **War funding witness:** specify whole-fleet/endpoint/path enumeration and
   exact private force/account binding. Declaration holds are transient eligibility
   reservations, never departure debits. The schema declares ForceWitness and
   distinct declaration-hold versus movement-payment audit facts. Declarer identity
   excludes defender objective entirely; central admission attaches it from the
   defender's same-tick private capture. This explicitly replaces the pre-integration
   actor-declaration two-objective requirement. Defender witness remains private;
   only successful installed War objective is newly disclosed.
8. **Evidence and resource limits:** full enumeration is bounded by cursors, not
   an arbitrary candidate cut. Compact CandidateAudit is now an explicit schema declaration;
   timing/memory and receipt caps must be independently qualified. No declaration
   here proves the forecast/rescan fits5 ms or a platform is accepted.

Scoring math and tie order are concrete for written review. Independent integrated review, complete vectors and implementation/qualification
gates remain prerequisites. The integrated wire/schema amendments are Proposed;
old reviewed source identities remain in the integration baseline and historical
reports. This document does not make Proposed
mechanics Current or reopen an architecture permission loop.


## 10. Integrated closed scoring payloads

The sole canonical field/tag registry for Candidate, ScoreComponents,
AdmissionTerms, ForceWitness, TieKey, CandidateAudit, DefenderObjectiveAudit,
WarAdmissionAudit and NeedsReserved is complete-schema §Integrated scoring,
admission and privacy payloads. This document owns their numerical/category/tie/
forecast/admission semantics; do not duplicate or infer omitted codec fields.
DecisionTier has all four categories; actor War action remains peer-only while
its exact declarer objective/force lives in AdmissionTerms. The defender objective
is attached only in central admission, never in declarer enumeration identity.

All eight specifications remain Proposed. Scoped scoring correction approval did
not accept balance, full vectors, integrated design, code or platform behavior.


## Integrated round2 environmental forecast selection (Proposed)

Before B(A) or any candidate counterfactual, use schema §Integrated correction I1:
select complete system/lane coverage across every permitted observation source,
including empty negative assertions, then select the newest child per event ID
INCLUDING expired children in winning scopes, filter ONLY the chosen child
by recorded expiry, then check128-event limits. Never retry an older identity
copy after filtering; retain authenticated scope contents unchanged. Source
refresh/replacement or eviction can remove the last newer identity record and
thus lose that knowledge, as schema I1 explicitly distinguishes. Never union losing scopes. Equal-tick
coverage chooses lowest original observer then canonical scope bytes. Unknown,
partial or unrelated observations cannot cancel a modifier; hidden cancellation
retains its stale counterfactual until permitted supersession/expiry. Both baseline
and candidate use the SAME selected scopes. Include empty scope winners in
permitted snapshot hash; consult no hidden root schedule. Fresh schedule visibility
requires an incident directly visible world, not own fleet traversal alone. All
integer formulas, categories, thresholds and score tables remain unchanged.
