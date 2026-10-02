# Living V3 diplomacy, logistics and conflict

Date: 2026-10-01. Status: **Proposed numerical contract for review**.
Parent: `2026-10-01-living-expansion-architecture.md`.
Boundary order is the shared contract in `2026-10-01-living-ecology-rules.md`.
This is an expansion design, not implemented or accepted V2 behavior.

## Offers and negotiated terms

Retain directed integer relations and limited observations. Add offers with exact
proposer/recipient, kind, terms, creation/expiry tick and identity. Offer kinds are
Transit access, Resource supply and Mutual defense. A civilization has at most
one outgoing pending offer; global pending capacity is64. Propose/accept/refuse
only at the100-tick diplomacy boundary using the immutable post-phase7 permitted
planning capture shared with phase9. Central admission protects category0 plans
before older pending-offer consideration; responses then use recipient-ID/offer-ID
order against leftover capacity. New terms do not recapture/rescore actor plans. New offers
are first considered at the next such boundary, never accepted immediately.

Offers expire at creation+300, end-exclusive. A refusal/expiry removes the pending
record and emits its exact reason. Missing/captured endpoint or participant,
incompatible war and exhausted obligations capacity make acceptance impossible;
they cannot alter a partially committed treaty or debit resources.

Acceptance rules:

| Offer | Eligibility | Terms |
| --- | --- | --- |
| Transit access | Both directed relations >=0; no war/truce; connected participants | Reciprocal movement through owned worlds/lanes for1200 ticks; no ownership or resource grant |
| Resource supply | Both relations >=0; eligible direct route; recipient known demand | One exact source/destination/resource route, batch1..10, cadence10, source reserve >=20; duration1200 |
| Mutual defense | Both relations >=40; no war/truce; connected participants | Reciprocal defense-request obligation for1200 ticks; no implicit offensive war or omniscient intelligence |

Resource supply's sender may use only stock above its own known next-needs reserve
and treaty route reserve; inability to dispatch creates an obligation wait fact,
not invented cargo. A later capture, war or capacity denial can suspend fulfillment
without changing previously dispatched manifest ownership/quantities.

The shared256 treaty account is exactly actual Agreement keys UNION derived
future Truce keys of retained Wars, plus distinct Commitment IDs; completed/import
State and projected creator prefixes obey the schema union/temporal predicates.
At capacity, offers remain pending until expiry with Capacity unavailable reason.
Autonomous material offers first obey scoring's exact category/deadline gates,
then integer utility, kind (Supply, Access, Defense), target and counterpart;
proposer/full identity break final ties. Supply's target is destination_world;
Access/Defense Terms name parties only and therefore their optional target is
absent. No phantom world is added to an offer solely for tie-breaking. Global merge is exactly scoring §7;
no second actor-first offer comparator applies. Supply utility is100 times the proposed
batch plus the minimum bilateral relation; it requires an explicit recipient
demand request, not access to its inventory. Access utility is100 times the count
of observed unowned worlds reachable through the proposed access, plus minimum
relation. Defense utility is100 times observed threatened owned worlds plus
minimum relation. Policy bonuses: Trader Supply1000/Access500; Guardian
Defense1000; Neutral Access500; Expansionist Access1000/Supply500; Industrialist
uses Neutral's Access500 and otherwise0. These remain Proposed weights. Clamp relation
contribution to0..100 and world counts to512. Evaluate all legal complete candidates from
the snapshot and reserve exact terms centrally. Offer Candidate uses the exact
ScoreComponents.offer mapping: Supply batch, Access reachable_worlds, Defense
threatened_worlds, other count fields0; min_relation/policy_bonus always explicit.
War/defender components use their exact objective-kind fields/zero rules, null
force travel0. Eligible-only trace and separate fixed central-denial reason order
are schema correction I4; utility weights are unchanged. Demand requests
carry actor/world/resource/units/tick and expire after200 ticks; the receiver
posts only its own shortage. Never disclose the receiver's complete stockpile.

Renewal requires mutual eligibility at a diplomacy boundary; never automatic
extension. Creator may set relations, create/remove exact commitments or cancel
offers through explicit revisions and complete incompatibility previews.

## Alliance intelligence and obligations

Mutual defense commitments form bilateral alliances; do not create transitive
alliances or automatic coalitions. An ally can request defense for one currently
owned threatened world it actually observes. One outstanding defense request per
commitment, cap256. Requests expire after200 ticks or when the threat/ownership
ceases. Sending records exact known enemy/target/timestamp and provenance.

The recipient can plan relief/defense without joining an offensive war silently.
If hostile action would require a declaration, produce a proposal subject to the
normal war admission rules and recorded refusal/acceptance. Existing third-party
commitments and own emergency needs take precedence; an unmet obligation remains
visible. No resource, hull or treaty can be reserved twice.

Intelligence sharing is limited to the exact requested target's observation.
Retain originating observer, observed tick and received tick. Receiving stale data
does not refresh its observed tick. For ordinary world facts own newer observations
win; equal-tick conflicts resolve by original observer ID. Environmental scope
assertions instead use the global per-scope selection in schema correction I1,
before world projection; contradictory provenance remains inspectable.
Sharing never copies private rival stockpiles or the whole omniscient galaxy.

## War objectives and resolution

War admission/root/queue validation derives the shared treaty account N=|AgreementKeys UNION FutureTruceKeys(Wars)|+|CommitmentIDs|<=256. No persistent lease field or fake Agreement exists. Exact creator overrides, generic removal cancellation, same-key past/touch/future Truce intervals and future-end/inherited phase1 exception are complete-schema §Derived future Truce capacity and mutation contract. Natural deadline phase2 or accepted peace actualE converts one reserved key to Truce[E,E+300) atomically; hidden capacity cannot block it. Autonomous declaration retains stage3 compatibility before capacity7, exact reserved=N_before and existing closed reasons; no legal121st pair exists under16 civs.


Add one primary objective to each war declaration:

- **Defend:** named owned world under observed hostile threat; success when no
  hostile force/occupation is present at three consecutive diplomacy boundaries.
- **Secure access:** named direct lane corridor; success when valid negotiated
  access exists or both corridor endpoints are owned by the declarer at three
  consecutive diplomacy boundaries.
- **Occupy:** named observed enemy world; success after completed ownership capture
  persists through the next diplomacy boundary.

Loss of the named reference invalidates the objective with an explicit result;
no replacement target is selected invisibly. A declaration still requires an
assembled reachable funded force, valid observations and treaty compatibility.
Defend outranks Secure access, which outranks Occupy; policy utility and stable
target ID break ties within a class; exact within-class formulas, force/endpoint/
path enumeration and ties are scoring §5. A candidate binds one whole eligible
idle fleet, complete sorted live hulls, departure world, arrival endpoint, selected
path and local departure-funding witness. Declaration reserves that force/account
transiently until publication but pays0 and starts no travel; later movement
revalidates current eligibility and pays once at actual departure. Record every
chosen/rejected candidate with complete AdmissionTerms and authenticated audits.

The declarer's War action/identity contains only its peer and own objective/force
terms. Central admission then selects the defender objective under the defender's
immutable same-tick phase7 capture, using the exact private objective/force ranking.
No valid defender objective rejects the unchanged attempted candidate and releases
its holds. A valid defender objective with no achievable force records null force
and force_unachievable; no fleet is invented. Attach both objectives atomically
before War publication. Only successful installation publicly discloses the
defender objective. Private defender force/accounts/capture contents remain in
defender-only audit projections; they never alter declarer enumeration/hash/ties.
WarAdmissionAudit binds that later selection to unchanged declarer identity. This
explicitly replaces the earlier actor-declaration two-objective requirement.
Creator ForceWar still supplies both objectives directly as a free override.

Objective success produces a peace proposal, not instantaneous unilateral peace.
Accept peace when the responder's own objective is complete/invalid, its eligible
observed force cannot achieve its objective, or war expires. War lasts1200 ticks;
peace/expiry converts its reserved key to a300-tick truce at actualE. Natural
expiry is phase2; completed/import War.end_tick>State.tick. A due inherited
phase1 row retains its reservation until exactly one conversion; new/replacement
ForceWar requires end>application and representable end+300. ForcePeace same-key
past intervals replace, touching/overlap coalesces and disjointfuture rejects
under schema; generic creator removal cancels its key without invented peace. For Occupy/Secure access, achievable means
one idle supplied reachable fleet's escort strength >=ceil(3*observed defense/2).
Escort strength is10 per live escort; observed defense is10 per observed defending
escort plus20 per operational battery. Attack observations must be no older than
300 ticks. Only own fleet HP/roles/supply and timestamped target observations enter
this comparison. Defend with a present threat remains achievable when such a fleet
can reach its owned target and has strength >=observed hostile escort strength;
no threat follows the explicit three-boundary success rule. War/truce and
occupation remain nonterminal; an empty or conquered galaxy stays editable.

## Fleet logistics and autonomous behavior

Every docked owned hull pays existing energy costs at departure. Efficient
logistics adjusts only that future departure cost. Existing paid return credits
and travelling manifests remain immutable. No global supply pool is introduced.

Add a fleet supply record: last supplied tick, supplying world and supplies paid.
At an owned dock every100 ticks, one energy per hull funds supply through the next
100 ticks; pay atomically per fleet in fleet-ID order subject to the due Food/
energy hold. No funds or held-needs denial records an unavailable latest attempt
with paid0; a NeedsReserved fact distinguishes physically held stock from missing
inputs. Preserve any prior successful Supply payment and its deadline; supplied
readiness lasts until that deadline even if the latest attempt failed, then becomes
supply_unavailable. Record this attempt at the current global100-tick supply
boundary; the next such boundary applies without an invented Supply clock/payment.
Without eligible payment retain Supply history and retain existing travel/return eligibility. Unsupplied fleets cannot
accept a new offensive attack objective; they may return, defend their current
owned world or complete already-paid travel. This prevents midflight disappearance
or a hidden extra fuel debit.

Relief uses actual resource supply routes and manifests. Blockade is a due
hostile fleet's explicit order at an enemy route endpoint, not a camera effect.
An idle supplied hostile escort fleet docked at the source or destination restricts
new dispatch to floor(batch/2) only when its live escort strength (10 per escort)
is at least the endpoint's live defending escort strength plus20 per operational
battery. Require declared war with that endpoint owner. Apply this factor once
even with multiple eligible blockaders, before event-ID-ordered hazard factors.
Scale the nominal restricted batch in event-ID order BEFORE the above-floor
stock cap, under schema Exact zero-freight transition. Earlier clock/suspension/
participant/path/permission/P>max(reserve,next_need) funding guards decide first.
Funded restriction-zero emits route_attempted actual0/reason null, consumes no
Shipment slot/ID/stock and grants no delivery/fulfillment/recovery credit. A
positive attempt still obeys capacity and actual paid provenance. Transient
first-batch holds use the supported first eligible phase10 boundary d, never
acceptance-time suppression or guessed resume; new Route capacity remains
independent. Never delete or resize in-flight cargo. The restriction receipt records fleet,
endpoint, eligible/restricted batch and actual dispatched quantity.

Fleet intent reasons expose target, objective, route, known threat, supply state,
reservation and fallback. On invalidated targets, release future reservations
without refunding already consumed travel/recipe inputs; emit Return/Wait fallback
using actual local eligibility. Boarding/passenger fate follows society rules.

## Creator, persistence and acceptance

Add typed offer/terms/objective/supply edits and complete removal/capture cascades.
Preview incompatible agreements, war changes, requests, fleets/routes and actual
resource implications. Creator War effects are exactly aggregate creator Change
facts in original operation mutation order; no duplicate autonomous war_ended,
peace/paid supply or negotiated fulfillment credit is emitted. Free creator overrides never masquerade as paid logistics
or negotiated actor acceptance.

Inspectors/Chronicle show terms, expiry, fulfilled/waiting obligations, known
intelligence, objective progress and fleet readiness. History/Notebook preserve
exact offer/treaty/objective/receipt references. A war-alternative experiment names
its common parent, creator diplomatic change, matched ticks and actual outcomes.

Required fixtures: offer delays/expiry, asymmetric relations, no connection,
captured endpoints, simultaneous commitments, starvation versus promised supply,
bilateral/nontransitive alliances, stale/shared contradictory intelligence,
third-party treaty conflicts, absent/failed objectives, interrupted occupation,
supplied/unsupplied attack/return, blockade versus already-dispatched cargo,
reservation scarcity, creator cascades, replay/save/transfer and native/wasm parity.

Independent canonical vectors and the reviewed V3 state/command schema are still
required before implementation. These proposed utility, force, expiry and blockade
values are explicit balance decisions for written review; they are not V2 changes.


## Creator order and trace corrections

SetFleetOrder explicitly submits objective after lane_path. Null clears/replaces
old binding deterministically; cancellation has kind/target/objective null and
empty path. Nonnull ObjectiveKey must match actual fleet owner, existing War pair,
its pending objective and compatible order target/path. Full exact allowed-kind/
target semantics are schema I5. ForcePeace/removal cancels pair-bound orders and
Blockades without changing an already-paid travelling leg or refunding inputs.
Reviewed previews must show these exact before/after Fleet effects.

Candidate trace context omits hidden State/revision provenance, but the full outer
audit authenticates it. Private defender changes may change admission/outer receipt,
never equal-input declarer trace bytes/digest. Known environmental freight factors
come only from bounded observed records, including actual creator-edited values.


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
