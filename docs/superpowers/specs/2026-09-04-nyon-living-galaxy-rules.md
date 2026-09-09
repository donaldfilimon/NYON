# NYON Living Galaxy: proposed V2 rules and strategy

Date: 2026-09-04

Status: Approved implementation contract on 2026-09-04; not yet implemented or balance-tested.
Amended 2026-09-08 on the owner's decision to close the three normative gaps raised in
`docs/superpowers/reviews/2026-09-06-living-authority-contract-gaps.md`: the receipt payload record and
its derivation order, the historical-versus-allocator split for `accepted_sequence` and `branch_sequence`
with two new archive fields, and the published phase and intent ordinal registries with two new queue
limits. Two amendments differ from that proposal: phases are numbered from 1 rather than 0, and the
`LivingEventKindV2` ordinal is deliberately left unassigned. No previously frozen vector is invalidated
by that first amendment.

Amended again 2026-09-08 on the owner's decision, closing a fourth gap found while preparing the state
schema: this document named `LivingGalaxyStateV2` without ever listing its fields, so the only ordered
field list lived in a downstream consumer plan. That list is promoted here, `counters` is defined as a
reserved empty record, and the entity-kind registry is assigned freely rather than pinned to the values
already present in the test corpus. **That last decision will invalidate frozen vectors**: the
`entity_kind` values in `crates/nyon-workshop-core/tests/fixtures/living-v2/vectors.json` are renumbered
when the registry lands, so the `creator_entities` and `autonomous_entities` rows and their digests are
re-derived. The commit that renumbers them must say so here. Everything frozen that carries no
`entity_kind` — the domain literals, the payload digests, and the revision, branch, event and claim rows
— is unaffected

**Renumbered 2026-09-08, discharging the obligation in the sentence above.** The entity-kind registry
landed in `2f6561a`, "feat(living): publish the V2 entity-kind and phase ordinal registries", the first
of the three commits that closed the commands-and-receipts task. It renumbered five `entity_kind`
values in the reviewed corpus: both `creator_entities` rows moved from `3` to `1` and now name the two
systems a topology batch creates, the two `ai_fleet_phase_8` rows moved from `4` to `10`, and
`route_shipment_phase_9` moved from `7` to `13`. Those five rows' `entity_digest` and `entity_id` values
were re-derived outside the implementation crate with `tools/living-v2-vectors.py`, which transcribes the
formulas of this section rather than calling the Rust code. No other byte of that corpus changed: the
domain literals, the payload digests, and the revision, branch, event and claim rows are the values they
have always carried, and `tools/living-v2-vectors.py verify` recomputes every one of them from this
section's formulas

Parent: [Product and architecture](2026-09-04-nyon-living-galaxy-design.md)

## 1. Authority and release envelope

Living Galaxy V2 is a new deterministic rule set. Classic RulesV1 and WorkshopV1 retain their original types, phase order, byte formats, and golden digests. None of the numerical values below changes V1 behavior.

The creator is omniscient and has no resource budget. Civilizations have local resources, limited intelligence, and explicit policies. There is no winning faction, terminal conquest count, or creator elimination. An empty galaxy continues to accept commands and advance time.

Default procedural content: 12 systems, 24 worlds, four civilizations, a connected lane graph, and one founding colony per civilization. The curated Vale Confluence starter is a separate 4-system/8-world/3-civilization fixture. A colony is an owned world with a hub; not every world must have a colony.

Proposed authority hard limits:

| Collection | Limit |
| --- | ---: |
| Civilizations | 16 |
| Systems / stars / worlds | 64 / 128 / 512 |
| Lanes | 256 |
| Deposits / industries | 1024 / 2048 |
| Routes / shipments | 2048 / 4096 |
| Fleets / total hulls / hulls per fleet | 256 / 2048 / 16 |
| Active or scheduled hazards | 128 |
| History branches / creator revisions | 64 / 10000 |
| Creator-batch operations | 1024 |
| Pending running-queue depth per boundary | 1024 |

The Living V2 persistence adapter separately allows 64 saved slots, 256 registered catalog packs, 32 MiB per archive, and 2 MiB per pack. These do not alter WorkshopV1's smaller byte/slot limits.

The two queue limits are set to the declared work-unit poll bound rather than to an arbitrary round number, so a single batch or a single boundary's queue can never exceed one poll's budget. Both reject atomically at submission, before any sequence, ordinal, entity ID, queue mutation, or resource is consumed.

Capacity checks are atomic. Accepting a building or hull job reserves its global entity capacity and its destination industry/fleet slot through completion. A hull job names an existing docked fleet or reserves creation of a new docked fleet; the target must still be valid at completion. A fleet with a pending hull-delivery reservation cannot depart, split, merge, or accept another mutually exclusive reservation until completion or cancellation. Validate and reject such an order before changing its existing order, job, credits, or energy. Combat destruction or capture of the target cancels the job using the existing half-refund and atomic capacity-release rule. Cancellation/capture releases reservations atomically. Creator operations count all outstanding reservations. Reject the relevant creation, construction, or dispatch before spending resources; existing objects keep working. Reaching a history limit offers export/new-document guidance, not silent history deletion. An allocation/arithmetic invariant failure preserves the last valid state and pauses with a diagnostic.

## 2. Tick and command contract

Run at 10 authoritative ticks per second. `state.tick` is the number of completed tick boundaries. A step from `t` to `t+1` evaluates boundary `T=t+1` and commits the entire candidate or nothing.

Paused creator commands apply synchronously at the current boundary without advancing time and require an empty running-command queue. Running commands apply before simulation phases at the next boundary. Every envelope records `expected_committed_revision`, `expected_tick`, and `expected_pending_sequence`; the last value is zero when the queue is empty. Submission compares all three with the authority's published committed cursor and current queue tail. A mismatch rejects as stale without allocating an ID or changing the queue.

An accepted running envelope receives the next document-global monotonically increasing `accepted_sequence` and is validated against a private projection containing all earlier accepted envelopes for that boundary. The counter is persisted and never reused after queue clearing, a fault, Undo, or branching; a rejected submission consumes no sequence. Its logical parent is the prior accepted envelope, or the committed revision if first. At boundary T, replay the whole accepted sequence into one candidate in sequence order. Each envelope becomes one immutable revision whose parent is the revision produced immediately before it; a multi-operation atomic edit is represented by one batch command/envelope. If any supposedly accepted envelope cannot reproduce its validated result or an invariant fails, commit neither the tick nor any of its queued revisions, retain the queue for diagnosis, and pause with a typed fault. Ordinary validation failures happen at submission and never enter the queue. After a successful boundary commit, clear the applied queue and publish its final revision/tick/tail-zero cursor. This defines enqueue stale checks, same-boundary chaining, and application atomicity without silently rebasing commands.

`accepted_sequence` and `branch_sequence` name two different things and the distinction is normative. The values carried inside `LivingGalaxyStateV2` are **historical**: they are frozen at the moment that state was produced, are never mutated by later activity on sibling branches, and are reproduced exactly on restore and replay, which is what keeps an older branch's `state_digest` reproducible. The engine separately maintains **durable allocator high-water marks** for both counters. Those marks are never part of any single `LivingGalaxyStateV2` and never inputs to `state_digest`; every new submission or fork allocates from them regardless of which branch is currently viewed. They are persisted independently of the committed revision graph, because a faulted-but-retained queue has already spent sequence values that never became committed revisions, and a mark recomputed by scanning the graph would under-count and reissue them.

For revision identity, `tick_u64` is the application boundary: `t+1` for a running envelope accepted at completed tick t, and the unchanged current boundary for a synchronous paused envelope. `ordinal_u64` is that envelope's document-global `accepted_sequence`. A paused envelope also receives the next sequence. These meanings do not change after replay or branch creation.

Entity IDs derive from versioned seed/state/actor/command identity, not UI labels, wall time, rendering order, or hash-map insertion order. Section 10 pins canonical bytes and hash framing.

Each step orders work as follows:

1. Apply queued validated creator interventions for T in recorded order.
2. Expire agreements/truces/wars and update hazard boundaries.
3. Resolve travel arrivals and freight delivery/return/capture.
4. Resolve automatic noncombatant retreat departures, due combat rounds, then occupation and settlement claims.
5. Complete construction; run hubs, solar, extraction, processing, and eligible repairs in stable entity order.
6. Refresh civilization observations from current visibility.
7. On multiples of 100, update diplomacy from one shared snapshot and resolve proposals.
8. On multiples of 50, generate economic and fleet intents from one shared snapshot; validate and reserve centrally.
9. Dispatch previously scheduled fleet orders due at T and eligible freight routes. Newly accepted fleet orders cannot depart before T+1.
10. Produce ordered event receipts, update counters and next-due times, hash and commit T.

Due recipes use stored next-due boundaries, not floating-point elapsed time. Every due attempt advances `next_due` by exactly one period whether it runs or is blocked; there is no catch-up production. Store the latest blocked reasons separately for inspection. Genesis at boundary G initializes each clock to G plus its period: hub energy, solar, and extractor at G+10; foundry at G+20; hub ore/fallback at G+100. Creator-created already-built facilities use the same rule unless a validated import supplies canonical clocks. Construction accepted at T for duration D completes at T+D, after D subsequent tick advances. A completed facility's first recipe is due at completion plus its period. It never produces on its completion boundary.

## 3. Economy, construction, and recovery

Use three resources: **energy**, **ore**, and **alloy**. Inventory is world-local and capped at 10000 units per resource. No empire-wide invisible pool exists. A recipe runs only if every input, reserve, and output-capacity condition holds; otherwise nothing is consumed and the inspector reports the reasons.

Each colony has a persistent hub, six industrial slots, one building queue, and at most one shipyard with one hull queue. A defense battery consumes one slot and is limited to one per colony. Reserved building slots count toward the six-slot limit. Creator-created noncolonized industry can exist, but autonomous construction requires an owned colony.

All civilization construction pays alloy up front. Jobs with invalid targets, full queues, inadequate alloy, or exhausted caps are rejected before deduction. Civilizations cannot build a replacement whose role already exists merely to bypass a queue.

| Facility | Alloy | Build ticks | Recipe/effect |
| --- | ---: | ---: | --- |
| Solar array | 20 | 100 | 8 energy every 10 ticks |
| Extractor | 30 | 150 | 2 energy and 4 deposit units become 4 ore every 10 ticks |
| Foundry | 40 | 200 | 4 energy + 4 ore become 2 alloy every 20 ticks |
| Shipyard | 60 | 300 | Builds one hull at a time |
| Defense battery | 40 | 200 | 40 HP; 4 damage each combat round |

Extractor operation requires a matching ore deposit with at least four remaining units. A final reserve below four remains visible and unused; no rounding creates extra ore.

### Hub baseline

The hub is not a combat target and persists through occupation. Each hub produces 2 energy every 10 ticks and 1 ore every 100 ticks. Its explicit fallback fabricator may consume 4 energy + 4 ore for 1 alloy every 100 ticks while local alloy is below 120. On a shared due boundary, apply hub energy, then hub ore, then evaluate fallback against that resulting inventory; each operation respects capacity. Fallback advances its clock even when blocked.

The ore trickle represents the colony's documented low-throughput scavenging capacity, not depletion-free industrial mining. The UI names it separately from the finite deposit. This prevents a surviving colony from permanently losing the ability to rebuild after its fast economy fails. No secret AI grants or automatic resurrections are allowed.

With no industry, no deposits, and empty inventory at tick zero, the hub first fabricates one alloy at tick 400. It can eventually fund any single facility or hull. This is intentionally much slower than restoring a supplied foundry.

### Starting assets and lifecycle

A procedural home starts with hub, solar array, extractor, foundry, shipyard, 100 energy, 100 ore, 120 alloy, and an ore deposit of 20000 units. It also starts with one scout and two escorts in one docked fleet; a civilization may split that fleet through its normal validated intent. These assets are declared genesis content, not newly spawned by AI.

Scrapping an owned facility or cancelling construction returns `floor(paid_alloy/2)` to its world. Excess refund above storage capacity is explicitly discarded with a receipt. Creator overrides can remove or replace assets without refunds, but the preview and command must identify that action as a creator override.

Capture preserves the hub, inventory, and surviving facilities. It cancels the old owner's building and hull queues with the same half-alloy refund into the captured world. It never silently deletes finite deposits or duplicates paid assets.

Outside combat, each damaged docked owned hull and owned battery can repair one HP for one local energy. First damage at D sets `next_repair=D+10`; additional damage does not postpone it. Every due attempt advances the clock by ten even when energy-blocked. Clear it at full HP. Repair uses stable ID order, cannot exceed maximum HP, follows production in the boundary, is visible in projected energy demand, and has no free regeneration.

## 4. Ships, orders, and intelligence

| Hull | Alloy | Build ticks | HP | Damage |
| --- | ---: | ---: | ---: | ---: |
| Scout | 10 | 50 | 10 | 0 |
| Colony ark | 60 | 200 | 20 | 0 |
| Escort | 20 | 100 | 10 | 2 |

Fleet orders are explore, move, settle, defend, attack, and return. Splitting/merging is permitted only for co-located, same-owner hulls while stationary; it preserves every hull's identity, HP, and fuel credit. Hulls cannot merge while travelling or escape a combat round through reorganization. Hull completion cannot produce a same-boundary departure.

### Travel and energy

All hulls share travel speed. A direct lane leg takes `max(10, ceil(distance_units/10))` ticks. Worlds in the same system use a ten-tick local transfer. Inter-system fleets choose the shortest sum of travel ticks, with lexicographic lane-ID-path tie-breaking; endpoint world travel has no additional hidden duration.

New launch from an owned colony costs two energy per hull for the outbound itinerary, plus two per hull without a return credit. A new two-hull fleet therefore costs eight energy; one whose two hulls already have return credits costs four. A return credit covers one whole itinerary back to an owned colony and is consumed per hull when that return departs. Credits survive splitting/merging and die with the hull. No energy refund or continuous fuel attrition applies.

Away from an owned colony, a fleet may complete its already committed multi-leg itinerary or return using its credits; a fresh unrelated excursion requires refuelling at an owned colony. One recovery exception applies to a dormant civilization: an ark fleet may consume its existing return credits for one itinerary to a known unowned settlement destination. This creates no energy or new credits. A stranded ark without sufficient existing credits requires creator intervention or an earlier branch. Movement on an already departed leg retains its endpoints and duration. At the next waypoint, revalidate the remaining route; if disconnected, hold and report No route. Creator removal of a referenced endpoint must explicitly resolve affected travellers as part of its atomic batch.

This release has no lane blockade or mid-lane battle. Foreign systems are traversable. Combat and occupation occur at world destinations, not arbitrary transit waypoints.

### Civilization intelligence

The creator sees all state. Civilizations know static world/system/lane geometry, but dynamic ownership, deposits, facilities, and hulls require observation. Store each world's observed tick, owner, remaining deposit, facilities, and stationed hulls.

A colony or scout reveals all worlds in its system. Other fleets reveal only their current world. Refresh observations every tick after combat/economy, before diplomacy/AI. Old observations persist with their original timestamp after visibility ends; hidden changes do not update them. Rival stockpile quantities are never revealed to AI. Own inventories and the civilization's contracted freight are known.

Scouts prefer never-observed systems, then the oldest observation, then shortest route and stable system ID. War and hostile launch require target observation age at most 100 ticks. Intelligence becoming stale after departure does not cancel the trip. The inspector distinguishes live creator truth from what the actor actually knew.

## 5. Settlement and freight

### Settlement

An ark may claim an observed unowned world reached through valid travel. A first eligible arrival at boundary A opens the half-open 50-tick claim window `[A,A+50)`, whose close boundary is C=A+50. Other claims arriving through C-1 participate; an arrival at C does not join the closing arbitration and may start a later window if the world remains unowned. On C, choose the lowest `hash(version, seed, world_id, claim_close_tick, civilization_id)` among claimants still present and eligible, with civilization ID as collision tie-breaker. Consume one winning ark and create an owned hub with 20 energy, 20 ore, and zero alloy.

Losing arks survive with Claim lost and can return. Multiple arks from one civilization are one claim. A hostile armed presence suspends settlement and clears the pending window; a new peaceful window begins only when eligible again. A world becoming owned through a creator command invalidates all outstanding claims before arbitration. Zero-cost settlement cannot be replayed for free inventories because the ark is consumed and ownership is checked atomically.

### Freight

Freight routes connect two worlds in the same system or worlds whose systems share a direct lane. Longer chains need explicit intermediate inventories/routes. Every route records source, destination, resource, batch size up to ten, cadence ten ticks, source reserve at least 20, and participating owners. A dispatch transfers `min(batch, available_above_reserve, hazard_adjusted_batch)` units; zero dispatches create no shipment. Routes operate in stable ID order after local production and AI reservations.

Trade between civilizations requires a current bilateral trade agreement at dispatch. Owned internal routes need no agreement. A creator may configure an explicit one-way Aid route without an agreement; it is labeled aid, cannot take another civilization's stock, and each completed 100-unit aid tranche can affect recipient relations as defined below.

Travel duration follows the same direct-lane formula as fleets. The built-in ion storm multiplies the route's per-dispatch batch by one-half, rounded down, during `[start_tick,end_tick)`. Multiple storms on a lane use the strongest reduction, not compounding. They affect new freight dispatches only; no in-flight duration changes. This is an explicit V2 rule, not Classic's Hydro hazard rule.

Every route records the source and receiver owners whose permissions validated it. Suspend it whenever either endpoint's current owner differs from those participants. Dispatch always revalidates current source ownership and current bilateral permission before deducting inventory. A suspended route resumes only after a newly validated adjustment by the current source owner; capture never silently transfers the route. Existing shipments retain their immutable manifests.

Each shipment records dispatch owner, intended receiver, units, departure/arrival ticks, and return status. At arrival and every waiting retry, first determine capture, friendly delivery, or one-time return from current ownership/war state and the immutable manifest. A required return departs immediately, regardless of destination capacity. Only then check the chosen delivery/capture recipient's capacity. On unloading:

1. If destination owner changed, its current owner receives the cargo and Shipment captured is recorded. An unowned destination keeps it as local inventory with Shipment delivered to unowned world.
2. If ownership is unchanged and the parties are not at war, deliver. Treaty expiry alone does not invalidate cargo already sent.
3. If ownership is unchanged but the parties are now at war, return once over the same duration, at no additional resource cost.
4. Returning cargo unloads at its original source, even if ownership changed; record capture there when applicable. Never bounce repeatedly.

If the selected delivery/capture recipient's storage is full, keep the shipment at the destination as Waiting for storage, retrying each tick and re-evaluating disposition before capacity. It continues to consume shipment capacity. Add trade/aid relationship credit only when inventory transfer succeeds; captured or discarded cargo adds none. Deliver atomically when space exists; the creator can explicitly discard it with a loss receipt. Removing a referenced world requires a previewed cascade that resolves its cargo as recorded loss or reroutes it; ordinary deletion rejects dangling references.

## 6. Diplomacy and bounded conflict

Relations are directed integers in [-100,100]. Each direction has a creator-set base disposition plus bounded reason counters. The total is their clamped sum. A civilization's own declaration does not make it resent the victim; the victim receives the declaration penalty.

| Reason | Directed effect |
| --- | --- |
| Delivered aid | Recipient toward sender +10 per 100 delivered units; counter cap +20 |
| Successful trade | Receiver toward sender +1 per 100 delivered units; counter cap +20 |
| Adjacent rival settlement | Existing neighbor toward settler -10 per new adjacent colony; floor -20 |
| War declared against it | Victim toward declarer -30; floor -30 |
| Colony captured from it | Former owner toward captor -30 per colony; floor -60 |

Adjacency means same system or one direct lane between systems. Every 600 ticks, each event counter moves one point toward zero; creator-set disposition does not decay. Counters and partial 100-unit delivery accumulators are canonical state. Destroyed/lost freight does not count as delivered trade or aid.

Trade is eligible when both directed relations are at least zero and a valid connection exists. Nonaggression is eligible when both are at least 20. Agreements last 1200 ticks with end-exclusive intervals and renew only through mutual eligibility at diplomacy boundaries. No unilateral treaty breaking or alliance subsystem exists. War/truce blocks new trade and nonaggression; ordinary trade and nonaggression also prevent war while active.

At a diplomacy boundary, expire records first, then apply delivered-event counters/decay, resolve mutually eligible agreements, and finally eligible war proposals. Use the same post-update snapshot for all proposals. A civilization may propose at most one new war per boundary; select its lowest relation, then target world ID and opponent ID. Existing wars continue until their expiry; identical bilateral proposals create one war.

War eligibility requires directed relation at most -20, no bilateral agreement/truce/war, a rival colony adjacent to one of the declarer's colonies, fresh target intelligence, and an assembled eligible escort fleet satisfying `2 * attack_strength >= 3 * observed_defense_strength`. At least one attacking escort is required. Nominal strength is ten per escort plus twenty per operational battery; scouts/arks contribute zero. This is an explicitly labeled heuristic, not an exact battle prediction. Queued hulls or distant fleets do not count as assembled attack strength.

Wars last 1200 ticks, then automatically become a 600-tick truce. No conquest target or negotiation deadlock prolongs them. A declaration at tick 100 may produce an AI attack order that boundary, with departure no earlier than 101. AI evaluation at tick 50 cannot declare war.

The creator can set a relationship or agreement and can force a war/peace override. Such commands identify the override and its affected bilateral state, atomically resolve incompatible agreements, and produce Creator intervention events rather than pretending the civilizations negotiated. Forced peace stops new damage at its application boundary and resets occupation progress.

### Battle and occupation

Only declared enemies fight. Combat occurs on global multiples of ten. Read all attackers and targets from a pre-round snapshot. Each armed hull/battery targets the lowest-ID hostile escort, otherwise battery, otherwise noncombatant. Sum damage and apply simultaneously; remove zero-HP assets after all attacks. Multiple attackers may overkill the same target; no retarget after a simultaneous hit.

At the start of a due combat phase, an unescorted noncombatant fleet attempts immediate return using its prepaid credits, before the pre-round target snapshot is taken. Select the reachable owned colony with minimum total travel ticks, then lowest world ID; select its path with the existing lexicographic lane-path tie-breaker. A successful return departs at that boundary and is not a target in that round. This is the sole immediate-departure survival exception; normal AI and creator movement orders depart no earlier than the following boundary. No reachable owned colony, invalid path, or missing credits yields Retreat blocked without consuming credits. Automatic noncombatant retreat does not wait for the 50-tick strategy cadence.

For occupation, a claimant must maintain an escort at an enemy colony with no defending escort/battery. More than one foreign claimant, or any third-party fleet hostile to a claimant, makes the world Contested. A unique eligible claimant increments occupation once per completed tick; any invalidating condition resets progress. Transfer at 100 consecutive eligible ticks. Claims do not begin during a treaty/truce, and expiry into truce resets them.

Loss of a civilization's last colony makes it Dormant, not deleted. It retains identity, policies, relations, history, and surviving fleets. Dormant actors can return/move existing fleets and use surviving arks, but cannot construct without a colony. Resettlement restores Active status. The creator can restore a colony or branch to an earlier future. Even loss of every civilization leaves the sandbox running.

## 7. Explainable autonomous choices

Every 50 ticks each active civilization may accept at most one economic intent and one fleet intent. Dormant civilizations run the fleet portion only, limited to completing/returning existing journeys and the ark recovery exception; they generate no economy or new assets. Generate candidates from the same visible snapshot for all actors, then centrally reserve resources, queue space, and targets. Reject conflicts without spending; record the reason. Stable actor ID resolves a true shared resource conflict, while settlement has its separate claim arbitration.

Economic emergencies take precedence only when actionable: restore missing power supply, then inaccessible ore supply, then missing alloy production. Project the next 200 boundaries by applying existing scheduled recipes, routes, repairs, and paid queues in exact phase order to a private integer forecast. Already-paid queued costs are not deducted again. Queued facilities contribute only after actual completion and first-recipe boundaries. Include hub production and committed freight; do not count speculative trade, unknown rival stock, or the resource cost of unaccepted desired jobs. An emergency is a projected deficit against known scheduled demand in that horizon. An unaffordable or impossible emergency does not suppress other legal candidates.

Outside emergencies, score legal candidates:

| Economic intent | Base score |
| --- | ---: |
| Supply an owned colony below 20 of a needed resource | 70 |
| Build extractor on observed usable deposit | 50 |
| Build foundry with projected inputs | 50 |
| Build ark for a currently eligible settlement target | 50 |
| Build escort while below defense target | 50 |
| Build solar array | 40 |
| Build scout while fewer than two exist | 40 |
| Build missing shipyard when a legal hull job is needed | 55 |
| Build missing battery at a threatened owned colony | 55 |

Do not build solar or processing capacity merely because it is affordable: the projected demand/output-storage test must show a use. Defense target is two escorts per owned colony, including already reserved hull jobs. A desired hull without a shipyard generates the shipyard candidate, not a permanently failing hull action. At a full six-slot colony, replace an idle/unsupplied nonessential facility only through an explicit scrap-then-build plan that preserves its hub and last usable production chain.

Policy bonuses: Expansionist +20 ark/settlement; Industrialist +20 extractor/foundry; Guardian +20 escort/battery/defend; Trader +20 supply/trade-route establishment. Neutral gets no bonus. Bonuses do not override legal conditions or emergency tiers. Tie order is score descending, target ID, then fixed action-kind ordinal.

Fleet priorities: actionable defense of an attacked owned colony, then noncombatant retreat. Otherwise score settlement 60, authorized attack 60, exploration 50, and reinforcement of an undefended owned colony 40, with applicable policy bonuses and stable ties. An eligible idle fleet can split co-located scouts/arks/escorts for an intended role; splitting and the resulting order are one reserved fleet intent.

When source stock above reserves and receiver deficit make a transfer useful, AI establishes or adjusts an internal direct route; with bilateral agreement it can establish a cross-civilization trade route under the same conditions. Autonomous aid is limited to a Trader sending a surplus needed by a nonhostile neighbor, scored as supply with the policy bonus. Never inspect rival private inventory: use an explicit receiver demand request generated from its own known state and exchanged through the agreement/aid proposal.

Every accepted/rejected intent records actor, target, action, chosen score/tier, relevant observed tick, and structured reason. The chronicle can explain **building a foundry because alloy demand exceeds projected output**; it must not invent motives when no reason exists.

## 8. Creator tools and strategic experiments

V2 creator commands cover creation/editing of topology and archetypes, finite deposits, inventories, colonies/ownership, facilities and queues, civilization policies and base relations, agreements, fleets/orders, routes, and scheduled hazards. All are bounded and validated; free editing does not allow illegal IDs, negative resources, cyclic ownership, orphaned routes, or halfway-applied cascades.

The command preview names affected objects and distinguishes a direct creator grant from a resource-funded civilization order. Deleting a world or civilization with dependents requires an explicit reviewed cascade; reject ambiguous deletion. Civilizations normally become Dormant through gameplay rather than being deleted.

Strategic loops to expose through real systems:

1. **Ore frontier:** finite deposits lead to scouting and settlement; distant supply needs transport; trader and expansionist policies choose different solutions.
2. **Arms versus factories:** alloy spent on escorts cannot also fund foundries or arks. Protected trade can outperform conquest; poorly supplied occupation can slow the victor.
3. **Neighbors become partners or rivals:** nearby settlement produces friction; deliveries improve relations; wars consume fleets and then must stop for a truce.
4. **Resilience versus concentration:** a centralized foundry is efficient to supply but vulnerable to a lost route; distributed colonies recover more slowly yet retain local hubs.
5. **Information before commitment:** a cheap scout can refresh defenses before an expensive escort/ark expedition. Creator omniscience does not give AI hidden target data.
6. **What-if authorship:** pause before a diplomatic or logistics turning point, branch one intervention, and compare exact states after the same number of ticks.

No strategy is promised to be dominant. Numerical balance is provisional until seeded play and endurance fixtures demonstrate understandable, non-stalled behavior across all policy presets.

## 9. Exact rule fixtures

Disable unrelated actors, repairs, and recipes where a fixture isolates one mechanism. Assertions below are planned tests, not observed results.

| Fixture | Required result |
| --- | --- |
| Building timing | Solar accepted at 0, duration 100: completes at 100; first 8 energy at 110, none at 100-109 |
| Hub recovery | Empty hub at 0, no deposit/industry/AI: first alloy at 400; fallback can accumulate beyond 20 toward 120 |
| Atomic shortage | Due foundry with 4 ore/3 energy consumes nothing; 4 ore/4 energy yields 2 alloy and consumes both inputs |
| Blocked cadence | Foundry due at 20 is short; inputs added at 21 produce nothing until the next due attempt at 40 |
| Storage full | Recipe that would exceed 10000 output units consumes no input; other independent recipes still run |
| Fleet fuel | Fresh two-hull launch costs 8 energy; 7 rejects without changing hulls, credits, or orders |
| Travel | Distance 101: 11 ticks; departure 50 arrives 61 |
| Simultaneous battle | Two enemy escorts with 10 HP each exchange 2 damage at 10/20/30/40/50 and both disappear at 50 |
| Occupation | Eligibility for boundaries 1-100 transfers at 100; a third claimant at 99 resets it and prevents that transfer |
| Claim race | Two peaceful arks within one 50-tick window resolve by claim key independent of collection insertion order; loser survives |
| Treaty boundary | Agreement [0,1200) prevents otherwise eligible war through 1199; declaration may occur at diplomacy boundary 1200 |
| AI cadence | War at 100 can create order at 100 but no departure before 101; no war decision at AI-only boundary 50 |
| Hidden information | An unseen rival building change does not alter actor decisions until observation refresh |
| Captured freight | Destination changes owner in transit: exactly one delivery/capture receipt and no duplicated inventory |
| Suspended route | Capture source just before dispatch: no deduction/new shipment until current-owner adjustment; earlier cargo resolves |
| Waiting capture | Cargo waits at full destination at 50, capture at 51, space at 52: one captured unload, no relationship credit |
| Waiting war return | Friendly cargo waits at full destination; war starts before retry; next retry returns despite full storage |
| War return | Same receiver becomes enemy: cargo returns once and never bounces between hostile endpoints |
| Full receiver | Arrived cargo waits without loss/duplication and delivers after sufficient space becomes available |
| Dormancy | Final colony lost preserves civilization/history; surviving ark can reactivate it; no global terminal state |
| Replay | Uninterrupted 36000 boundaries equals save/reload at 18000 then continue, including observations and queue timing |
| Fleet/ship capacity | At 256 fleets an existing fleet moves, but split/new-fleet reservation rejects. At T with 4096 shipments and no phase-3 removal, due freight deducts/creates nothing; if one unloads in phase 3, one eligible phase-9 dispatch may use the freed slot |
| Capacity reservation | With 2047 hulls, accept one of two funded hull jobs; reject the other uncharged; accepted job completes |
| Hull delivery reservation | Accept a hull job for docked fleet F; a departure/split/merge attempt rejects without changing order/job/credits/energy; completion adds the hull to F |
| Dormant recovery | Last colony lost with credited ark elsewhere; fleet-only AI consumes credits, settles known neutral, becomes Active |
| Genesis clocks | G=0: first hub-energy/solar/extractor attempts at 10, foundry at 20, hub-ore/fallback at 100 |
| Repair clock | Damage at 11 first attempts at 21; energy block advances next attempt to 31; later damage does not postpone |
| Retreat target | Equal-time reachable colonies choose lower world ID and stable lane path, independent of insertion order |
| Claim close boundary | First arrival at 100 opens `[100,150)`; arrival 149 participates, arrival 150 does not; arbitration at 150 hashes close tick 150 |
| Creator atomicity | Invalid cascade preserves all references and resources; valid target removal retires affected pending orders with reasons |
| Legacy | All preexisting RulesV1 and WorkshopV1 bytes/digests/phase fixtures are unchanged |

## 10. Data and validation obligations

V2 catalogs define the exact recipes, construction and hull parameters, archetypes, and policy constants described above. Store their canonical hash in saves. The initial rules release ships these values as one built-in validated pack, not remotely fetched content.

### Canonical wire and identity contract

V2 canonical documents are UTF-8 JSON with no BOM or insignificant whitespace. The schema-declared field order is normative. Every field is present; an optional value is encoded as `null` or its value, never omitted. Tagged enums put the `type` field first and use the exact lowercase-snake-case discriminants declared by the schema. IDs and 32-byte seeds/digests are lowercase fixed-width hexadecimal strings. Numbers are signed or unsigned base-10 integers in their declared ranges; V2 canonical data contains no floating-point values. Collections that are logically maps are encoded as arrays sorted by stable ID; other arrays retain their explicitly meaningful order. Generic JSON maps are forbidden inside canonical authority types. Names use the validated printable-ASCII contract. Decoders reject duplicate/unknown fields, invalid Unicode/escapes, out-of-order keyed arrays, and bytes whose decode/re-encode is not identical.

Top-level catalog packs use `kind="NYON_LIVING_GALAXY_DATA"`, `format_version=2`, and `rules_version=2`. Their top-level fields, in order, are `kind`, `format_version`, `rules_version`, `pack_id`, `pack_version`, `star_archetypes`, `world_archetypes`, `resources`, `industry_definitions`, `hull_definitions`, `hazard_definitions`, and `policy_definitions`.

Archives use `kind="NYON_LIVING_GALAXY_ARCHIVE"`, `format_version=2`, and `rules_version=2`. Their top-level fields, in order, are `kind`, `format_version`, `rules_version`, `catalog_hash`, `genesis_seed`, `genesis_generator`, `genesis_manifest`, `revisions`, `branches`, `active_view`, `final_tick`, `final_digest`, `accepted_sequence_high_water`, `branch_sequence_high_water`, and `integrity_sha256`. The two high-water fields are `u64` and carry the durable allocator marks defined in section 2; they participate in `archive_integrity` like every other listed field. Integrity hashes the same ordered payload without the final integrity field. `genesis_generator` records an exact generator/starter ID and version for provenance; `genesis_manifest` is the validated source of replay truth. Generators are never rerun when decoding. Revisions contain creator inputs only. Checkpoints are excluded from the canonical archive and treated as discardable derived local caches. The archive references its catalog hash and never embeds a pack. Unknown kind, format, or rules values reject with distinct typed errors.

Use distinct wrapper types `LivingCatalogHashV2`, `LivingStateDigestV2`, `LivingRevisionIdV2`, `LivingEntityIdV2`, `LivingBranchIdV2`, `LivingReceiptDigestV2`, and `LivingEventIdV2`. No API accepts a WorkshopV1 wrapper where a Living V2 identity is required, even though serialized widths may match.

All hashes use SHA-256. Literal domains below are ASCII bytes including their terminating NUL. Fixed-width numbers use little-endian byte order. The normative formulas are:

```text
catalog_hash = SHA256("NYON-LIVING-PACK-V2\0" || canonical_catalog_bytes)
state_digest = SHA256("NYON-LIVING-STATE-V2\0" || canonical_state_bytes)
archive_integrity = SHA256("NYON-LIVING-ARCHIVE-INTEGRITY-V2\0" || canonical_payload_bytes)
receipt_digest = SHA256("NYON-LIVING-RECEIPT-V2\0" || canonical_receipt_bytes)

revision_id = SHA256("NYON-LIVING-REVISION-V2\0" || rules_u32 || catalog_hash_32 ||
                     genesis_seed_32 || parent_tag_u8 || parent_32_if_present ||
                     tick_u64 || ordinal_u64 || command_length_u64 || canonical_command_bytes)

creator_entity_digest = SHA256("NYON-LIVING-CREATOR-ENTITY-V2\0" || revision_id_32 ||
                               entity_kind_u16 || batch_local_id_u16)
autonomous_entity_digest = SHA256("NYON-LIVING-AUTO-ENTITY-V2\0" || rules_u32 ||
                                  catalog_hash_32 || genesis_seed_32 || branch_id_16 ||
                                  tick_u64 || phase_u16 || actor_id_16 || intent_ordinal_u16 ||
                                  entity_kind_u16 || local_id_u16)
entity_id = first 16 bytes of entity_digest

root_branch_digest = SHA256("NYON-LIVING-ROOT-BRANCH-V2\0" || rules_u32 ||
                            catalog_hash_32 || genesis_seed_32 || genesis_manifest_digest_32)
root_branch_id = first 16 bytes of root_branch_digest
fork_branch_digest = SHA256("NYON-LIVING-FORK-BRANCH-V2\0" || parent_branch_16 ||
                            fork_revision_32 || fork_tick_u64 || branch_ordinal_u64)
fork_branch_id = first 16 bytes of fork_branch_digest

event_digest = SHA256("NYON-LIVING-EVENT-V2\0" || receipt_digest_32 || event_ordinal_u16)
event_id = first 16 bytes of event_digest

claim_rank = SHA256("NYON-LIVING-CLAIM-V2\0" || rules_u32 || genesis_seed_32 ||
                    world_id_16 || claim_close_tick_u64 || civilization_id_16)
```

`genesis_manifest_digest_32` is the Living state digest of the validated tick-zero state materialized from that manifest. In the formulas, `entity_digest` means the creator or autonomous formula appropriate to the provenance. Shipments, AI-created fleets/hulls, and other tick-created objects use the autonomous formula; creator batches use the creator formula. `phase_u16`, `intent_ordinal_u16`, entity kinds, and local IDs come from the explicit never-reordered schema tables published below, not from Rust enum layout. The same autonomous identity inputs may be used only once; route dispatch uses the route as actor and its stable per-boundary dispatch ordinal. Optional tags are exactly `0x00` for absent and `0x01` followed by the fixed-width value for present. Batch-local IDs start at zero and are unique within the command.

`branch_ordinal_u64` is allocated from a separate document-global monotonically increasing `branch_sequence` only when a fork command validates and is accepted. Rejected or stale forks consume no ordinal. Accepted ordinals remain reserved and are never reused after queue clearing, a fault, Undo, later branch navigation, or archival. The root branch does not consume the fork sequence.

### Authoritative state schema

`LivingGalaxyStateV2`'s field order is normative and is published here. It was promoted from the
civilizations implementation plan on 2026-09-08 so that this specification, rather than a downstream
consumer plan, is its authority. Reordering a field is a format break, not a cosmetic edit.

Every field is present. Logical maps are arrays sorted by stable identifier; generic JSON maps are
forbidden. Fields 4 through 23 are sorted arrays.

| # | Field | Notes |
| ---: | --- | --- |
| 1 | `tick` | completed boundaries |
| 2 | `accepted_sequence` | historical, per section 2 |
| 3 | `branch_sequence` | historical, per section 2 |
| 4-7 | `systems`, `stars`, `worlds`, `lanes` | topology |
| 8 | `civilizations` | |
| 9-10 | `deposits`, `colonies` | |
| 11-13 | `facilities`, `construction_jobs`, `hull_jobs` | |
| 14 | `fleets` | hulls nested inside the fleet record |
| 15-16 | `routes`, `shipments` | |
| 17-19 | `relations`, `agreements`, `wars` | relations are directed |
| 20-21 | `observations`, `hazards` | |
| 22-23 | `settlement_claims`, `occupations` | |
| 24 | `counters` | reserved; see below |

Fields 2 and 3 are the historical values defined in section 2. The live allocator high-water marks are
not state fields, never appear in any `LivingGalaxyStateV2`, and are never inputs to `state_digest`.

`counters` is a `LivingCountersV2` that carries no fields in this rules version. It is reserved rather
than omitted so that adding document-global counters later does not reorder fields 1 through 23. The
cost is deliberate and is stated so it is not later mistaken for an oversight: it contributes a constant
to **every** `state_digest`, permanently, and populating or removing it is a format break requiring new
vectors. The per-relation delivery accumulators of section 5 are not this record; they live inside the
relation record where section 5 places them.

Entity kinds were assigned by the task that fixed the authority entity set, numbered freely rather than
chosen to match values that already appeared in the test corpus. The resulting registry is published
below with the phase and intent registries. Renumbering invalidated the affected frozen vectors, which
were re-derived. Re-derivation did not license a new method: the affected digests were recomputed
outside the implementation crate by the same independent path the original vectors used, because a
corpus regenerated from the implementation's own output is a recording of the code rather than evidence
about it.

### Receipt payload and derivation order

`canonical_receipt_bytes` encodes `LivingReceiptPayloadV2`, a record distinct from the public `LivingTickReceiptV2`. Its fields, in order, are `tick_u64`, `applied_revisions` (the sorted array of 32-byte revision IDs), `event_payloads`, and `state_digest_32`. Each entry of `event_payloads` is `{ordinal_u16, provenance, kind}` in emission order, using the same provenance and kind encoding `LivingEventV2` carries but omitting `id`.

The derivation order is normative and resolves the circularity in the formula above: compute `receipt_digest` over the payload first, derive each event `id` second from `receipt_digest` and its ordinal, and assemble the public `LivingTickReceiptV2` third. Pending events held inside a step context before commit carry no `id` and no placeholder value; an implementation that hashes a receipt containing its own digest, or that substitutes zeroed identities to break the cycle, is not conformant.

### Ordinal registries

These tables are normative, never reordered, and never reissued. `phase_u16`, `intent_ordinal_u16`, and `entity_kind_u16` are hash inputs to the creator and autonomous entity formulas, so an implementation that renumbers them produces different entity identities for the same history.

Phase ordinals are the ten steps of section 2 numbered as that list numbers them, from 1:

| `phase_u16` | Step |
| ---: | --- |
| 1 | Apply queued validated creator interventions |
| 2 | Expire agreements, truces and wars; update hazard boundaries |
| 3 | Resolve travel arrivals and freight delivery, return and capture |
| 4 | Resolve retreat departures, combat rounds, occupation and settlement claims |
| 5 | Complete construction; run hubs, solar, extraction, processing and repairs |
| 6 | Refresh civilization observations |
| 7 | Update diplomacy and resolve proposals |
| 8 | Generate economic and fleet intents; validate and reserve centrally |
| 9 | Dispatch scheduled fleet orders and eligible freight routes |
| 10 | Produce ordered event receipts, update counters, hash and commit |

One-based numbering is deliberate. The step list is already numbered from 1, and the reviewed vectors in `crates/nyon-workshop-core/tests/fixtures/living-v2/vectors.json` label phase 8 as the AI fleet case and phase 9 as the route shipment case, which is only correct under this numbering.

Intent ordinals cover the economic intents of section 7 and then its fleet priorities:

| `intent_ordinal_u16` | Intent |
| ---: | --- |
| 1 | Supply an owned colony below its reserve |
| 2 | Build extractor on an observed usable deposit |
| 3 | Build foundry with projected inputs |
| 4 | Build ark for a currently eligible settlement target |
| 5 | Build escort while below the defense target |
| 6 | Build solar array |
| 7 | Build scout while fewer than two exist |
| 8 | Build missing shipyard for a needed hull job |
| 9 | Build missing battery at a threatened owned colony |
| 10 | Defend an attacked owned colony |
| 11 | Retreat a noncombatant |
| 12 | Settle |
| 13 | Authorized attack |
| 14 | Explore |
| 15 | Reinforce an undefended owned colony |
| 16 | Establish or adjust a freight route |

Entity kinds are the authority records that carry an identity of their own, numbered in the order the state schema declares them:

| `entity_kind_u16` | Entity |
| ---: | --- |
| 1 | System |
| 2 | Star |
| 3 | World |
| 4 | Lane |
| 5 | Civilization |
| 6 | Deposit |
| 7 | Facility |
| 8 | Construction job |
| 9 | Hull job |
| 10 | Fleet |
| 11 | Hull |
| 12 | Route |
| 13 | Shipment |
| 14 | Hazard |

A colony, relation, agreement, war, observation, settlement claim and occupation are deliberately absent, because each is keyed by the identities it relates rather than by one of its own, so none of them can be the subject of either entity formula. The `LivingEventKindV2` discriminant ordinal is deliberately **not** assigned: tagged enums travel the wire as lowercase-snake-case strings, and `event_digest` consumes an emission ordinal rather than a kind, so no byte or hash depends on it. It may be published later without invalidating any frozen vector.

Claim arbitration chooses the lexicographically smallest full 32-byte rank after all higher-level eligibility and strength comparisons. Whole exported-file SHA-256 shown by the UI is an ordinary hash of the final canonical file and is distinct from the domain-separated payload integrity. Strings or variable byte slices included by a future domain formula must be preceded by their `u64` byte length; adding such a field requires a new explicitly documented formula/domain.

Hash-primitive goldens include `PACK domain || {}` = `fb53ddfe7525550e3eda1e6eed938928e6350c7b6bb5b66ca16f3d74350e020c`, `STATE domain || {}` = `23aea47d6aa816e31f3f5a65dfe7d6d9b6c8889cf6442826f030ab1800bfd683`, and `ARCHIVE-INTEGRITY domain || {}` = `dcf8ed382ce041b714305622a8c800cc6ae19bb1753b6f9369e62a14871e2714`. The reference implementation must add checked-in full valid minimal pack, genesis/state, command/revision, creator/autonomous entity (including a shipment), root/fork branch, receipt/event, claim, and archive vectors with exact canonical bytes and expected hashes before authority, persistence, native, and web work split into parallel modules. Every implementation consumes the same corpus.

Archive validation checks format/rules domains, canonical encoding, integrity, exact catalog identity, validated genesis, references, capacities, integer ranges, queued costs/durations, fuel credits, observation timestamps, treaty intervals, fleet paths, claim/occupation state, and history replay. Unknown versions or fields produce typed errors; they are not guessed into compatibility. Living V2 rejects V1 kinds before its own manifest/catalog mutation. The product import router never submits V2 bytes to the legacy store; WorkshopV1's existing lower-level validation and rejection behavior remains unchanged.

V2 archive input is capped at 32 MiB, a catalog at 2 MiB, nesting at 32 levels, and all entity/queue/history counts at the declared limits. Reject oversize bytes before parsing. These limits are independent of V1's unchanged limits. Bound a decode/replay poll to 1024 work units, where one unit is a visited entity, record, or candidate validation rather than an entire unbounded tick. A work slice may retain private candidate progress but never publish partial authority. Show progress and allow cancellation between polls. Replayed event pages and comparison snapshots use the same bounded job mechanism.

The first implementation plan must assign focused tests to each new invariant, including continuation within an unfinished tick and cancellation before commit. No game logic may depend on presentation preferences, audio availability, GPU results, current browser, or wall-clock timing.
