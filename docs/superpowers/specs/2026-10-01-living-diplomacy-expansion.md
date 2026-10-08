# Living V3 diplomacy, logistics and conflict

Date: 2026-10-03. Status: **PROPOSED coordinated review submission2; independent integrated review, fresh complete vectors and human approval pending**.
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
| Transit access | Both directed relations >=0; no war/truce; connected participants | Reciprocal nonoffensive entry to participant-owned Worlds for1200 ticks; public System/Lane transit needs no treaty; no ownership or resource grant |
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

## Inherited free Trade/Nonaggression handshake

These two legacy kinds are free bilateral protocol, NOT new three-kind Offers,
material Candidate/Trace/attempt or diplomacy lane fills. H follows older OffersO
and precedes PeaceK/remainingM. Freeze unordered H7 Active pairs a<b, max120;
consider Trade then Nonaggression. Both original actor inputs must independently
prove bilateral relations: Trade>=0 plus some valid own-Colony public graph
connection; Nonaggression>=20 with NO connection requirement. Same System suffices;
otherwise real public graph path. Missing Relation is exact source-defined0, not
foreign account access. Both captures remain full phase7 and do not leak to peer.
Retained same-kind Agreement, any retained same-pair War including future, or active
Truce excludes captured attempt; current gain cannot create a new H7 proposal.

For captured-eligible group revalidate current participants Active/references;
no retained same-kind key; no retained War then no active Truce; both actual totals;
Trade-only current valid Colony connection (alternate current own Colony pair may
replace removed captured witness); treaty union slot; checked T+1200 and output.
This is the exact free first-cause order. Old captured witness remains original
H7 historical proof, so its removal alone is not reference_missing if current
connection exists. Actual failure emits two rejected Decisions with one actual
cause each, no Agreement. Capacity uses admission_capacity agreements_commitments,
actor subject, limit256/reserved currentN; threshold uses each direction's actual
BilateralThreshold. No connection uses path_unavailable. Current gains never rescore.

Success installs exact Agreement kind/a/b/startT/endchecked(T+1200), no ID/AUTO,
resources, Relation credit, objective or witness. Group: a intent_decided, b
intent_decided, agreement_formed full one-key null→new Change/resources[]/delta0.
Each free Decision phase8/candidate null/score0/tierordinary/targetSome(peer)/
observed_ticknull/original H7/actionexactkind; success reasons BilateralThreshold
then Accepted mutual_trade_with_connection|mutual_nonaggression. Current guard
facts do not claim recomputed material utility. Failure has exact one reason.
Actor/phase8 Decision ordinal includes earlier real free/material Decisions; H
fills no lane and emits no CandidateAudit entry. Change actor-null source is real
full pre-pair current prefix. No self Receipt/Event source.

Active/future same-kind keys are never extended/replaced. Actual phase2 expiry
removes once; at next100 H can mutually REFORM after fresh eligibility, with real
off-grid gap and no backdated rights. Trade/Nonaggression/Truce keys cannot coexist
with ANY retained same-pair War, even future. Autonomous War cannot delete them:
actual H can block laterM, real category0R War blocks H. Creator ForceWar explicitly
removes every same-pair legacy key in reviewed closure then installs/replaces War,
counts future Truce once and executes active-now C/Request/order causes only if
start<=T. Future install leaves current C rights until genuine phase2 crossing.
Reciprocal creator SetAgreement explicitly removes same-pair War/bound invalid
orders/Blockades before installation/recovery; no silent future lease theft or
restoration of old legacy keys. Continuing C has independent ID/rights/capacity.

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

Objective terminal outcome can propose Peace, never unilateral conversion.
K freezes actual ascending War-pair roster and entry Peace per pair. Older
Proposed.created<T receives one response; none selects fresh terminal proposer,
refused selects eligible semantic-change retry, created==T receives no response.
Choose deterministic lower eligible proposer ID; newly refused never retries
in that same K pass. Genuine same-actor objective outcome
and original H7 classify achievable force TRUE/FALSE/UNKNOWN, never unknown defense0.
Terminal succeeded/invalid or proven FALSE accepts. TRUE with actual matching
eligible force refuses; TRUE blocked solely by actual strict recovery owner/holds
DEFERs unchanged. UNKNOWN DEFERs unchanged. No consideration tick/event is invented
for defer. A real refusal retains proposer/created, sets consideredT/statusrefused/
reasonobjective_still_achievable and emits one exact peace_refused; original full
Facts.peace record and installed reason must match. Natural expiry is phase2. War lasts1200 ticks;
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

Fresh Peace uses reasonobjective_complete or objective_invalid from actual terminal
outcome, proposed status/createdT/considerednull, one peace_proposed full installed
record. Existing Proposed is never overwritten. Refused retry compares genuine
semantic original refusal baseline: same objective instance, target observation
membership/strength/freshness class, actual original whole force/path/funding
eligibility. Ignore raw timestamp/receipt/pointer/evaluation transport refresh alone;
changed real predicate permits new proposal, no same-pass response. Actual War
end atomically removes War/order/Blockade/occupation consequences, forms reserved
Truce and exact continuing Request/Fleet/C groups, no fake universal two-event count.
Unknown source replay remains Pending/fault, not force_unachievable success.
Every genuine creator SetWarObjective resets the exact instance to pending/count0/
evaluatednull/witnessnull and clears Peace even byte-equal replacement. Creator
ForceWar constructs both sorted participant objectives reset likewise, peacenull,
checked end=start+1200, end>application and representable end+300. Pending Secure
submits exact live directional pair; other kinds null. No creator constructor may
submit arbitrary success/witness, borrow old samples or masquerade as negotiated
fulfillment. Paid location/Hulls/counts/credits stay intact under bound order cleanup.

## Fleet logistics and autonomous behavior

Ordinary paid Fleet motion is one origin-to-FINAL-World itinerary. A distinct
same-System target takes10 ticks with empty path; otherwise use minimum checked
sum of direct Lane durations max(10,ceil(distance_units/10)), tie by lexicographic
decoded Lane-ID sequence. Intermediate Systems/Lanes are public unowned graph
transit, with no docked waypoint, event, refuel, combat or continuation cursor.
Paid duration/path/roster/payment remains authenticated by its original departure
producer after future order edits. Later geometry, permissions or technology cannot
halt/reprice it. Actual required endpoint removal uses explicit reviewed traveller
relocation/destruction, never invented nearest placement, refund or fake credit.

At actual owned nonReturn launch each Hull pays base2 Energy plus2 iff missing
return_credit. Efficient Logistics applies ceil(3*base/4), minimum1, PER HULL,
then checked sum in Hull-ID order. Two credited Hulls therefore pay4, not aggregate
rounded3; this is an explicit NEW Proposed balance recommendation. Debit once,
grant each Hull credit once. Return/exact credited Dormant Ark recovery pays0 and
consumes every Hull credit once for the whole itinerary. Already paid travel never
pays again. Shipment's immutable original-duration return is a separate producer.
No global pool or forecast output funds a real departure.

Actual ownership comes from World.owner: own iff Some(actor), unowned iff null,
foreign iff Some(other actual issued participant). Colony absence never changes
that category. Separate OwnedColony requires matching World and Colony owner and
remains mandatory for Return/owned launch where prescribed. Missing World or nonnull wrong-kind/absent owner ID
is a typed fault; owner=null is legitimate unowned. Actor formation uses own full World records or its authorized
fresh ownership observations; absent/stale knowledge is UNKNOWN, not null.
Actual authority may deny from real current ownership without enriching H7 scores.

| Destination/order | Entry permission |
| --- | --- |
| Move/ordinary Explore | own/unowned allow; foreign requires active usable bilateral Transit |
| Pure-scout Explore | nonoffensive discovery entry to any live World without asserting owner/Transit |
| Settle | actual unowned target plus its own known habitability/Ark/claim purpose guards |
| Return | actual OwnedColony destination plus whole credits |
| Unbound Defend | actual own target plus its owning defense predicates |
| Request-bound Defend | exact accepted live original R/C/threat and requester-owned named target |
| Attack/Blockade | actual foreign enemy owner, active matching War and no active Truce |

Transit grants no offense, settlement, stock, intelligence or ownership. Trade,
Nonaggression, Supply, pending/future Transit and other Requests do not substitute.
An accepted original bound Defend authorizes only that named nonoffensive entry,
not generic allied Transit or declaration against its enemy. Required instance/
typed references are checked first, destination-purpose ownership second, public
topology third, then Truce-before-War offense or exact Transit/duty permission.
Each first failure carries its finite schema witness, not guessed rival data.

PureScoutRoster requires the entire nonempty whole Fleet to have scout-role Hulls,
count0 and voyage_origin null. It contains no idle/order/location/hold predicate.
H7 formation separately requires own Docked idle/funded/compatible whole Fleet and
public target/path, with objective/request null. Unknown destination ownership never
blocks that exact discovery family. Actual due Explore reuses current roster/kind/
null bindings, not order=null; structural holders do not revoke entry and block only
at their later guard. Changed mixed roster falls back to ordinary actual permission;
central scores/identity stay original and standing dispatch creates no Decision.
Discovery gives no combat immunity, occupation, cargo or offensive rights.

Attack/Blockade and travelling Defend require supplied. Move/Explore/Settle retain
nonoffensive travel eligibility without new military Supply requirement. Return,
exact Dormant credited recovery and already paid travel allow unsupplied. Stationary
own-world unbound Defend keeps its exception; this is no remote relief waiver.

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
