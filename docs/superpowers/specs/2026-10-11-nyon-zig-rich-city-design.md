# NYON Zig: rich city economy and playable capital

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Historical; Zig continuation canceled 2026-10-19.** Preserve this source contract and evidence. Active recovery targets Rust stable1.99.1; see the 2026-10-19 Rust designs/plans. Historical review labels do not approve new Rust behavior. Tests and CI gates remain paused.
<!-- END 2026-10-19 rewrite decision -->


Date: 2026-10-11. Author: Abbey.
Status: **Proposed — selected under delegated game-design authority; independent
technical review, controller approval of the reviewed revision and a detailed
implementation plan are required before implementation.** No new human approval
pause is implied by the delegation. This document claims no implemented API,
qualified storage fit, runtime result or finished game.
Compiler: exactly `0.18.0-dev.146+35accc06e`.
Proposed semantic identity: `nyon.rich-city.v1`.
Proposed composed rules identity: `nyon.city-combat-galaxy.v1`.
Proposed composed storage profile: `nyon.city-combat-storage.v1`.
Proposed scenario identity: `nyon.capital-start.v1`.

## Decision and alternatives

Build a complete local city loop around households, paid construction, worker
allocation, water, food, power, medicine, storage and connected transport. Every
city belongs to one civilization and one world. Cities share the coordinator's
one-hour clock and atomic publication; no city owns an independently advancing
clock, history, policy or public mutation lane.

Choose eight stored commodities and hourly integer recipes with a city-wide
warehouse ledger. Individual factories draw from that ledger through the city's
connected infrastructure; local freight transfers between cities. Per-building
inventories and vehicle simulation would add independent capacity, deadlock and
routing rules without improving the first playable production decisions enough
to justify them. Those are possible future versions, never hidden approximations
that claim to be implemented here. A two-commodity economy is insufficient for
the requested city game; unrestricted recipe scripting would leave balance,
conservation and work bounds unspecified. Use the fixed rules below.

Choose a trusted, deterministic capital scenario at new-game creation. Pure ruler
play starts with a productive capital and visible initial endowment. It does not
accept creator commands, conceal a creator tick, or alter the grant-free seeded
manifest/coordinated-v0 genesis. An empty sandbox is a separate explicit start.

The authority/storage work is a dependency, not a completed interface. Its
Proposed profile and outstanding review are cited below. This semantic extension
requires a new composed rules identity and reviewed integration plan; do not
insert these phases or fields into fullscale-galaxy.v1, coordinated-galaxy.v0,
planet-economy.v0 or seeded-galaxy.v0. Existing bytes, fixtures, old profiles and
old behavior remain unchanged. Future strategic systems require another reviewed
composition/version; this document defines their city-facing boundary only; the separately owned tactical rules are incorporated
through the explicit proposed interfaces and phase registry below.

## Selected base and remaining owner reconciliation

The controller and [independent reconciliation review](../../qualification/2026-10-11-city-contract-reconciliation-review.md)
select rich-city as the base proposal. The separately owned
[city combat/recruitment annex](2026-10-11-nyon-zig-expanded-city-economy-design.md)
now references that base. This revision incorporates the recommended city-side
integrity, capacity, training, occupation and **stalled/cancelable freight** rules.
It retains one commodity registry, power/health,512 population, road-as-building
quotas, next-hour project activation and the receipted capital scenario.

This is a selected **Proposed composition**, not integrated approval. The annex
read during fix1 still allowed damage to roads, guaranteed delivery into a captured
city and suppression of zero-worker solar on occupation; those alternatives were
explicitly not selected. Later owner revisions report adopting the selected rules,
but require fresh scoped comparison and repinning to this corrected city revision.
The tactical owner must match the
phase tags, training tier, effect/grant schema, composed identity and unique event
emitters below. No writer may combine incompatible alternatives in source.

The initial full-scale codec continues to define only `nyon.fullscale-galaxy.v1`.
This document's city codec appendix is a new proposed byte contract; it never
widens that identity. Tactical record/command payloads and storage/profile
registration still need one jointly reviewed composed codec. The durable-session
owner's latest draft admits32MiB logical transactions through1MiB segments; that
is Proposed compatibility progress, not an accepted session implementation or a
reason to restore superseded1MiB logical-node limits. Full owner amendment lists
and remaining proofs are recorded at the end of this file.

## Sources and authority ownership

- [Approved planet economy](2026-10-11-nyon-zig-planet-economy-design.md),
  “Topology, generation and footprints,” “Commands, roles and economic
  admission,” and independent vectors: reuse cube adjacency, explicit costs,
  reservations, typed provenance, staged mutation, cancellation and replay
  concepts. Its recipes, hunger/growth thresholds and old tags remain its own.
- [Approved coordinated galaxy](2026-10-11-nyon-zig-coordinated-galaxy-design.md),
  “Routing, roles and local economics” and “Global phases and prepared barrier”:
  all-world participation, shared allowance, immutable views and one publication.
- [Seeded galaxy design](2026-10-11-nyon-zig-seeded-galaxy-design.md),
  “Capacity decision,” “Generator rules,” “Navigation boundary” and “Roles and
  initialization”: 64/512/16 manifest ceiling, side4..32, stable founding worlds,
  no manifest grants, and distance distinct from travel time. Its reviewed
  revision must be pinned by the integration plan.
- [Proposed full-scale storage](2026-10-11-nyon-zig-fullscale-authority-design.md),
  “Capacity profile,” “Logical indexing and sparse storage,” “One global
  transaction,” and “Global history and endurance boundary”: shared sparse pool,
  qualified records, globally capped entities, bounded transcript and segmented
  history dependency. Its [independent review](../../qualification/2026-10-11-fullscale-authority-design-review.md)
  identifies ownership/provider/codec work; this city spec does not resolve it
  by assumption.
- [Expanded roadmap](../plans/2026-10-11-expanded-zig-rewrite.md) and
  [delegated decisions](../../qualification/2026-10-11-delegated-game-design-authority.md):
  complete Zig game, creator/ruler/combined, cities and all delivery targets;
  independent review remains mandatory.
- Current [role admission](../../../zig/src/play_role.zig) and
  [pure economy context](../../../zig/src/planet_economy/rules.zig) provide reuse
  seams. Their existence is not evidence that the new city types exist.

The civilization registry owns identity/presence and the global coordinator owns
policy. A city record owns owner civilization, population, needs, stocks,
reservations and local preferences. Buildings own recipe selection, footprint,
construction state and priority. Shipments own cargo after departure; destination
cities own only incoming reservations. Claims belong to cities, never to a UI
selection. The future civilization controller may submit bounded internal intents
through the same rule checks, but no AI behavior is introduced here. No public
command transfers city ownership, deletes a civilization or replaces policy.
Only a validated tactical occupation effect can change city owner; the city
authority applies it within the shared unpublished candidate.

## Units, limits and logical records

One published tick is one hour; completed tick0 is genesis. Horizon1,000,000.
All quantities and arithmetic are integer, checked, with widened intermediates.
Stocks, cargo, costs and incoming reservations use u32 units; population u16,
health debt u8 in0..100, needs/growth streaks u16. IDs and clocks retain distinct
strong types. Entity references are `{planet:u32,local:u64}` plus typed kind;
cells are planet-qualified. Local IDs begin1, increase only for accepted
creations, never reuse, and cannot wrap. No negative stock, float, wall clock,
locale or host scheduling enters authority.

Adopt the Proposed storage quotas without increasing them: 4,096 cities globally,
64/world; 16,384 buildings/projects globally,256/world; 4,096 local shipments
and64/world;36,864 claims globally,576/world and actual surface cell count.
Roads, stores, docks and unfinished buildings all consume the building quota.
Each city has at most256 buildings subject to stricter shared world/global caps,
population0..512, and eight commodity ledgers. Claims stay within one hall plus
at most two cells per building. A hall is the city's claim, not a second building.
Per-city stock capacity is512 for **each** commodity; each completed warehouse
adds512, maximum4,096 per commodity after seven effective warehouses. An eighth
warehouse is rejected as CapacityNoBenefit. Multiple commodities do not share a
mass bucket. Incoming reservations count against the matching commodity cap. The absolute
hard bound is stock+incoming<=4,096 per commodity. Involuntary loss of functional
capacity preserves existing goods and reservations and reports over_capacity;
normal new deposits and discretionary refunds still require functional capacity.
Reserved arrivals change stock/incoming by opposite quantities, preserving the sum.

Commodity tags: food0, water1, timber2, stone3, ore4, alloy5, tools6, medicine7.
Power is an ephemeral hourly service, not cargo or a stored commodity. Housing
and workers are capacities, never transferable items. Seeded terrain remains
rocky/fertile under the approved formula; resource extraction below has explicit
source receipts and no finite-deposit claim. Ecology/depletion needs a later rule.

A city record must fit one256-byte payload: identities/counters, eight pairs of
u32 stock/reserved, population/debt/streaks, growth toggle, construction toggle,
hall integrity, occupation tick and supply/care summaries derived from receipts. Building records fit one256-
byte payload including two cell IDs, kind, facing, enabled, priority, remaining
crew-hours, integrity and activation tick. Shipment header fits256 bytes plus its existing
three route pages. No per-building stock, worker entity, unbounded demand list or
new persistent index family is authorized. Transient graph/work allocation
scratch must be accounted within the Proposed8MiB shared scratch. Compile-time
layout, codec maxima, path cursors and max-dirty-root proofs remain integration
prerequisites; these sentences are proposed envelopes, not measured fit.

## Cube placement and infrastructure

Retain the six face bases and independently verified adjacency from the economy
spec. A farm occupies its anchor plus one resolved neighbor in the anchor's
local facing; its anchor must be fertile. Quarry anchor must be rocky. Every
other building occupies one cell and accepts either terrain. Preserve farm
anchor/facing and both resolved cells. No longer rectangle, implicit transported
compass or diagonals are admitted. A route is a list of qualified cell IDs, not a
sequence of unchanged compass headings across faces.

A city hall occupies one unclaimed cell. Roads must attach by a cardinal edge to
the same city's hall or existing road, including reserved road projects for
placement only. Every other building must have some footprint cell adjacent to
that city's hall or road (reserved road projects also permit placement). All
footprint cells are distinct, unclaimed, on the named world and acquired atomically.
A completed building operates only from its activation tick, while integrity>0
and connected to its own hall through completed roads; a farm's two cells are internal access to the same building,
not public roadway. Adjacent cities do not merge owners or internal stock.

At each hour's infrastructure phase, traverse all occupied hall/road components
once per world, following same-city ownership with sorted-neighbor cell IDs.
This yields each city's hall-connected component in one metered traversal; it is
not a full6,144-cell scan repeated for every city. Clear scratch once per world
and inspect each claim/edge at most a fixed number of times. Long lookups remain
charged. No unaccounted persistent reachability cache is introduced.
A building attaches if any footprint cell adjoins that reachable network. The
same connection supplies warehouse access, water and power distribution; this
version has no hidden pipe or cable overlay. Roads carry pedestrian/workforce
access without throughput limits inside a city. Infrastructure capacity decisions
are city stock, power and workforce; traffic congestion is not simulated.

Freight routes may use only completed hall/road cells of the endpoint civilization;
intermediate halls/roads may belong to another city of that same civilization.
Every consecutive pair must be cardinal neighbors, no repeated cells,2..65 cells,
first source hall and last destination hall. Endpoints differ and share a world.
This makes road corridors a meaningful construction expense. An arbitrary path
over undeveloped terrain is not valid freight in this new version. No automatic
pathfinding decision is authoritative: a host may propose a path, the authority
validates every edge. Seam direction reversal follows adjacency, not “opposite
heading.” Growth, routes and projects cannot depend on camera face selection.

## Buildings, recipes and construction

Costs below list nonzero stored commodities. All ruler construction costs are
consumed once at admission. `work` means remaining crew-hours; one successful
construction hour assigns exactly two workers and subtracts2. Completion is
recorded that hour, but the building becomes operational next hour. Construction
requires completed connection as above (a road project requires adjacency to the
completed network), creating explicit staged road-building dependencies.

| Tag / building | Cost | Work | Operating workers | Per-hour operation |
| --- | --- | ---: | ---: | --- |
| 0 Road | stone2 | 2 | 0 | Completed connection cell |
| 1 Waterworks | timber8,stone8,tools2 | 4 | 2 | Source40 water; no power/input |
| 2 Farm | timber12,stone4,tools2 | 6 | 4 | water8,power4 → food32 |
| 3 Forestry | timber6,stone4,tools1 | 4 | 2 | Source6 timber; no power |
| 4 Quarry | timber8,stone8,tools2 | 4 | 2 | Source6 stone +2 ore; power2 |
| 5 Solar array | stone8,alloy4,tools2 | 4 | 0 |24 hourly power |
| 6 Residence | timber12,stone8,tools1 | 4 | 0 | +16 housing |
| 7 Warehouse | timber12,stone12,tools2 | 4 | 0 | +512 capacity/commodity, max4,096 |
| 8 Foundry | stone16,alloy4,tools4 | 6 | 2 | ore4,power4 → alloy2 |
| 9 Workshop | timber12,stone12,alloy4,tools4 | 6 | 2 | timber2,alloy2,power4 → tools2 |
| 10 Apothecary | timber8,stone8,alloy2,tools2 | 4 | 2 | food4,water2,power2 → medicine2 |
| 11 Clinic | timber12,stone12,alloy4,tools2 | 4 | 2 | medicine1,water2,power2 → care |
| 12 Spaceport | stone32,alloy24,tools8 | 12 | 2 | Reserved external transfer service; power4 |

Solar is a rated array with integrated buffering:24 units every connected enabled
hour. No day/night, weather, battery commodity or orbital shadow is implied.
Forestry/quarry are renewable rule sources in this version; their receipts name
source kind and exact extraction. Source terrain and later ecology may change
only with a reviewed rules version. A disabled solar array produces zero.

An intact hall gives16 housing; completed connected intact residences add16 to a
maximum512. A ruined hall gives zero hall housing and blocks worker operations;
its cell still anchors the road component and its base512 ledger capacity remains.
Accept another residence only if its completion can add housing under that cap,
counting admitted residence projects. Warehouse admission similarly counts
projects against its seven-increment cap. Disabled residences/warehouses retain
capacity; only productive/service buildings have an enable toggle. Roads/housing/
warehouses can be dismantled subject to the capacity/connectivity checks below.

A city may enable/disable construction and each productive/service building,
and assign each project/building priority0..3 (0 first). Idle projects preserve
remaining work without recurring cost. Cancellation of an unfinished project
returns floor(original_cost * remaining_work / initial_work) independently per
commodity; work already performed is sunk. Preflight refund capacity before any
claim release. A rejected cancellation has no effect. Completed dismantling gives
floor(original_cost/2) per commodity, requires room for all refunds, and cannot
reduce housing below population, capacity below stock+incoming, disconnect any
completed building from the completed network or project from a path through
completed/reserved roads, or remove a road/
hall used by active freight. Houses/stores created freely retain their listed nominal costs for explanation,
but those costs were waived and are not refundable. Hall
removal/city deletion is not admitted. A zero-population city remains inspectable
and recoverable through the explicit resettlement expedition below or creator
population; food cargo alone cannot create residents.

To avoid refund creation through free authoring, authored buildings additionally
store a paid-cost flag: creator-authored buildings have **zero** refundable cost
unless the creator explicitly chose paid construction through an ordinary ruler
envelope. Thus nominal waived costs are explanatory receipts, not salvage stock.

## Hourly allocation and basic needs

All worker counts use population and health debt from the hour's start, after
commands/arrivals but before needs. Available workers are
`floor(floor(population/2) * (100-health_debt) / 100)`; nobody belongs to two jobs
in one hour. Unassigned workers remain residents. Workers are not consumed.
Population changes in the needs phase affect next hour only.

At city infrastructure entry, connected enabled solar arrays produce their rated
power. Reserve domestic demand `ceil(population/8)` first, supplying min(demand,
available). A shortfall is visible and blocks growth. Remaining power supplies
operations; unused power expires with an explicit hourly total. No energy can be
shipped or carried into the next hour. Complete housing/capacity is recalculated
from connected structures before supply allocation.

Process cities ascending local ID, with this fixed operating tier order:
waterworks; farms; clinics; construction; training; industry (forestry, quarry,
foundry, workshop, apothecary); spaceports. Within each tier sort by `(priority,BuildingId)`.
This is a gameplay rule; priorities do not move an industry ahead of food/water.
Players disable essentials deliberately to reassign their workers. Construction
is a separate tier using the same remaining workforce. No same-hour completion
is inserted into an earlier tier.

Each operation preflights in this order: enabled, intact/activated, connection,
occupation/ruined hall, workers, stored inputs, power, all positive output capacity
including incoming reservations. Zero-worker solar is computed at infrastructure
and remains available during occupation if intact/connected. Worker operations
assign zero workers in an occupation hour or while the hall is ruined. A stopped
operation records precisely the first idle reason and takes no input, power or
workers, allowing later operations to use them. Otherwise consume inputs/power,
assign workers and add complete outputs atomically. No fractional recipe, partial
quarry output or speculative borrowing of future output. Prior successful outputs
are visible to subsequent operations in the specified order. Same-hour medicine
from an apothecary cannot retroactively run a clinic. Player priorities within the
industry tier can run quarry→foundry→workshop in one hour when stocks, people,
power and capacity permit; that order must be deliberately chosen or follows IDs.

A connected enabled clinic operates even at zero health debt, consuming1medicine,
2water,2power and2workers. Each operational clinic supplies care for up to32
residents; cared residents=`min(population,32*operating_clinics)`. Health treatment
is8 debt reduction if care covers all residents, otherwise
`floor(8*cared_residents/population)` (zero for empty city). Clinics are not
required for workforce when debt is zero; they are required for growth.

Needs follow all operations. Let P be current population at needs entry. Consume
`min(food,P)` food and `min(water,P)` water, independently. A city with P=0
consumes nothing and resets growth/shortage streaks; it gets no growth/care bonus.
Partial supply consumes the available stock and counts as shortage. No input
reservation protects households from earlier recipes: the preview explicitly
shows that industry can consume their food/water, and the player may disable it.

For P>0, shortage streaks increment on short supply and reset on full supply.
Health debt becomes clamp0..100 of previous debt +6 if food short +10 if water
short +2 if domestic power short +2 if population exceeds housing, minus1 when
both food/water are full and housing/domestic power suffice, minus clinic
reduction. Additions/subtractions are one signed widened expression before clamp.
Mortality removes at most1 resident/hour: water streak>=3 wins first, then food
streak>=6, then resulting debt=100. On mortality reset both shortage streaks and
growth streak; receipt includes winning cause and all simultaneous deficiencies.
Consumed needs use pre-mortality P. No assets or claims disappear with population.

Growth is enabled by default. Occupation or a ruined hall resets growth streak.
At P>0 with no death, no occupation/ruined hall, full food/water, sufficient
power/housing, resulting debt<=20 and full clinic coverage, increment growth
streak. Otherwise reset it. On streak24, add1 resident only if housing>P and
P<512; otherwise emit GrowthHousingBlocked; either way reset streak. No food is
minted by growth; the new resident consumes next hour. Turning growth off resets
streak and permits a stable manufacturing city. This is an abstract population
unit, not a claim that human gestation takes24hours.

## Local trade, founding and external boundary

`ship_local` moves1..64 units of one commodity between same-owner cities.
Preflight stock, current destination functional capacity and full route, then
atomically debit source, create shipment cargo and increment destination incoming.
Record launch owner, departure tick, path, position0 and stalled_ticks0. Admission
checks departure+edge_count<=horizon, but that nominal ETA is not a guarantee.
There is no movement on the departure tick. Each subsequent local-freight phase
visits shipment IDs ascending and advances exactly one edge if both endpoint
owners still equal launch owner, both halls are intact, and next path cell is a
completed hall/road of launch owner. Otherwise keep position/cargo/reservation,
increment stalled_ticks by1 and expose the first reason in that order. Checked
stalled count fitsu32 because the entire clock horizon is1,000,000.

Reaching the last cell converts reservation into stock, decrements incoming and
retires cargo exactly once. Functional capacity may have fallen: an already
reserved arrival is honored because stock+incoming stays constant and<=4,096.
An interrupted journey resumes when conditions hold; no timeout, automatic loss,
retarget or teleport. ETA is shown only when unstalled and is explicitly a
projection. Deliberate dismantling of any cell on an active route is still blocked.
Roads are not combat targets in this composition; capture of intermediate cities
can nevertheless change next-edge ownership and stall a shipment.

`cancel_shipment` belongs to the **current source-city owner**, including after
capture. It returns all cargo to source only if source stock+incoming+return fits
current functional capacity and the4,096 hard cap. Atomically release destination
reservation, retire shipment and return cargo; a rejected return preserves every
field. The destination's current owner cannot cancel someone else's source cargo.
Successful source capture may therefore let its new owner recover outbound cargo;
destination capture alone cannot deliver or seize in-transit cargo. Reclaiming the
endpoint for launch owner can permit resumed delivery. Consumed ShipmentId is
never reused. Creator minted cargo uses explicit source=mint and otherwise obeys
the same route, reservation, stall and cancellation rules.

Local founding is a paid frontier expedition: source city must retain at least2
residents after transferring4. Cost timber16,stone16,tools4 is consumed; food32
andwater32 move into the new city's ledger. Target is an unclaimed hall cell on
the same world; no terrain/path reachability is required for this explicit
frontier abstraction, and no freight-route claim is made for it. New city has4
residents,16housing and512/commodity capacity, debt0 and no buildings. It consumes
needs in its founding hour. Founder and new city share owner. Receipt separates
consumed construction from transferred population/stock. The host must preview
its eight-hour water/food runway and cost of building a road connection. Growth
cannot found cities. `resettle_city` is a separate paid recovery expedition to
an existing same-world/same-owner city with population0: transfer4 residents,
32food and32water from a source that retains at least2 residents, consume
4timber and1tool, require destination housing>=4 and capacity for both deliveries,
and reset destination debt/streaks to0. No other inventory or claims change.
This shares the explicit instantaneous frontier abstraction; it is not freight
or orbital travel. Rejected expeditions consume nothing. No interplanetary
colonization is authorized here.

City mechanics export only the following **Proposed** typed boundary to a future
reviewed strategic transaction; no public host may call it as a grant API:

- `CityExportOffer {city, dock, commodity, amount:u32, offer_id:u64}`: query/intent,
  no mutation; amount1..64 per transfer. Export preparation requires a connected
  enabled Spaceport and this hour's successful2workers/4power service. A transfer
  consumes one of that dock's at most4 transfer slots/hour.
- `ReserveCityImport {transfer_id:u64,city,dock,commodity,amount:u32}`: coordinator-
  internal command that reserves stock capacity; owner/relationship/arrival
  authority comes from the future strategic contract, never from the city UI.
- `CommitCityExport {transfer_id,expected_city_revision,offer}`: atomically debits
  stock and assigns **sole cargo ownership** to a strategic shipment in the same
  unpublished global candidate. No independent city commit or receipt injection.
- `CommitCityImport {transfer_id,strategic_shipment,city}`: verified arrival event
  converts the already reserved amount into stock and retires strategic custody
  in the same candidate. No power/crew requirement at arrival after reservation;
  disablement cannot strand already promised goods. A port with active incoming
  reservations cannot be dismantled.
- `ReleaseCityImport {transfer_id,cause}` removes only the reservation after an
  authenticated strategic terminal outcome; it never creates/forgets cargo.

Outstanding import reservations require canonical per-transfer records and
strategic quota ownership. They are deliberately not an admitted city-v1 command:
adding them requires the separate strategic schema, phase insertion, role matrix,
record partition and transcript proof. City-v1 Spaceport status is
`awaiting_strategic_rules` and consumes no workers/power until that reviewed
composition exists. Its construction is permitted and honestly previewed as
preparation, never represented as working interplanetary trade. The complete
city gameplay journey below does not depend on a nonfunctional port. Travel
speeds, diplomacy permissions, fleets, hulls, research and foreign trade
are Out of scope here and remain mandatory future game work.

## Structural damage, occupation and paid training

These are city-owned integration rules proposed for joint ratification with the
annex/tactical owners. The tactical document alone owns attacks, hostility,
movement, rounds, hit-point formulas and capture eligibility; this section does
not invent replacements for them.

Hall and completed nonroad buildings have integrity0..1000, initially1000.
Integrity1..1000 retains full functionality; zero disables the structure's service.
Roads remain building/quota records but are **not tactical targets**, have fixed
integrity1000 after completion and never provide cover. Unfinished projects are
not targets. Constructed completion atT sets activation_tick=T+1; authored
completed buildings admitted atT have activation_tick=T and can appear in that
hour's projection. Genesis buildings activate at0. Activation tick may equal
horizon+1 for a project completed in the last legal hour; no additional tick is
thereby admitted. Completed buildings with activation_tick>target are not targets.

An eligible target need not be enabled or currently road-connected: projection
contains every activated completed nonroad structure, including ruined ones.
All footprint cells are passable. Cover is integrity>0 except farms and solar;
hall cover is integrity>0. Only an intact hall is capturable. Roads' absence from
the projection is equivalent to passable=true,cover=false,capturable=false and
not attackable, not an obstacle. Tactical overlay does not own city claims.

`repair_structure` costs2timber+2stone and restores min(250,1000-integrity) on an
owned activated completed target with integrity<1000. No road/project target.
`author_repair_structure` restores1000 with explicit bypass and receipts waived
cost `ceil(restored/250)*(2timber+2stone)`. Repair occurs in commands before combat;
it does not promise survival in that hour. No automatic regeneration occurs.
New deposits/refunds query functional capacity against prior accepted command
changes, including repairs, without using a stale previous-hour capacity cache.

City validates the complete tactical effect batch before any write: source
revision=current committed revision; target tick=the unpublished target; tactical
identity matches composed genesis; at most4,096 distinct damage targets and2,048
occupation attempts; ordinals1..N contiguous; all references appeared in the frozen
projection; damage amount1..1000 and one aggregated effect/target; at most one
occupation/city, matching expected owner, distinct present new owner and the
associated ObjectiveId. Effect order is damage `(planet,kind,id)` then occupation
`(planet,hall_id,ObjectiveId)`. Malformed input aborts the whole candidate.
Tactical emits raw-round evidence and the derived effect-input records; city is
sole emitter of each applied/blocked effect outcome.

Apply damage first with integrity=max(0,integrity-amount). Involuntary warehouse/
residence destruction reduces functional capacity/housing without deleting goods,
reservations or people. Absolute stock+incoming<=4,096 remains invariant;
over_capacity is derived when the sum exceeds functional capacity. Production
checks each **positive** output's resulting sum after recipe inputs against
functional capacity; an operation with no positive output for an over-capacity
commodity may consume it. Current recipes never consume/produce the same commodity,
but this explicit test prevents a hidden blanket rejection of useful consumption.
Previously reserved arrivals honor their constant-sum promise. New cargo/mints,
production, refunds and cancellation returns cannot force a deposit beyond
functional capacity. Voluntary dismantling retains its existing stricter guards.

For an occupation attempt, tactical first validates end-round control. An attempt
that loses end-round control has a tactical-only lost_control result and is not
sent as a city effect. An effect received by city can be blocked because this
batch destroyed its hall; emit blocked_destroyed_hall and retain owner. Otherwise
change city owner and all attached buildings/roads/stocks/training to new owner,
retaining IDs/people/resources. Set occupation_tick=target. No unit or completed
reserve changes owner. Tactical consumes the outcome against its matching
ObjectiveId exactly once: successful capture completes it; destroyed-hall block
resets its attempt. This matching is part of the shared candidate, not a callback
that can arrive after publication.

Occupation causes zero worker assignments and resets growth streak for that hour.
Households still consume goods; health responds to actual shortages/power/housing.
Intact connected zero-worker solar continues to supply power. Training/construction/
production/clinic care stop; the hall's ordinary storage/housing remains if intact.
AtT+1 no occupation suppression applies. A ruined hall independently disables all
worker operations until repaired, but retained roads and residences/solar may
remain connected and supply their passive functions. A ruined hall retains its
base512-per-commodity storage while providing zero hall housing. Capture never
rewrites a shipment launch owner; freight uses the explicit stall/cancel rules.

Training quotas are2,048 jobs globally,32/world,4/city. Each one-page city training
record owns its tactical roster reservation: no duplicate reservation page.
Global UnitId is realm-zero u64, distinct from qualified `{planet,TrainingId}`.
Live+reserve units+training reservations<=4,096 global and256/world. Reserve both
IDs/quotas and every effect before spending; rejection consumes no resources/IDs.

| Class tag | Paid goods | Crew moved from city | Successful staffed hours |
| --- | --- | ---: | ---: |
| infantry0 | food8,tools2 | 1 | 2 |
| armor1 | food8,alloy8,tools4 | 2 | 4 |
| artillery2 | food8,alloy6,tools6 | 2 | 3 |

`recruit` requires owned intact activated hall, known class, resources and at
least one resident remaining. Remove crew before the hour's workforce snapshot;
trainees supply no city workers and have no recurring household demand (the
paid food is their abstract training ration). Earliest eligibility is target+
duration; require it<=horizon before admission. No creator provenance may use
recruit; creator-authored units use the tactical explicit mint path.

After construction, before industry, visit TrainingId ascending per city. Each
job requires2 unassigned workers, intact hall and no occupation that hour;
subtract one remaining hour and hold those workers to hour end. Otherwise emit
one idle reason hall→occupation→workers→horizon without taking workers. Newly
admitted training may work immediately. At horizon, a job that would complete
idles_horizon because eligibilityT+1 would exceed it; it remains cancelable.

When remaining reaches0, one typed grant converts the reserved UnitId/slot into
one full-HP tactical reserve with crew1/2/2, current city owner and eligibilityT+1.
City removes training and tactical creates reserve in the same candidate; no
cell/occupancy is allocated. The grant producer is the qualified TrainingId,
never merely its local number. Missing/duplicate/mismatched reservations are an
invariant abort, not a normal capacity failure or lost crew. Tactical validates
class/crew/planet/hall/owner and consumes each producer once. Training completion
follows all rounds, so that unit cannot fight/deploy until a later command tick.

`cancel_training` returns all crew and floor(each original paid good/2), regardless
of progress; this is distinct from remaining-work building refunds. Current city
owner may cancel captured jobs. Require population+crew<=512 and every refund
fits current functional capacity/hard bound; housing may be exceeded with normal
needs consequences. Rejected cancellation preserves job/reservation/crew/costs.
Success retires job and releases roster quota but never reuses reserved UnitId.
Pending jobs transfer with occupation and pause that hour; existing reserve units
remain tactical owner's original civilization. No transfer is implicit on deploy.

## Trusted capital genesis

New-game setup chooses `empty.v1` or `nyon.capital-start.v1`, binds exact seeded
manifest bytes/digest, composed rules identity, quotas and trusted initial policy.
The seeded manifest remains immutable and grant-free; these are new mutable-state
scenario records in the composed authority's own genesis. Roles are validated
against present civilizations before creation. Ruler/combined require a selected
present civilization; creator accepts null/present. No role change is smuggled
inside a city envelope.

Capital start places one capital for **every** present civilization on its
manifest founding planet, even when the player selects only one. This avoids
making initial assets depend on player selection. Other civilizations' cities
receive the same economy simulation without invented AI decisions. Creator can
choose either scenario; ruler defaults to capital start. Empty ruler is allowed
only as explicitly labeled empty scenario, with no claim of a playable capital.

Capital template: population24, debt/streaks0; hall16housing, one completed
Residence(+16), Waterworks, Farm, Forestry, Quarry, Solar array. Canonical
enabled/priority bytes for all fourteen buildings are fixed below; growth and
construction are enabled. Stock vector `[food192,water256,timber96,stone96,ore24,alloy24,
 tools16,medicine8]`; all incoming zero; hall/building integrity1000, activation tick0, occupation
absent and training/unit/objective registries empty. Template buildings have paid-cost=false
and no salvage. No clinic: deliberate growth requires the ruler to construct and
staff care plus replenish medicine. Capital starts with12 workers, stable food/
water and2 spare workers after life support uses6 and forestry/quarry use4.
Construction can take those remaining2 without interrupting existing production.

Every template building is completed with remaining_work=0,initial_work equal
to its kind's table value,paid_cost=false,integrity=1000,activation_tick=0. The
following table explicitly fixes fields even when they do not affect passive
behavior. True encodes1. No implicit Zig/host initialization supplies any value.

| BuildingId | Kind | enabled | priority |
| --- | --- | ---: | ---: |
| 1 | Road | true | 1 |
| 2 | Road | true | 1 |
| 3 | Road | true | 1 |
| 4 | Road | true | 1 |
| 5 | Road | true | 1 |
| 6 | Road | true | 1 |
| 7 | Road | true | 1 |
| 8 | Road | true | 1 |
| 9 | Waterworks | true | 0 |
| 10 | Farm | true | 0 |
| 11 | Forestry | true | 1 |
| 12 | Quarry | true | 1 |
| 13 | Solar array | true | 1 |
| 14 | Residence | true | 1 |

Decoder/recomputed-genesis comparison rejects any alternate enabled/priority
byte, including solar/road/residence changes that appear behaviorally irrelevant.
This table is a scenario rule; author_building's separate true/priority1 defaults
do not override the two priority0 scenario buildings.

City creation/default mapping is also explicit. Capital has the preceding owner,
hall,population24,stock vector,health_debt0,all three streaks0,growth_enabled=true,
construction_enabled=true,hall_integrity1000,occupation_tick absent,incoming all0.
`empty.v1` contains no city/building/claim records, so it fabricates no city fields;
its global/world/theater defaults and present initial policy remain the codec's
stated zero-state. `author_city` takes owner,hall,population,stocks exactly from
its admitted input and gives all other city fields the capital defaults just
listed, including zero incoming and absent occupation. `found_city` takes owner
from source, hall from target,population4,food32,water32 and all other stocks0;
its other city fields use those same defaults. Neither creation inherits the
source's debt,streaks,policy toggles or occupation.

`resettle_city` is a mutation of an existing zero-population city: set population4,
health_debt0 and all three streaks0; add32 food/32water to existing stock, and
preserve owner,hall,all other stocks,incoming,hall_integrity,occupation_tick,
growth_enabled and construction_enabled. It does not repair the hall or reset
history/ownership. Source population/goods effects remain the paid expedition
rule. Ordinary paid `build` sets enabled=true,priority=command input,initial_work=
remaining_work=kind work,paid_cost=true,integrity0,activation_tick0. `author_building`
sets enabled=true,priority1,remaining0,initial_work=kind work,paid_cost=false,
integrity1000,activation_tick=target. All use their admitted city/footprint fields.
These are explicit existing-rule defaults, not new commands or changed recipes.

Placement is deterministic and must succeed for every admitted seed/side4..32.
For each founding world try fertile hall cells H in ascending numeric ID whose
north neighbor Q is rocky. Reserve Q for quarry. Starting with queue[H], breadth-
first expand completed backbone roads: dequeue one cell, examine its neighbors
in ascending numeric cell ID, and append each not-yet-enqueued cell except Q.
Stop immediately after eight roads have been added; H is not a road. Let network
be H plus those eight roads. Farm candidates are fertile unclaimed anchors
adjacent to network, with a second cell adjacent in the anchor's facing and not
in network/Q. Choose lexicographically smallest (anchor,facing) using algorithm tie order
north,east,south,west. Direction wire tags are independently frozen below. Reserve both farm cells. Attach Waterworks, Forestry,
Solar and Residence in ascending building-kind order to the smallest still-free
cell adjoining network. Quarry remains at Q. If any stage lacks a candidate,
discard the temporary template and try next H; no gameplay ID is consumed by
failed trials. Terrain/footprint rules and overlap checks remain mandatory.

A disposable Python standard-library basis/adjacency model checked all116
combinations of side4..32 and the four terrain-seed residue classes on2026-10-11:
all found a template with eight roads. This is a design feasibility check, not a
Zig authority test or independent runtime qualification. Preserve an independent
checked-in Zig fixture before implementation. At side4/terrain seed residue0:
hall29, quarry40; roads in BFS order[25,28,30,21,24,26,44,95]; farm17/east has
cells[17,18]; Waterworks20,Forestry22,Solar27,Residence31. Startup consumes16
claims: one hall,eight roads and seven cells in six nonroad buildings. No hidden
fallback changes starting amounts, road count or terrain.

Canonical IDs: capital CityId1 on its world; roads have BuildingId1..8 in BFS
order; Waterworks9,Farm10,Forestry11,Quarry12,Solar13,Residence14. The nonroad
assignment follows ascending kind tags, separately from placement order.
Claims encode resolved cells. Genesis stores exact IDs, placement and stocks;
replay recomputes and compares the entire deterministic template. No command
sequence is consumed at genesis. `ScenarioEndowment` receipts name scenario,
civilization/city, population, each commodity quantity, every authored building,
waived nominal cost/work and paid-cost=false. Road endowments are included.
Trusted origin is `scenario` in genesis receipts, distinct from creator/ruler/
autonomous command provenance. It is impossible to submit through public commands.

Capital creation uses the shared-budget builder, stays invisible until every
capital/claim/receipt validates, and atomically installs once. Failure/cancellation
preserves the resident session. This is a new supported start, not migration of
old empty v0 genesis or a creator grant accepted in ruler mode.

## Commands, receipts and phase order

New command tags are scoped to rich-city.v1, never appended to old enums:

| Tag | Payload | Admission |
| --- | --- | --- |
| 0 | build(city,kind,anchor,facing?,priority) | Ruler, owned city; paid project |
| 1 | set_enabled(building,bool) | Ruler; productive/service only |
| 2 | set_priority(building,u8) | Ruler; priority0..3 |
| 3 | set_city_policy(city,growth:bool,construction:bool) | Ruler |
| 4 | cancel_project(building) | Ruler; explicit proportional refund |
| 5 | dismantle(building) | Ruler; capacity/connectivity/route checks |
| 6 | ship_local(source,destination,commodity,amount,path) | Ruler |
| 7 | found_city(source,target_cell) | Ruler |
| 8 | author_city(owner,cell,population,stocks[8],bypass) | Creator only |
| 9 | author_building(city,kind,anchor,facing?,bypass) | Creator only; completed, zero paid cost |
| 10 | grant(city,stocks[8],population,bypass) | Creator only; bounded positive additions |
| 11 | author_shipment(source,destination,commodity,amount,path,source_kind,bypass) | Creator only; transfer or mint explicitly |
| 12 | resettle_city(source,destination) | Ruler; paid zero-population recovery expedition |
| 13 | recruit(city,class) | Ruler; paired city job/unit reservation |
| 14 | cancel_training(training) | Ruler; full crew and half-goods refund |
| 15 | repair_structure(target_kind,qualified_target) | Ruler; timber2,stone2 |
| 16 | author_repair_structure(target_kind,qualified_target,bypass) | Creator; explicit waived cost |
| 17 | cancel_shipment(shipment) | Ruler; current source owner, strict refund capacity |

Envelope fields are sequence:u64, provenance:u8 using existing role tags,
actor optional CivilizationId, then family:u8=0 for city, tag:u16 and payload. Optional encoding is
presence:u8 then value; booleans0/1; kind/commodity/priority/facing u8; direction wire tags east0,west1,north2,south3; population
u16; quantities u32; cell references planet:u32/local:u64; entity references same
width but typed; path count:u8 followed by cells. Numeric fields little-endian,
no padding/trailing bytes. Local outer route planet must equal all payload cells/
entities. Path max65 keeps body below1,024bytes; creator stock vector is32bytes.
A frame that cannot fit is structural failure, never truncated.

Family1 is tactical, with its own disjoint tag registry owned by the tactical
codec; no tag is inferred from payload shape. Unknown family/tag is structural.
Creator commands require creator provenance, null actor and bypass=true; they
retain all topology, uniqueness, limits, capacity and ownership-consistency checks.
Creator may target any present civilization. Author shipment endpoints must share
owner. Grant permits population above housing up to512, creating visible needs
consequences. Author city stocks must fit512/commodity and population0..512.
Author building is enabled with priority1; ordinary topology and nominal
capacity-no-benefit checks still apply. Combined ruler envelopes pay normal costs;
combined creator envelopes have explicit bypass receipts. Ruler commands in
creator-only policy are rejected. Autonomous/scenario provenance is internal-only.

Rejection precedence: role admission; command/provenance; explicit bypass;
target/present owner; ownership; ranges; topology/connection; quotas/ID counters;
stock/capacity/resources. All preflight passes finish before committing an
individual command effect into the candidate. A gameplay rejection changes no
resources, claims, IDs or reservations, but consumes its sequence if the tick
publishes. Duplicate/old sequence, malformed wire, unknown planet or >64 local
commands/world is whole-tick structural failure before gameplay. Cap1,024 commands
globally. Execution is ascending planet then per-planet sequence, preserving the
Proposed full-scale ordering for shared quotas. Queries never see partial effects.

Composed phase tags/order are fixed in the codec appendix below. They replace
this Proposed document's earlier0..9 city-only registry; no implemented or approved
old identity changes. The city/annex/tactical owners must adopt the same registry
before dispatch. City owns exactly one effect outcome per applied/blocked effect;
tactical owns raw rounds and derived effect inputs, never a second city outcome.
The next sections freeze the city-side structures and byte/event bounds.

## Proposed city codec and composed phase registry

This appendix freezes the **city-side proposal**, including its new initial state.
It is reviewable input for the single composed-codec owner. It does not complete
the separately owned unit/order/objective/theater/hostility codecs or register
the composed identity with storage/session. No implementation dispatch is allowed
until those owner contracts and independent vectors are jointly frozen.

Notation: `S=u16 byte_length + ASCII bytes`; `O(X)=u8 presence0/1 + X when1`;
`V=8*u32` in food,water,timber,stone,ore,alloy,tools,medicine order. All integers
little-endian; bool0/1; no padding/trailing bytes. `Q=planet:u32,local:u64` is a
typed qualified reference; `Structure=planet:u32,kind:u8,id:u64`, kind hall0 or
building1. Direction wire is east0,west1,north2,south3. Capital tie order remains
north,east,south,west (wire2,0,3,1); choosing a direction wire does not change the
already stated template witness. Unknown tags, noncanonical optionals/booleans,
truncation and overlong counts are whole-batch structural failures. Valid known
enum/range values that fail gameplay limits get the ordered rejection below.

### Shared ordered phases

| Phase tag | Work and barrier |
| --- | --- |
| 0 | Structural admission, root/history reservations and globally sorted input |
| 1 | Public commands: planet ascending, then global sequence |
| 2 | Copy all city and separately owned hostility projections |
| 3..12 | Four tactical rounds: start, intents, move proposals, arbitration, position install, shots, damage accumulation, damage application/deaths, withdrawal, objectives |
| 13 | Validate all city effects; apply damage then occupation and pair objective outcomes |
| 14 | Local freight movement/stalls/arrival |
| 15 | Infrastructure, power and city worker snapshots |
| 16 | City operations: waterworks,farms,clinics,construction,training,industry,spaceports |
| 17 | Household needs,health,mortality,growth |
| 18 | Global/local invariants,canonical state/receipt hash, prepared commit bytes |
| 19 | Prepared barrier |
| 20 | One descriptor publication |
| 21 | Retirement |

For each round0..3 execute all worlds at phase3, then all at4, through12 before
starting the next round. Other phases also visit every world before advancing.
No phase is conditioned on world visibility. Round:u8 occurs in tactical event
payloads; there is no independent round clock/revision. Canonical event ordinals
are consecutiveu32 across the whole tick (not reset per phase/world/round).
Operation tier is encoded in building/training status, not a second global phase.
Publication consumes one unit and emits/hashes no new bytes; commit framing was
already prepared in18. Initial genesis events use tick0/phase0; they are outside
the first command tick. No extra strategic phase is admitted in this registry;
the hostility producer remains a required separately reviewed composition input.

### New genesis and canonical state

Genesis object grammar:
`NYRC || wire_version:u16=1 || S(nyon.city-combat-galaxy.v1) ||
S(nyon.city-combat-storage.v1) || S(nyon.rich-city.v1) ||
S(nyon.tactical-ground.v1) || scenario:u8 || mechanics_version:u16=1 ||
annex_version:u16=1 || manifest_length:u32 || manifest_wire ||
manifest_digest[32] || initial_policy:Policy || quota_count:u16=52 ||
quota_values[52]:u32 || initial_record_count:u32 || canonical_records`.
Scenario empty0 or capital1 means exactly empty.v1 / nyon.capital-start.v1.
Policy is role:u8,O(civilization:u32) and is **present even at tick0**; validate
existing role/selected-civilization rules. Tick/revision/sequence0 are in global
metadata. Mechanics/annex versions pin the exact numerical tables/formulas in
this proposal; there are no tunable hidden recipe overrides. Manifest is the
unchanged verified seeded wire<=16,384bytes, never a changed v0 genesis.

Quota positions0..33 copy the exact34 numeric values/names of the initial
full-scale codec's frozen vector as profile facts (they do not authorize its
record grammar). Additional positions34..51, in order, are:
`256 buildings/city,512 population/city,4096 stock+incoming hard bound,
512 base storage,16 base hall housing,2048 training global,32 training/world,
4 training/city,4096 units+reservations global,256 units+reservations/world,
2048 objectives global,16 objectives/world,6144 effects/tick,
4096 damage targets/tick,2048 occupations/tick,8388608 tactical scratch bytes,
4 rounds/hour,64 max local cargo`. These values are fixed, not host-selected.
A composed storage owner must accept this profile and actual layouts before it
can be registered. Reserved strategic quota families admit no opaque records.

A canonical record is `record_length:u16 || realm:u32 || kind:u16 || local:u64 ||
payload`; length counts the14-byte key plus payload, excluding its own2bytes.
Numeric `(realm,kind,local)` traversal, strictly increasing unique keys. World
and local entities use realm=PlanetId; global metadata/units/objectives use realm0.
The following city records are exact; unused physical page bytes are zero and
excluded from wire. A missing claim is unoccupied manifest terrain.

| Kind | Key | Payload in exact order |
| --- | --- | --- |
| 0 global | realm0,local1 | tick:u64,revision:u64,sequence:u64,Policy,next_unit:u64,next_objective:u64,counts[7]:u32 for city/building/claim/shipment/training/unit/objective |
| 1 world | realm=planet,local1 | next_city:u64,next_building:u64,next_shipment:u64,next_training:u64,counts[7]:u32 in the same order (unit count excludes training reservations) |
| 2 city | realm=planet,local=CityId | owner:u32,hall_local:u64,population:u16,health_debt:u8,food_streak:u16,water_streak:u16,growth_streak:u16,growth_enabled:bool,construction_enabled:bool,hall_integrity:u16,occupation_tick:O(u64),stocks:V,incoming:V |
| 3 building | realm=planet,local=BuildingId | city:u64,kind:u8,anchor_local:u64,facing:O(u8),footprint_count:u8,footprint_locals:u64[count],enabled:bool,priority:u8,remaining_work:u16,initial_work:u16,paid_cost:bool,integrity:u16,activation_tick:u64 |
| 4 claim | realm=planet,local=cell coordinate | city:u64,occupant_kind:u8 (hall0/building1),building:O(u64) |
| 5 shipment | realm=planet,local=ShipmentId | source_city:u64,destination_city:u64,launch_owner:u32,commodity:u8,amount:u32,departure:u64,position:u8,stalled_ticks:u32,last_stall:u8,path_count:u8,path_locals:u64[count] |
| 6 training | realm=planet,local=TrainingId | city:u64,class:u8,crew:u8,remaining:u8,initial_duration:u8,paid_goods:V,reserved_unit:u64,created_tick:u64 |

Kinds7 unit,8 objective and9 theater are reserved for the tactical owner's exact
payload amendment; no unspecified bytes may enter a decoder. Theater key is
realm=planet/local1; its sixteen civilization victory counts must have an explicit
bounded payload. No separate training-reservation kind exists. Global/world unit
counts count live/reserve units only; roster admission additionally sums training
counts. City/training owners are derived from current city, never duplicated.
No additional last-hour summary is canonical: copy views derive it from the exact
hour receipts; city state above is complete. This refines the earlier record-size
outline rather than adding invisible fields.

A building project's integrity=0,activation_tick=0,remaining>0; a completed
building has remaining0 and activation>=0. Its initial_work and refundable
paid_cost are retained; paid costs derive from fixed kind table. Authored/genesis
completed buildings have paid_cost=false,remaining0,initial_work=table work and
activation=admission tick or0. Road completed integrity is1000. Footprint order is
anchor then resolved neighbor, not sorted; count1 requires absent facing and
count2 only farm. Claims are sorted by key independently. Shipment path is in route
order with2..65 distinct cells, position<path_count; completed arrivals have no
live record. Stall tags none0,source_owner1,destination_owner2,source_hall3,
destination_hall4,next_edge5. Training quantities/costs must equal class constants.
Occupation absent until first capture, thereafter its last successful tick; it
suppresses work only when equal to current target. Counters/counts cross-check all
records; no stale next-ID or dangling cargo/grant reference is canonical.

Genesis contains one global record, one world record per enrolled world, and for
capital start one city/fourteen buildings/sixteen claims per civilization;
no shipments/training/units/objectives. Proposed kind9 theater has one explicit
record per enrolled world with sixteen u32 victory counts, all initially zero;
nonpresent civilization counts must remain zero. The tactical owner must ratify
this exact theater payload rather than choose a second initial-state encoding.
Next city/building are2/15 on founding worlds, otherwise1; shipment/training and
global unit/objective next IDs1. Counts match those exact objects. All initial
records are recomputed from manifest/scenario rather than trusted decoded grants.
Upper bound for the city-bearing genesis object is
`512*96 +16*128 +224*96 +256*64 +512*80 +16384 +4096 =150,528bytes`;
choose explicit object cap262,144. This includes conservative framed record sizes,
zero theater records and fixed header/quota/string/global slack. The tactical
owner must ratify its80byte framed theater record before composition approval.

Let `G=SHA256("NYON-RC1-GENESIS"||genesis_object)`;
`S=SHA256("NYON-RC1-STATE"||manifest_digest[32]||S(rules_identity)||
canonical_global_and_entity_records)`;
`H0=SHA256("NYON-RC1-HISTORY")`;
`Hnext=SHA256(Hprevious||u32(receipt_byte_length)||receipt_bytes)`.
No NUL/implicit domain lengths. The state digest S includes policy,counters,all live canonical
city/tactical records and the shown manifest/identity prefix; it excludes history
head to avoid recursion. Here S(text) remains the length-prefixed string encoding;
the standalone digest symbol S is the32byte hash, never a string encoding. G and session logical-genesis/transport digests are distinct;
provider registration must use this G/history recurrence without changing the
initial full-scale codec's NYON-FS1 domains. Physical scheduling is excluded. The composed authority keeps the storage
provider's fixed NYON-FS1 transport-tree domains as a transport protocol, using
this composition's G and history head in Prefix. The logical city domains above
do not rename transport hashes or session NYON-ES1 node digests; the provider and
session owners must verify that registry distinction before accepting the codec.

### Commands and rejection registry

Outer command frame is `planet:u32,body_length:u16,body`; body grammar is the
previous sequence/provenance/actor/family/tag definition. All city Q refs must
match outer planet. City command payloads are exactly the fields in the command
table, using Q for city/building/shipment/training, Cell=Q, u8 for kind/class/
priority/commodity/source_kind, u32 for amount, u16 for population, V for goods,
bool for enable/policy/bypass, O(u8) for facing, and path_count:u8 then Q[path_count].
Field order is left-to-right in that table. `grant` stock vector precedes population;
`author_city` owner:u32 precedes cell; source_kind transfer0/mint1; repair kind
hall0/building1 precedes Q. `set_city_policy` is growth bool then construction bool.
Known enum mismatches to constraints reject; unknown enum byte fails structural
validation. `family0` is city with tags0..17; tactical family1 tag/payload grammar
must be supplied by its owner before a composed decoder exists.

Structural precedence is lifecycle/expected revision/target; trusted policy;
counts/body lengths; input decoding by original input ordinal; unknown outer
world/per-world count; incremental global sequence sorting/zero/old/duplicate.
This completes before gameplay. Invalid poll budget is a request error without
job mutation. Gameplay rejection precedence follows the main command section;
within quotas global→world→city→tactical roster; then IDs/eligible horizon; then
return population/storage; then resources by commodity tag; then admission crew.
Connection/functional-capacity evaluation uses candidate state, with all lookup/
BFS work charged. One rejected command has one outcome; published rejections
advance the sequence watermark, aborted ticks do not.

Reject wire tags:1 CreatorForbidden,2 InvalidActor,3 RulerForbidden,
4 CivilizationRequired,5 WrongCivilization,6 InternalCommandOnly,
7 ProvenanceMismatch,8 ExplicitBypassRequired,9 UnknownTarget,
10 UnknownCivilization,11 WrongOwner,12 InvalidScalar,13 InvalidCell,
14 InvalidFacing,15 TerrainMismatch,16 OccupiedCell,17 RepeatedCell,
18 NonAdjacentPath,19 PathEndpointMismatch,20 Disconnected,21 NoRoadAccess,
22 GlobalCapacity,23 PlanetCapacity,24 CityCapacity,25 IdExhausted,
26 PopulationCapacity,27 HousingCapacity,28 StorageHardLimit,
29 FunctionalCapacity,30 InsufficientResource,31 InsufficientPopulation,
32 InvalidTargetState,33 PathInUse,34 CapacityNoBenefit,35 TrainingCapacity,
36 RosterCapacity,37 InvalidClass,38 TrainingHorizon,39 ReturnCapacity.
These are new-city codes, never claimed to equal v0 codes. InvalidClass is reserved for a known class without a city recipe; all three
current classes have recipes, so this revision never emits it. A byte outside
the known class enum is a structural failure. Non-role codes are checked only after existing role admission.

### Independent scalar and arithmetic anchors

A disposable Python standard-library scalar pack, written from this appendix and
without importing game code, produces this city recruit fixture: outer planet1,
sequence1,ruler provenance1,actor present1,family0,tag13,city(planet1,local1),
class infantry0. Body length30, complete outer frame36bytes:

```text
body = 0100000000000000010101000000000d0001000000010000000000000000
frame = 010000001e000100000000000000010101000000000d0001000000010000000000000000
```

This pins family width, actor presence, tag width and qualified city order. It is
an independently packed design witness, not a passing game decoder result.
Scalar-width arithmetic also yields city payload98bytes maximum, building53,
training60, world60; key+length adds16bytes. Shipment canonical route can exceed
one page and remains encoded incrementally from header plus its three child
pages. Complete event/genesis/role/rejection/maximal tactical hex and hash fixtures
must still be checked in and independently reviewed before implementation.

### Framed events and sole emitters

Receipt header is `tick:u64,phase:u8,planet:u32,ordinal:u32,tag:u16` (19bytes);
frame adds length:u32 (23byte total fixed overhead). Earlier conservative event
bounds allow27bytes, retaining4byte slack; do not accidentally encode a second
length. Payloads below are exact city records. A bounded field array is always
preceded by its specified count; there is no hidden native struct padding.

| Tag / owner | Payload |
| --- | --- |
| 0 genesis / coordinator | G[32],manifest_digest[32],initial_state_digest[32],scenario:u8,city_count:u16,building_count:u16 |
| 1 scenario_city / city | city:Q,city_payload as kind2 above |
| 2 scenario_building / city | building:Q,building_payload as kind3,waived_goods:V,waived_work:u16 |
| 3 scenario_stock / city | city:Q,commodity:u8,amount:u32 |
| 4 tick_input / coordinator | previous_revision:u64,previous_sequence:u64,previous_policy:Policy,current_policy:Policy,command_count:u16 |
| 5 command_accepted / command-family owner | sequence:u64,family:u8,command_tag:u16,created_local:u64,counterpart_id:u64,debit:V,credit:V,population_moved:u16,waived_goods:V,waived_work:u16 |
| 6 command_rejected / command-family owner | sequence:u64,family:u8,command_tag:u16,reject:u16,offending_index:O(u16),commodity:O(u8) |
| 7 freight_hour / city | shipment:u64,source_city:u64,destination_city:u64,launch_owner:u32,commodity:u8,amount:u32,position:u8,stalled_ticks:u32,reason:u8,outcome:u8 (moved0,stalled1,delivered2) |
| 8 city_hour / city | city:u64,owner:u32,population_before:u16,population_after:u16,food_demand:u16,food_supplied:u16,water_demand:u16,water_supplied:u16,housing:u16,available_workers:u16,assigned_workers:u16,power_produced:u16,domestic_power:u16,operation_power:u16,unused_power:u16,cared_people:u16,debt_before:u8,debt_after:u8,food_streak:u16,water_streak:u16,growth_streak:u16,occupation:bool,stocks:V,incoming:V |
| 9 building_hour / city | building:u64,city:u64,kind:u8,status:u8,workers:u16,power:u16,inputs:V,outputs:V,work_before:u16,work_after:u16,integrity:u16,activation_tick:u64,paid_cost:bool |
| 10 population_change / city | city:u64,kind:u8 (growth0,death1,growth_housing_blocked2),cause:u8,deficiencies:u8,before:u16,after:u16 |
| 11 tick_commit / coordinator | revision:u64,sequence:u64,state_digest[32] |
| 12 command_input / coordinator | input_index:u16,planet:u32,body_length:u16,body[body_length] |
| 32 training_hour / city | training:u64,city:u64,reserved_unit:u64,class:u8,crew:u8,workers:u16,remaining:u8,outcome:u8 |
| 33 deployment_grant / city | producer:Q,reserved_unit:u64,owner:u32,world:u32,hall:Structure,class:u8,crew:u8,eligible_tick:u64 |
| 34 city_effect_input / tactical | type:u8 (damage0,occupy1),effect_id:u64,source_revision:u64,target:Structure, then damage amount:u16 OR occupy objective:u64,expected_owner:u32,new_owner:u32 |
| 35 city_effect_outcome / city | type:u8,effect_id:u64,target:Structure,objective:O(u64),previous_owner:u32,new_owner:u32,integrity_before:u16,integrity_after:u16,outcome:u8 (applied0,blocked_destroyed_hall1) |
| 36 effect_batch / tactical | source_revision:u64,target_tick:u64,effect_count:u16 |

Tactical raw UnitRound/ObjectiveRound/Theater event tags64,65,66 respectively
are reserved. Their exact payloads remain tactical-owner
amendments. No city-effect outcome is emitted a second time by tactical. Input
records for the complete public batch occur once in phase0 in sorted sequence
order, retaining original input_index; outcomes occur in command execution order.
Per-phase events follow documented world/entity/tier order. Each building gets
exactly one hourly status, including passive/disabled/newly completed projects;
completion is not a second building event. Solar has outputs zero stored goods
and status solar_output with power=generated; operation_power excludes that value.
City_hour is the sole household consumption summary, using supplied food/water. Scenario_stock
is the sole initial commodity-source event; scenario_city stocks are a state
assertion and must not be counted as a second endowment. Scenario_city population
is the sole initial resident-source event; scenario_building waived vectors never
enter goods custody. Construction inputs/recipe inputs occur only in their actual
command/building event, so a state snapshot cannot be mistaken for a second sink.
There is at most one population_change per city/hour, and at most one training_hour
and one grant per job/hour. One freight_hour per live pre-phase shipment admitted before this tick; new
departure shipments emit no freight_hour until a later tick. Their command
receipt is the departure event.

Building status tags: productive0,disabled1,ruined2,not_active3,disconnected4,
occupation5,hall_ruined6,workers7,inputs8,power9,output_capacity10,
construction_progress11,construction_complete12,passive13,solar_output14,
awaiting_strategic_rules15. Construction toggle=false uses disabled. Training
outcomes progressed0,idle_hall1,idle_occupation2,idle_workers3,idle_horizon4,
completed5. Population cause none0,water1,food2,health3; deficiency bit0 food,
bit1 water,bit2 power,bit3 housing, all upper bits zero. Zero-population city gets
city_hour with zero needs/work and no population_change. GrowthHousingBlocked uses cause0. The next table supplies exact accepted-command
field semantics, replacing the earlier generic two-city transfer description.
Tactical-family mappings remain its owner's review obligation in the same256byte
outcome envelope.

### Accepted city-command effect mapping (tags0..17)

In every row, `debit` and `credit` describe **city stock changes at command
admission**, never a destination reservation or cargo already in a shipment.
`0` means the all-zero V or scalar of that column. `Cost(k)` is the listed eight-
commodity building price; `A` is amount in the command's one commodity;
`Sup=[32,32,0,0,0,0,0,0]`; `FoundCost=[0,0,16,16,0,0,4,0]`;
`ResettleCost=[0,0,4,0,0,0,1,0]`; `Repair=[0,0,2,2,0,0,0,0]`.
Refunds use the actual paid flag/cost: project proportional to remaining work,
completed dismantle half, training half regardless of progress. The command input
identifies the city whose stock is affected; the table identifies the side for
multi-city commands. `population_moved` is an unsigned magnitude; its direction
and mint/transfer/return meaning are fixed by the command tag, never guessed from
an inventory delta. IDs are local u64 except the explicitly global UnitId.

| Tag / command | created_local | counterpart_id | debit | credit | population_moved | waived_goods / waived_work |
| --- | --- | --- | --- | --- | --- | --- |
| 0 build | New BuildingId | 0 | Cost(kind), owned city | 0 | 0 | 0 / 0 |
| 1 set_enabled | 0 | Target BuildingId | 0 | 0 | 0 | 0 / 0 |
| 2 set_priority | 0 | Target BuildingId | 0 | 0 | 0 | 0 / 0 |
| 3 set_city_policy | 0 | Target CityId | 0 | 0 | 0 | 0 / 0 |
| 4 cancel_project | 0 | Retired BuildingId | 0 | Actual proportional paid refund, target's city | 0 | 0 / 0 |
| 5 dismantle | 0 | Retired BuildingId | 0 | Actual half paid refund, target's city | 0 | 0 / 0 |
| 6 ship_local | New ShipmentId | 0 | A, source city | 0 | 0 | 0 / 0 |
| 7 found_city | New CityId | Source CityId | FoundCost+Sup, source city | Sup, new city | 4, source→new city | 0 / 0 |
| 8 author_city | New CityId | 0 | 0 | Input stocks, new city (mint) | Input population (mint) | 0 / 0 |
| 9 author_building | New BuildingId | 0 | 0 | 0 | 0 | Cost(kind) / kind initial_work |
| 10 grant | 0 | Target CityId | 0 | Input stocks, target city (mint) | Input population (mint) | 0 / 0 |
| 11 author_shipment, transfer | New ShipmentId | 0 | A, source city | 0 | 0 | 0 / 0 |
| 11 author_shipment, mint | New ShipmentId | 0 | 0 | 0 | 0 | 0 / 0 |
| 12 resettle_city | 0 | Destination CityId | ResettleCost+Sup, source city | Sup, destination city | 4, source→destination | 0 / 0 |
| 13 recruit | New TrainingId | Reserved global UnitId | Class price, recruiting city | 0 | Class crew, city→training custody | 0 / 0 |
| 14 cancel_training | 0 | Released global UnitId | 0 | Half paid_goods, job's current city | Full crew, training→city | 0 / 0 |
| 15 repair_structure | 0 | Target hall CityId or BuildingId, per input kind | Repair, target's city | 0 | 0 | 0 / 0 |
| 16 author_repair_structure | 0 | Target hall CityId or BuildingId, per input kind | 0 | 0 | 0 | ceil(restored_integrity/250)*Repair / 0 |
| 17 cancel_shipment | 0 | Retired ShipmentId | 0 | A, current source city | 0 | 0 / 0 |

Created IDs identify only newly created objects; never put a retired object into
created_local. Counterpart meanings are exactly the table; unlisted fields are
zero. For cancel_training the input already identifies the retired TrainingId,
while counterpart preserves its released/burned UnitId. No ID is reused after
any retirement. Target kind for repair is fixed by input, not inferred from a
potentially equal hall/building local number.

At ship/author-ship admission the destination stock is unchanged, incoming grows
by A and a new shipment takes custody of A. The credit vector is zero even when
the road path ends next door. For creator mint, the admitted original tag11 input
**source_kind=mint,commodity,amount** plus accepted created ShipmentId is the
explicit mint-source record; it adds A to cargo, with no source debit, city credit
or cost waiver. Transfer versus mint cannot be inferred from zeros alone: read
the complete authenticated command_input linked by sequence. Only a later
freight_hour(outcome=delivered) credits destination stock A and removes incoming
A/cargo A. It is a transfer event, never a second mint. Cancel instead credits
source A and removes destination reservation/cargo; that refund creates no goods.

For founding/resettlement the two city stock changes occur immediately in commands.
`debit-credit` is precisely FoundCost or ResettleCost, the consumed material sink;
Sup moves custody without creation. Author_city/grant credits are explicit mints,
not transfers. Recruit consumes its entire class goods price and moves existing
people into training custody. Training cancellation's half-goods credit is the
explicit bounded refund source already allowed by the conservation equation;
all crew returns. Building creation/repair waivers are explanatory only and never
enter a commodity-source sum. Zero-effect policy commands still emit their exact
zero vectors and target counterpart. Replay compares these fields to the derived
candidate effects, not merely to the record's size or checksum.


### Narrow fix2 independent byte and custody fixtures

These fixtures were independently packed from the specified scalar grammar with
Python standard-library `struct`/`hashlib`, without importing authority or a game
serializer. They are Proposed byte witnesses, not executed Zig/decoder tests.

For the side4/terrain-residue0 capital placement above, realm1/city1, concatenate
kind3 canonical records in BuildingId1..14 order. Single-cell payload44bytes,
farm payload53; each has16bytes of key/length overhead. Total849bytes; SHA256:
`d4587faaf1900d6eeccb988b2d17d22946b00290b0aa948c5256e8e7ef40ee11`.
This digest is only those fourteen building records, not full genesis G. The
Solar13 canonical record (60bytes,including length/key) is exactly:

```text
3a000100000003000d000000000000000100000000000000051b0000000000000000011b0000000000000001010000040000e8030000000000000000
```

Changing any of the14 enabled/priority bytes must fail recomputed-genesis equality,
even if another serializer recomputes a different internally consistent digest.

The following accepted-command frames are three **separate** admitted command-
phase fixtures with tick1,phase1,planet1,ordinal3,receipt tag5,sequence1,family0.
Each has length field146,19byte header,127byte payload:150 framed bytes, within
the unchanged256byte bound. The preceding phase0 tick-input/header and one full
command_input occupy ordinals1/2 and bind all omitted target/path/provenance fields.
Transfer is tag6,source city1,destination city2,food4,valid road path,created
ShipmentId7. Mint is tag11 with creator provenance,bypass=true,source_kind=mint
and otherwise the same endpoints/cargo/new ID. Founder is tag7,source city1,new
CityId3 with the defined4people and Sup, at a valid unclaimed target. Fixture
prerequisites include sufficient goods,owners,capacity,IDs and valid geometry;
these byte checks do not claim that a running game exercised those admissions.

Transfer accepted frame:

```text
920000000100000000000000010100000003000000050001000000000000000006000700000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
```

SHA256 `f3f1f3ce748c235e63fc0d40261de274d78e52a14166fcdf00a7e395afd5e4e3`.
Source debit food4,credit all0,population0. Destination incoming+4 is represented
by the admitted command and shipment state, not by stock credit.

Minted-cargo accepted frame:

```text
92000000010000000000000001010000000300000005000100000000000000000b000700000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
```

SHA256 `2fcd17eccf0dc22541a667629ae43a0b9ab31860199f2de346e58f30f25286b7`.
Debit/credit/waiver vectors and population are all0; linked tag11 mint input is
explicitly the four-unit cargo source. Stock credit waits for arrival.

Founding accepted frame:

```text
920000000100000000000000010100000003000000050001000000000000000007000300000000000000010000000000000020000000200000001000000010000000000000000000000004000000000000002000000020000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000000000
```

SHA256 `9bb4c993121736656ebece2ef3c292a0e821f93067905fa6a3350c44f62a39c9`.
Debit `[32,32,16,16,0,0,4,0]`,credit `[32,32,0,0,0,0,0,0]`,population_moved4,
created_local3,counterpart1; waiver all0. The exact net sink is16timber,16stone,
4tools, and four residents move rather than being minted.

Independent command-boundary custody arithmetic (no household phase included):

| Branch from source food100/destination20/no cargo | Source stock | Destination stock | Destination incoming | Cargo | Owned food total |
| --- | ---: | ---: | ---: | ---: | ---: |
| Transfer admission | 96 | 20 | 4 | 4 | 120 |
| Transfer arrival | 96 | 24 | 0 | 0 | 120 |
| Creator mint admission | 100 | 20 | 4 | 4 | 124 |
| Minted cargo arrival | 100 | 24 | 0 | 0 | 124 |

Incoming is excluded from owned totals. At a founder command boundary source
population12 becomes8 plus new-city4; source stock vector
`[100,100,100,100,10,10,10,10]` becomes
`[68,68,84,84,10,10,6,10]` and new-city stock is Sup. Per-commodity summed
stocks differ by FoundCost exactly. These checks detect false destination credit
at departure, duplicated mint at arrival and treating consumed founding materials
as transferred supplies.

Conservative **framed** city-only bound remains6,258,688bytes:
`1024*(1056 input+256 outcome)+16384*192 building+4096*(256 city+64 population)
+4096*96 freight+65536 headers/commit/slack`.
Add training `2048*128 +2048*128`, effect inputs `6144*96`, and city outcomes
`6144*96`: city+integration subtotal7,962,624bytes. These effects are counted once.
Raw tactical rounds reserve `16384*256 +8192*64 +512*128 =4,784,128bytes`;
use128 for theater, since16 civilization u32 scores cannot fit its older32byte
summary estimate. Shared input/outcomes/headers are already in the city bound.
The proposed **composed tick reservation is12,746,752bytes**, below32MiB, pending
largest-payload/multiplicity witnesses and tactical-owner acceptance. Do not sum
the old annex/tactical effect/outcome estimates again. This is an upper envelope,
not measured tick output or proof of the5ms target.

Genesis object bytes are stored once outside receipt
framing; do not duplicate its manifest as transcript chunks. Initial receipt
stream bound is `192+16*256+224*192+128*64 =55,488bytes`, cap65,536. Emit one
genesis event, then cities byplanet with scenario_city, buildings byID, commodity
stock events bytag; scenarios/claims/costs are deterministically regenerated from
genesis. Empty start emits only genesis. Segments are transport boundaries only;
no per-segment receipt can change these bytes. Session registration must admit
262,144byte genesis and65,536byte initial stream plus the composed12,746,752tick
reservation; its latest1MiB genesis/initial and32MiB tick envelope permits those
numbers arithmetically, without proving provider/retention correctness.

## Conservation, jobs and failure semantics

For each commodity at every published revision:
`initial scenario + explicit creator mint + extraction + recipe output + refund
 - recipe input - construction - household consumption - export + import
 = city stock + local cargo`.
Imports/exports are zero until a reviewed strategic composition exists. Within a
complete future composition their cargo appears once in strategic custody, with
matching boundary receipts. Reservations are promises, not extra material, and
never enter the owned-quantity sum. Refunds are bounded by paid costs; creator
waived-cost buildings have zero refundable value. Population custody is city residents+training crew+live/reserve unit crew =
initial scenario+explicit creator people+growth-city deaths-tactical casualty
sinks; founding/resettlement/recruitment/cancellation/deployment transfer custody
without creating people. Partial unit HP damage does not remove crew; unit death
has one explicit full-crew tactical sink. Health,
workers, housing and power have separate equations; power production equals
domestic use+recipe/service use+unused. Water source and solar generation are
explicit rule sources, not concealed conservation errors.

Per city and commodity enforce stock+incoming<=4,096; every incoming unit has
exactly one live local shipment in this composition; over_capacity is a derived
functional-capacity condition, never permission to exceed the4,096 hard bound. Per hour assigned workers<=snapshot
workers and consumed power<=produced. Every building footprint/claim is bijective;
every operational network connects to its own hall; every shipment preserves its original unique adjacent path and valid endpoint
references; current ownership/access can legitimately make it stalled. Streaks, IDs, priorities, paid-cost flags and scenario origin are canonical.

Use the Proposed full-scale single owner/lane/COW lifecycle and immutable copied
queries, not separate city jobs. `beginTick` snapshots bounded inputs; all routing,
sorting, graph checks, costs, mutations, phases, codecs, hashes, validation, abort
and retirement consume the same caller allowance0..4,096. Budget0 changes nothing.
At most one bounded record operation, one edge/neighbor, one index movement,
one<=256byte copy/zero or one<=64byte hash block per unit. Variable recipe joins,
route membership and graph reachability use resumable cursors. Sort priorities
with metered stable comparisons/moves; never sort each city invisibly in one unit.
Two256-entry building lists and a6,144-cell visited/queue scratch can be reused
between worlds only after the world's infrastructure phase cursor finishes. Bounds/layout and
cancellation ownership require proof against the8MiB scratch reservation.

No allocation during poll. All candidate state remains invisible until invariants,
receipts and descriptor are complete. Any arithmetic, storage invariant or history
encoding defect aborts the entire tick. Gameplay resource/placement shortages
reject only the command or idle the operation according to rules. Stale job/view
requests do not mutate a newer lane. Cancel before publication discards candidate
claims/resources/IDs/history under budget and preserves committed sequence and
policy. After publication cancel reports AlreadyPublished; cleanup still drains.
Old leases can delay retirement/next tick, never expose half a city or world.
Cleanup corruption enters faulted admission; no attempt repairs an economic
ledger by silently dropping cargo. Host save/Continue cannot claim this identity
until the independent durable-session codec can retain/replay it completely.

## Player controls and observable consequences

New-game setup shows scenario, role, civilization, capital population/resources,
waived starting buildings and rules identity before creation. A ruler enters
capital view with visible production, stock runway and pending command queue.
The city inspector shows per-hour food/water/power balance, capacity/reservations,
housing, population/debt, workers by tier, growth streak and exact next action
cost. A paused game stages previews; only a submitted tick changes authority.

Building inspector shows priority, enable switch, crew/work, connection path,
paid/refundable cost and first idle reason. Supply deficits and fixed tier order
are explained using actual candidate preview values, not claimed future certainty.
Road overlays show qualified seam neighbors and blocked/occupied cells. Freight
shows departure, path, current position, projected ETA or exact stall reason,
launch/current endpoint owners, cargo custody, cancel outcome and reservation. Receipts
allow explaining why a workshop idled while a farm used the remaining workers.

Typical decisions: spend timber/stone/tools on food redundancy or more industry;
disable growth to retain stock; run care at the expense of construction workers;
choose industry priorities to close the ore→alloy→tools loop; extend roads for
satellite founding and freight; reserve cargo space versus local production.
Stock full, shortage, lost growth and deaths are explicit outcomes. External trade
buttons remain unavailable with reason “Strategic transport rules unavailable”
until qualified integration. UI rendering, accessibility, host input, undo and
real save/relaunch remain distinct acceptance work, not implied by this design.

## Independent fixtures and complete gameplay journey

Before implementation, derive expected arithmetic and wire with an independently
written Zig/std oracle, not implementation-generated goldens. Hand arithmetic:

1. Capital with all startup infrastructure connected and no commands:12workers;
   waterworks2+farm4+forestry2+quarry2=10, spare2. Power24-domestic3-farm4-quarry2
   =15unused. End hour1: food200,water264,timber102,stone102,ore26,alloy24,tools16,
   medicine8; population24,health0,growth0 (no clinic). End hour2:208,272,108,108,
   28,24,16,8. These follow source40water-farm8-households24 and farm32-food24.
2. Paid Residence at hour1 costs timber12,stone8,tools1; two construction workers
   progress4→2. End hour1 stocks food200,water264,timber90,stone94,ore26,alloy24,
   tools15,medicine8. Hour2 completes (remaining0); housing32 remains through
   hour2's needs and becomes48 at hour3 infrastructure. Existing10 operating
   workers plus2builders fit12. No same-hour housing benefit is inferred.
3. A new frontier has4people,32food/32water at admission, consumes4/4 immediately
   and ends28/28. No production/clinic implies no growth. With no aid, it ends0/0
   on its eighth hour; water-short streak1,2,3 causes first death on hour11,
   food shortage simultaneously present; one death, both streaks reset. Health
   accumulation does not cause an earlier debt100 death in this fixture.
4. An unstalled4-unit food shipment over `[70,71,4,5]` at side4 has3edges and arrivesT+3 when all three moves remain legal;
   the independent cube witness +Z(3,1)=71 east→+X(0,1)=4 validates the seam.
   Build/author the endpoints and road71/4 explicitly in a fixture; farm occupancy
   on71/4 would reject the road route. Destination reservation4 prevents a
   producer from filling those4slots. Budget count cannot accelerate arrival.
5. Disable all supply in an isolated healthy4-person city with0food/water and
   ample housing/power. Debt16,32,48 at first3hours; third-hour water death is1,
   not two competing death events. With full food/water/power/housing next hour,
   debt becomes47 without clinic. Add full care to obtain39. Workers use the
   beginning debt/population, never the just-recovered amount.
6. Foundry output-capacity shortage consumes no ore/power/workers. At stock
   alloy510,incoming1,capacity512, output2 idles; at stock509 it succeeds exactly.
   A quarry with stone capacity available but ore capacity full produces neither.
7. Creator/ruler/combined matrix; authored structures refund0; paid project
   cancellation at remaining2/initial4 returns floor(half each paid commodity);
   wrong owner/actor, missing bypass, forged scenario origin and autonomous
   envelope reject without canonical economic effects.

The executable acceptance story must use public begin/poll/query commands and
trusted capital creation, never hand-mutate world state. Start pure ruler with
capital-start, inspect endowment receipts; run2hours and compare fixture1; build
and complete a Residence; build a Foundry and Workshop using2spare builders,
then pause Forestry/Quarry selectively to demonstrate worker scarcity and
industry priorities. Produce alloy/tools and verify conservation against exact
receipts. Build an Apothecary and Clinic; accumulate medicine by choosing a
medicine production interval, using eight successful Apothecary hours to add16medicine to the initial8,
then disable Apothecary, enable Clinic and supply it for24consecutive
hours with sufficient housing to grow exactly1resident. During care, reduce
industry staffing explicitly; life6+clinic2 leaves4workers. No growth test may
quietly assume all industries remain staffed. Build roads, found a satellite,
deliver food/water before its three-shortage-hour deadline and establish its
waterworks before continuing. Demonstrate rejected overlapping seam placement,
full destination reservation, a paused construction project and an intentional
recoverable supply shortage; inspect all consequences. Save/restart/Continue is
added only once the separate durable composition is reviewed and implemented.

The detailed plan must pin the exact world/cell route, command schedule including
paid training/deployment, per-hour
stock table and oracle digest for that entire story before implementation. In
particular the satellite4population supplies only2workers: it cannot independently
run a4worker farm. It needs sustained food imports until growth or additional
population creates sufficient labor. The scenario must demonstrate an actually
sustainable supported settlement, not claim this frontier is self-sufficient.

8. Full-hour capital recruitment fixture at tick1: recruit one infantry, no
   other commands. Population24→23 before snapshot; workers12→11. Waterworks2,
   farm4,training2,forestry2 use10; quarry idles_workers with1 unassigned.
   End tick1 vector `[193,265,102,96,24,24,14,8]`, training remaining1,
   city23+training crew1=24. Tick2 empty: `[202,274,108,96,24,24,14,8]`, job
   removed, reserve UnitId1/crew1/eligible3; city23+reserve1=24. Power unused17
   each hour. No clinic means growth0. With an otherwise empty friendly hall,
   deploy at tick3 pays no second goods/people: ordinary capital operations end
   `[211,283,114,102,26,24,14,8]`, city23+deployed crew1=24. Hostility absence
   here is a fixture input, not proof that the strategic hostility producer exists.
9. Alternative branch cancel after recruitment tick1: tick2 command returns
  4food/1tool/1person, burns reserved UnitId1 and releases quota. Full capital
   tick2 ends `[205,273,108,102,26,24,15,8]`, population24, no unit/job, next
   UnitId2. Compare against the completing branch to catch accidental full refund,
   retained trainee workforce or duplicate reserve creation.
10. Damaged storage fixture: food stock600,incoming64, functional1024. Destroy
    its one warehouse: retain sum664 with functional512. A valid reserved64-food
    arrival yields stock664,incoming0. Canceling infantry training requiring
   4food refund fails ReturnCapacity, preserves crew/job/unit reservation. Repair
    warehouse from0→250 for2timber/2stone restores1024 capacity; with sufficient
    population room the same cancellation now credits food4/tools1 and returns
    one resident. Repair creates no food. Repeat with captured city and recorded
    new-owner policy, asserting all IDs and pending custody preserved.
11. Capture freight fixture: cargo4 on a3-edge route, creationT, move to index1
    atT+1. Capture destination before freightT+2: position1,reason destination_owner,
    stalled_ticks1; no arrival at originalT+3. Reclaim for launch owner before
    freightT+3: index2; T+4 delivers, reservation4→0. On an alternative branch,
    source capture lets its new current owner cancel, but source stock510+return4
    exceeds512 and rejects atomically. After goods consumption or repaired storage
    creates room, successful cancellation returns4 and releases the destination
    reservation once, retaining the burned ShipmentId. Unauthorized destination-
    owner cancellation rejects; cargo never silently transfers on capture.
12. Occupation at24 residents with pre-hour food/water sufficient and intact
    solar: workers0,care0,growth0; solar24,domestic3,unused21. Households consume
   24food/24water. No construction/training/industry progresses. A ruined hall
    instead loses its16 housing; surviving Residence supplies16, creating8
    overcrowded residents and the exact+2 debt contribution. No population is
    implicitly deleted by capacity recalculation. Road target damage rejects the
    whole internal batch before any valid neighboring effect applies.

Additional required evidence: exhaustive seed low2bits×side4..32 startup placement;
all directed seams/corners from an independent basis oracle; road/project
connection and dismantle cancellation at every graph cursor; initial capacities
and maximum advertised simultaneous cities/buildings/cargo; malformed codec and
foreign references; source/sink identities including refunds; health/death/growth
thresholds and fractional clinic coverage; per-hour ordering with varied IDs and
priorities; cancellation at every phase/COW transition and immediate prepublish;
old views after publication; zero-budget/budgets1,7,256,4,096 and randomized polls
produce identical canonical bytes; duplicate/stale/exhausted tokens; zero poll
allocations and measured maximum poll latency separately; byte-bound emit-site
proof, history backpressure, fresh replay and exact schema rejection.

## Review gates and delivery boundary

1. Independently review this numeric/mechanical contract and its capital recipe.
   Controller selects the reviewed revision under the recorded delegation.
2. Resolve full-scale storage ownership, authenticated history provider and codec
   review; freeze the composed city schema/phase and exact maximum-event grammar.
   Prove startup placement,256byte record layouts, scratch and whole-root bounds
   with pinned-compiler witnesses. A changed storage profile requires new review.
3. Write/review the detailed implementation plan and independent complete-story
   fixtures before source work; one owner mutates shared authority modules.
4. Implement pure city semantics and integration, run `zig/tools/check.sh` with
   city gates and independent review. Preserve existing v0/seeded vectors and
   Rust-free required pipeline. Record exact command, exit status and artifacts.
5. Qualify durable sessions and actual host gameplay separately. The city CLI
   does not establish strategic transport, diplomacy, research, full tactical
   gameplay, native
   graphics, browser behavior, accessibility or complete-game delivery.

## Reconciliation disposition and exact cross-owner amendments

This revision addresses the city-side substance of review issues1–8 and11:
one registry; preserved involuntary over-capacity; capture stalls/cancellation;
one numeric phase/workforce registry; trusted capital; one road/quota model;
paid crew custody; and explicit city records/events/genesis/rejections. The review's
raw statuses remain untouched. These changes are Proposed and need fresh scoped
independent review; no issue is declared integrated PASS by its author.

| Other owner | Required matching amendment / remaining proof |
| --- | --- |
| City annex | Replace damageable roads with noncombat roads; guaranteed captured-destination arrival with the exact stalled/cancelable freight contract; occupation all-operation freeze with zero worker assignments and surviving passive solar. Match ObjectiveId on effects, crew:u8 and qualified grant producer; adopt command17 cancel_shipment, one effect outcome emitter, framed event tags/payloads and phase registry. Remove superseded duplicate transcript subtotal. |
| Tactical | Use proposed city-combat-galaxy.v1 and city-combat-storage.v1 or return one jointly selected alternative before dispatch; phase3..12 round barriers and city tiers including training after construction; nonroad-only structural targets; ObjectiveId/current revision/target tick effect binding; successful-city-only objective completion; training crew:u8/paired reservation; no duplicate city outcomes. Supply exact family1 command/order and kinds7/8 unit/objective codecs, ratify kind9 sixteen-score theater, event64/65/66 payloads and128byte theater bound. Resolve separately owned hostility producer. |
| Authority/codec | Register the new52-entry quota/profile and262144/65536/12746752 genesis/initial/tick bounds; keep initial-economy codec unchanged. Prove83,216 index rows and95,504 pages/root = initial74,512/86,800 +2,048 training +6,656 tactical, with35,567 pages headroom. No duplicate roads or training reservation pages. Prove city256byte records/ship route children, shared8MiB scratch plus tactical8MiB, index/transition controls, old/new roots and source+target against the current provider/fix revision. |
| Session | Latest32MiB logical transaction and1MiB transport segment proposal is arithmetically sufficient; independently review and register the exact new G/history codec and262144byte genesis,65536byte initial stream,12746752byte tick reservation. Prove authenticated retained prefixes,32MiB spanning/partial-tail behavior, source/target buffering, cancellation,GC,save/reopen and exact selected Continue. No previous smaller-node conflict is asserted against this new text. |
| Plan/qualification | Freeze checked-in independent capital/codecs/full gameplay schedule (including care/recruitment and sustained satellite food freight), maximum record/emit-site witnesses and allocation/cancellation cuts. The newly shown reduced hour tables are not the full journey or an executed runtime. |

Proposed memory arithmetic: adding tactical8MiB to the current188,809,216-byte
full-scale reservation yields197,197,824bytes/authority,394,395,648 for source+
replay target,8,257,536bytes below384MiB. This assumes city scratch fits existing
8MiB and all added records fit the pool; the missing compiled controls/record
witness may disprove it. Never spend that margin silently or call it resident
memory measurement. Reserved future strategic families remain future schemas.

Review issue10 remains an explicit feasibility gate: named hardware,10,000 full
loaded-tick samples, p95<=5ms at the declared caps, with request copy,simulation,
validation,encoding/hash and retirement separately reported. Poll p95<=2ms/max<=8ms
at256units is a different measurement. City component traversal once/world avoids
a repeated full-world scan per city, but does not establish total tick speed;
tactical BFS/index movement need their owners' algorithm and hardware evidence.
Do not reduce caps, skip worlds, publish partial ticks or relabel poll timing to
close it. Issue9's latest session revision needs fresh durability/retention review;
32MiB arithmetic is not acceptance. Complete100,000-tick retained workload and
maximum-load byte stress are separately measured fixtures.

Current artifact result is Proposed design only. Open integration proofs are
explicit prerequisites above; they are not transferred to runtime as permissive
fallbacks. No source, instruction file, user profile, production dependency,
commit, push or deployment is changed by this document.
