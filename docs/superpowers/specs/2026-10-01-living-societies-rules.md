# Living V3 society rules

Date: 2026-10-03. Status: **PROPOSED coordinated review submission2; independent integrated review, fresh complete vectors and human approval pending**.
Parent: `2026-10-01-living-expansion-architecture.md`.
These values are proposed balance choices, not implemented or accepted V2 rules.

## State and limits

Each colony stores population, specialization, its next needs boundary, latest
demographic outcome and workforce assignments. Food belongs to the world
inventory; needs, recipes and freight all debit that single canonical account. Shared planning,
central reservations and guarded due spending follow scoring §§1–8 and the
complete-schema §§Integrated planning/admission and scoring payloads.
Habitation capacity is derived from completed owned facilities and ecology, not
stored as a second independently editable total. Population is an aggregate integer count, 0..=10000; no individual
person entities exist. Food storage is 0..=10000, alongside local energy/ore/alloy.
Population and food are canonical state, never inferred from presentation.

Each civilization stores one optional research target, progress by technology
and an ordered set of completed catalog technologies. One actor can research one
technology at a time. Colony specialization is Balanced, Agrarian, Industrial or
Research. These choices are policy priorities, not free yield bonuses.

Retain 512 worlds and 16 civilizations as initial V3 topology limits. Use eight
industrial slots per colony, counting completed facilities and paid construction reservations;
global facilities remain capped at 2048. Changing limits requires versioned rules.

Ownership transfer preserves local population/food/infrastructure and clears the
old owner's assignments. Research belongs to the civilization and is not captured
with a colony. Removal disposes population explicitly in the creator cascade;
it cannot silently teleport to another world. Population zero is a nonterminal
colony state and does not by itself erase a civilization's ownership.

## Needs, growth and workforce

RecipeFacts records actual positive debits/outputs, with the sole authenticated Farm Food0 exception in complete-schema §ResourceAmount domains and empty-list meanings. For a due applicable Farm: checked clock, ordinary disabled branch, committed worker, then environment; q0 reports only environmental_limit with Food0 and no funding/storage/needs read. Positive q collects complete physical blockers before the due-needs guard. No lab/research point is a material ResourceAmount output. Strict recovery preserves every baseline funded imminent Food/energy payment at all own colonies under the conditional B/F operator, and central same-actor overlay rechecks prior costs. Optional bootstrap leaves last_needs=null even population0, status Active iff colony exists and first needs/environment clocks100; it runs no Needs payment or actor action.


Needs are due every100 ticks. Before phase5 discretionary input debits on a due
boundary, hold actual local Food/energy for the due phase6 population. Guarded
stock is held, not absent: non-life-support recipes/labs/repair/fleet supply may
spend only unheld stock. A held-energy Food recipe may execute only when its
atomic result strictly lowers combined imminent exposure and neither previously
fully funded resource becomes short. Report NeedsReserved separately from actual
MissingInput; denial consumes no input and obeys existing blocked-clock rules.
Complete-schema §Integrated planning and admission defines entry/recomputation/
release and exact guard facts. The holds do not change need quantities or clocks.
At phase6 use population at phase entry:
Food need = ceil(population/100); energy need = ceil(population/200). Zero
population needs zero. Debit each resource by min(available, need), record the
two actual shortages and advance the clock even when blocked. No negative
inventory, cross-world invisible payment or delayed catch-up consumption exists.

After needs, carrying capacity is min(10000, 1000 + 1000 * completed habitation
facilities), adjusted by the ecology specification. When both needs were fully
paid and population is below effective capacity, compute proposed growth as
min(capacity-population, max(1, floor(population/100))). Apply it only when
remaining Food >= ceil((population + proposed growth)/100), the next-boundary
need of the resulting population. Otherwise remain steady unless another decline
cause applies. Population zero does not grow spontaneously.
Otherwise, an unmet need or population above capacity reduces population by
min(population, max(1, ceil(population/100))). Population at capacity with paid
needs remains unchanged. Record growth/decline/steady with the actual causes.

One worker unit is available per 100 population at the needs boundary. Each
enabled industrial facility requires one unit to operate; completed habitation
does not. Hub baseline remains unstaffed so a declining colony can recover.
Assign workers after demographics. Existing paid construction/repair is not
cancelled solely by a workforce shortage. An unstaffed recipe advances its clock
and reports Workforce unavailable without consuming inputs.

Assignment priority groups:

| Specialization | First | Second | Third | Last |
| --- | --- | --- | --- | --- |
| Balanced | Food/power | Extraction/processing | Research | Military industry |
| Agrarian | Food | Power | Extraction/processing | Research/military |
| Industrial | Power | Extraction/processing | Food | Research/military |
| Research | Food/power | Research | Extraction/processing | Military industry |

Within a group use facility ID order. Defense and habitation are passive;
shipyard counts as military industry. Jobs complete independently of assignment,
but an unstaffed shipyard cannot accept a new autonomous hull job. Creator
overrides may set specialization/population/food but do not bypass structural
capacity; their preview includes changed assignments and needs.

## Added catalog definitions

| Facility | Alloy cost | Build ticks | Operation |
| --- | ---: | ---: | --- |
| Habitation | 30 | 150 | Adds 1000 base carrying capacity; passive |
| Farm | 30 | 150 | Every 10 ticks, 2 energy produces 20 Food; one worker |
| Research lab | 60 | 300 | Every 20 ticks, 2 energy + 1 alloy produces 1 research point; one worker |

Recipes are atomic: all required inputs/output room/assignment must exist or
consume nothing. First operation is completion plus period. Farms require a
habitable owned colony. Ecology adjusts renewable Food output using integer
basis points; exact multiplication and flooring are in its specification.
Food freight follows the same explicit local route, batch, reserve, transit and
ownership permission model as other resources. Accepted shipment quantities are
immutable while travelling.

## Research

The initial finite catalog graph is:

| Technology | Prerequisite | Points | Effect |
| --- | --- | ---: | --- |
| Agronomy | None | 100 | Farm base Food output becomes 25 instead of 20 |
| Resilient habitation | Agronomy | 150 | Ecology §World environment and conservation Formula A: Cbase-floor((Cbase-Cnormal)/2); capacity-only penalty reduction, base habitability remains the capacity limit; e and Farm Food unchanged |
| Efficient logistics | None | 100 | Actor-funded departure cost multiplied by3/4 PER HULL, ceiling/minimum1 then checked sum; already paid motion unchanged |

Research has no effect before completion. For labs due on one boundary, process
stable ID order. A lab pays only if its owner has an eligible selected target, progress is below
cost, staff/inputs/output conditions hold and the due-needs guard permits its
local debit. Physically present held energy cannot be spent by the lab. On reaching cost, emit completion, mark the technology
complete and clear the target; later labs that boundary do not pay for an absent
target. Do not credit excess points to a new target automatically.

Changing targets preserves progress in a per-technology ordered record rather
than refunding inputs. Creator grant/revoke/target changes are explicit history
operations. Revocation names downstream completed technologies and target
dependencies; an incomplete cascade rejects atomically. Existing travelling
fleet payments are not recalculated after a technology change.

Autonomous research target selection runs at the actor decision boundary from
its permitted snapshot: select an incomplete eligible technology by policy
priority, then slug. Trader prioritizes Efficient logistics; Guardian prioritizes
Resilient habitation through Agronomy; Neutral/Expansionist prioritize Agronomy,
then Efficient logistics, then Resilient habitation. Industrialist uses the same
Neutral order. Scoring §4 specifies every eligible target priority/score exactly. Material utility
uses tagged economy components; every raw numeric field has a fixed PolicyScore
reason order. Eligible-only trace excludes unfunded/unstaffed/unformed alternatives;
central scarcity denial is recorded separately under schema correction I4.
Selection reserves no phantom
resources; actual labs pay locally when due.

## Autonomous economy and settlement

Add explicit candidates for Food production, habitation, staffing-support power
and research. Needs recovery is a hard priority above discretionary
growth/research/military spending; policy ranks candidates within that priority
and cannot trade imminent Food/energy recovery for discretionary goals. This
priority was selected by the user on2026-10-02. Exact Proposed within-priority
utility, policy bonuses, forecast horizons, imminent<=100 category predicate,
obligation tier and deterministic tie/merge order are scoring §§1–8. Reduce an
actual imminent owned needs outcome to qualify category0; later-only benefit is
category1. The separately declared public Food/energy request opportunity exception
never counts promised freight as exposure saved. Phase8/9 share one post-phase7
permitted capture. Forecast includes owned local state and actual known immutable
incoming manifests, not unmanifested contracted promises or private rival stock.
Central guards/paid accounts and candidate costs use the same exact terms.
Candidate identity is full canonical action plus AdmissionTerms; batch/fleet/
force alternatives are distinct. Category0 precedes all discretionary spending,
then recovery, obligation and ordinary follow the exact shared ledger ordering.

Every Hull has required Passengers{count:u32,voyage_origin:Option<WorldID>}.
Non-Ark requires count0/origin null; count0 clears origin. Per-Ark physical maximum
is explicitly u32::MAX, a NEW Proposed capacity recommendation, with checked
addition/transfer and separate Colony population/effective capacity. There is no
invented narrower passenger-full denial or Catalog capacity field. A reached
Board addition overflow is a whole unpublished Arithmetic fault, not omission or
capacity Decision; earlier actual guard precedence remains intact.

Autonomous Board transfers exactly100 from actual owned origin Colony population
at least200 to exact Ark, with immediate workforce recomputation. Preserve origin
Some or null; boarding/refuel/order/split/merge never substitutes present World for
voyage origin. A first genuine positive-passenger NONRETURN departure with null
origin installs actual departure World; Return never creates it. Some persists
through later motion/reorganization/ownership changes until count0 or explicit
creator cohort reset/deletion. Creator CreateHull uses count-only input and origin
null; SetPassengers override starts the submitted count with origin null, never
claims actor-paid provenance. World deletion clears matching origin while preserving
people under exact reviewed closure, not nearest-origin substitution.

Emit intent_decided then passengers_boarded then one operation_changed containing
complete Fleet AND Colony Changes with
passenger_delta+100/−100/resources[], workforce_changed iff assignments changed.
These describe ONE transfer, never duplicate debit. Genuine phase3 pre-arrival
Return unloads only Hulls whose original origin equals endpoint and current Fleet
owner equals live matching Colony owner. Transfer min(count, max(effectiveCapacity−
population,0)) in Hull-ID order; zero emits nothing/no stationary retry. Positive
emits passengers_unloaded owning the population transfer, then one
operation_changed with complete containing Fleet Change only (delta−unloaded
count/resources[]), then workforce_changed iff assignments changed. No duplicate
Colony Change repeats that transfer;
remaining people stay aboard with their original origin. Count reaching0 clears it.
No Food/Energy/Needs clock, demographic growth or new credit occurs on transfer.

Founding consumes only lowest globally eligible Hull ID among winning civilization's
actual closing-member Arks; Fleet ID is not first tie. Transfer its exact count P
to new Colony once, removing containing Fleet iff last Hull. P must fit the existing Colony
population domain and structural admission predicates; never truncate people. Other Arks retain counts/credits.
Primary settlement_claim_resolved has full conserved Fleet−P/Colony+P Changes;
passengers_unloaded iffP>0 has absent prior population=null, newSome(P), not fake0.
No hull_destroyed/passengers_lost on settlement. Genuine combat/creator destruction
records passenger loss once; captured Hull ownership changes faction with people,
never resets their original voyage proof. Last-Hull creator removal uses reviewed
containing-Fleet destruction/job occupancy closure, not an empty Fleet or combat
half-refund convention. All exact source/capture/Changes are retained in history.

## Creator, explanation and evidence

Add typed population/Food/specialization/research edits, habitation/farm/lab
creation and passenger boarding/disposition operations. Preview shows exact
dependencies, workforce, needs, capacity and downstream technology cascades.
Actor-paid operations and free creator changes remain separately receipted.

Expose needs paid/shortfall, growth cause, workforce assignment, operation state,
research target/progress/funding and completed technologies in visible/semantic
inspectors. No green Enabled badge substitutes for actual operating conditions.

Tests must cover population 0/1/99/100/199/200/10000; Food/energy shortages
independently and together; exact capacity; disabled/unstaffed/overflow recipes;
simultaneous labs completing one target; target switching; technology cascades;
boarding/return/capture/destruction conservation; scarcity reservations; hidden
rival inventory; poll budgets; undo/replay/save/transfer. Independent wire vectors
must include the new fields/events before this Proposed contract becomes code.


For known freight recovery forecasts use only timestamped observed lane/stellar
interval/factor records in the permitted payload, never hidden root hazards or
unannounced schedule changes. Actor-only trace digest excludes hidden authority
State/revision provenance while the full outer audit remains replay-authenticated.
Required fixtures include simultaneous staff/funds failure (no trace record),
absent path/unobserved target (no record), every viable full-slot scrap replacement,
and central conflict denial with unchanged trace count/hash.


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
