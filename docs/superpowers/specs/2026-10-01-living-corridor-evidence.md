# Living corridor observations and controlled comparison

Status: **Partial** interface slice for the accepted optional experiment
“Disrupt a corridor and identify affected industries.” A non-serialized readonly
witness type, pending route-attempt producer, authority receipt publication,
additive Notebook locator admission and bounded pure observation join exist.
Focused locator qualification passed four core integration tests. Individual
record controls have parent-reported host UI coverage (two tests, exit 0).
The joined replay cache passed seven focused integration tests and three pure
join tests; independent review is pending. Paired UI and full workflow completion remain
**Proposed**. Generic events cannot publish a result.

## Current evidence boundary

Interrupted Corridor schedules an ion storm on its declared lane from tick 10
through tick 150, end exclusive. Its route attempts a batch of two every ten
ticks, starting at tick 10; receiving worlds start without Ore and their local
extractors are removed. The accepted fixture requires reduced or zero dispatch
and downstream production wait by tick 200.

Authority currently emits bare HazardStarted/HazardEnded and successful
ShipmentDispatched events. A zero dispatch emits no ordinary event. Actual
FacilityBlocked explanations contain the facility entity and exact MissingInput
resource, required amount and available amount. These events establish a storm
and a local wait, but do not establish that a particular route was restricted.
Production precedes dispatch within a step: a shortage at the dispatch tick cannot
be attributed to that tick's newly restricted shipment.

## Proposed readonly attempt interface

Like creator recovery evidence, regenerate private-constructor, receipt-bound
attempt witnesses during ordinary execution and replay, without altering legacy
ordinary events or canonical bytes. Retain only attempts with positive eligible
batch before the hazard adjustment and strictly smaller batch afterward. Each
witness needs exact route ID, source/destination worlds and systems, sender and
receiver, resource, requested batch, reserve-limited pre-hazard batch, post-hazard
batch, actual accepted dispatched units, relevant active hazard IDs and their lane/interval/definition, and receipt
digest/tick. Preserve the distinction between a hazard restriction and unrelated
reserve shortage, absent travel, suspension, diplomacy, shipment capacity or
ownership mismatch. Admission must reflect the actual permitted dispatch path;
a route skipped before hazard evaluation supplies no restriction proof. For the
initial observation interface, actual dispatched units must equal the restricted
batch; zero/zero qualifies a restriction to zero, while a positive restricted
batch followed by capacity-blocked zero dispatch does not qualify.

`LivingCorridorRestrictionV2` supplies readonly getters including
`actual_dispatched_units()` and `hazard()`. The rules producer records a pending
candidate only after the actual dispatch path, including its access/ownership,
source reserve and capacity checks. A positive candidate follows successful
spend/acceptance; a zero candidate follows positive eligibility reduced to zero.
Forecast-only contexts skip candidate retention. Pending candidates are capped
at 4096 with explicit failure, not silent evidence loss. Successful authority
receipt binding and publication are implemented by the authority owner and were
reviewed separately from the standalone producer.
The initial compact witness uses one effective minimizing hazard, chosen stably
by ID, rather than retaining an unbounded list. The readonly Notebook observation
operands have no public constructors. `LivingNotebookCorridorObservationJoinV2`
bounds each input to 4096 witnesses, advances at most one comparison per call and
retains a stable first matching restriction per wait. It does not independently
authenticate producer facts or certify the full experiment.

This interface is private evidence, not a gameplay command or a persisted proof.
It requires independent review before producer work. In particular, the ordinary
event list must not gain a fabricated ShipmentDispatched event for zero units.

## Proposed linked observation

### Additive Notebook locator shape

The approved record addition is an optional `route_attempts` array after `events`.
Each locator has required fields in order `{ "route": <entity hex>, "hazard":
<entity hex> }`. Absence omits the key and preserves all historical bytes. Present
empty arrays are rejected; locators sort strictly by route then hazard, and the
combined ordinary-event plus attempt-reference count may not exceed 32. Only
CorridorImpact records may carry locators. Such records may have no event
references when they contain at least one real attempt locator, including a
genuinely empty ordinary receipt at a restriction-to-zero boundary.

Replay authenticates the exact bookmarked state and ordinary receipt digest, then
resolves each route/effective-hazard pair against the authority's regenerated
receipt-bound restrictions. It stores no claimed amount or completion bit. Generic
HazardStarted event records retain their historical decoding/encoding shape but
cannot qualify as Corridor evidence: they lack route restriction proof. Existing
legacy Notebook goldens remain unchanged. The new independent golden is
`notebook-corridor-attempts.json`, calculated from the ordered Python payload and
existing Notebook SHA-256 domain by the independent additions vector script.

At t1, replay must establish an actual permitted due route with positive eligible
batch restricted by the declared active corridor hazard. At t2 > t1, replay must
establish autonomous FacilityBlocked with MissingInput on that route's exact
destination world, matching resource and recipient ownership. The industry must
actually consume that resource according to the attached catalog. Both references
must belong to the same validated retained branch and nondecreasing actual creator
revision prefixes. Names, present inventories, coincident quantities and generic
HazardStarted events cannot supply missing route identity.

The approved additive locator authenticates the actual attempt boundary without
inventing an ordinary dispatch event. Later shortage evidence remains strictly
later than the restricted attempt. A generic storm event cannot replace a missing
locator or a missing downstream industry receipt.

The visible result is **“Corridor restriction and matching downstream input wait
observed.”** Show route, hazard/lane, endpoint worlds, resource, both batch amounts,
industry, shortfall amounts, branch and exact bookmarks. This pair alone does not
complete the full Corridor experiment or prove the hazard solely caused the wait.

Replay retains compact witnesses rather than extra complete states. Bound attempts
by the existing route capacity per receipt and retained joins by evidence-reference
capacity. Charge each candidate comparison to one replay work unit; select a stable
first matching restriction per later wait. Cancellation and validation failure
discard unpublished results. A removed supporting record removes the derived pair.

## Proposed controlled sibling comparison

A stronger optional workflow can compare exact matching-tick sibling futures from
one validated pre-intervention boundary. Require identical embedded genesis,
catalog, common immutable history prefix, state and pending queue at the fork.
The intervention arm may differ by exactly one creator operation cancelling the
specific future or active hazard. Require no other changed creator operations or
pending commands in either arm through the comparison tick. Verify actual revision
operation payloads and retained fork anchors, rather than labels or merely equal
ticks. Autonomous consequences after that single intervention may diverge.

At one common later tick, replay both arms and show the exact affected world's
resource measurement already supported by Notebook comparison, plus actual route
restriction and industry-wait witnesses from the restricted arm. Recovery in the
cancelled arm requires actual matching FacilityProduced input facts, not a larger
inventory alone. If either arm lacks those witnesses, preserve the comparison as
an inconclusive experiment. This is a deterministic controlled intervention result
within the simulated histories, not a claim about all possible gameplay states.

The existing experiment wire has matching-tick stockpile comparisons but no typed
link to Corridor evidence; this proposed workflow still needs a reviewed readonly
derivation interface before a completion label can be published. It does not add
a mandatory baseline requirement to the lighter observation pair.

## Required negative coverage and qualification

Reject zero pre-hazard availability, no effective reduction, same-system routes,
wrong lane, inactive interval, skipped/ineligible dispatch, unknown or omitted
facts, wrong endpoint/actor/resource/industry recipe, Disabled or MissingDeposit
without matching MissingInput, t2 <= t1, reverse revision prefixes, sibling joins,
and stale receipt bindings. Cover two simultaneous hazards and stable selection,
end-exclusive timing, zero and partial restrictions, removal/cancellation,
budget-one replay, support deletion, save/Continue and import revalidation.

For the stronger comparison, reject differing genesis/catalog/queues, unrelated
creator edits, more than one changed operation, renamed sibling labels with
unrelated prefixes, wrong hazard cancellation and unequal comparison ticks.
Actual fixture receipt tests, visible host journeys, live native/browser journeys,
IndexedDB persistence and named-device performance remain separate evidence.

Current focused qualification: three pure unit tests pass. They cover later
matching waits, zero/partial restrictions, stable first matching joins, empty
inputs, over-capacity rejection, receipt mismatch, same-tick/reversed-prefix/
sibling rejection, mismatched destination/actor/resource/system, expired hazard,
and a positive restriction without accepted dispatch. These constructed operand
tests do not qualify real producer evidence, catalog admission or the fixture.

Focused locator qualification additionally passed **four integration tests, exit 0**,
in `/tmp/nyon-corridor-locator-tests2.log`: genuine empty ordinary receipts,
malformed/stale or mismatched references, generic-hazard rejection with actual
later MissingInput admission, and independent present/absent wire fixtures.
Strict core test-target lint passed with exit 0. The first fixture execution
failed because its source reserve was invalid; the corrected validated reserve
passed. These checks do not establish joined replay observations or live UI
qualification. Independent staged authority coverage passed 18 tests, exit 0.

## Replay-derived observation cache integration

`ValidatedLivingNotebookV2::corridor_observations()` exposes immutable
`LivingNotebookCorridorObservationPairV2` values with private constructors and
no serialization. Each contains exact attempt record/bookmark/view/route-hazard
locator, the authenticated restriction, exact wait record/bookmark/view/event,
facility, world, system, actor, resource, required quantity and available quantity.
The attempt anchor does not manufacture an ordinary event for a zero-dispatch
receipt. Existing Notebook bytes remain the stored evidence; pairs are regenerated
by replay.

The Corridor record stage retains its one replay authority while authenticating
locator candidates and extracting waits. It advances one locator/witness
comparison or one wait extraction per work unit. The existing whole-vector
locator search must be replaced on this path; merely adding a bounded join after
that search would leave unbounded new admission work. Exact receipt/state
binding and sidecar verification still precede admitted operands. Once the task
is admitted, no additional full replay state is retained for its evidence.

Every catalog-confirmed MissingInput reason is collected; the three-resource
enum bounds waits at 3 × 128 × 32, while attempt operands remain bounded at
128 × 32. This includes simultaneous shortages when the corridor resource is
not the first reason.

The join compares one compact candidate per work unit, retaining the first
matching restriction for each wait in stable evidence order. Each resulting
proof is materialized in a separate counted operation. Temporary pair indices
and operands retire one element per counted operation after proof construction.
Cancellation and sticky failure must never publish a partial validated cache.
The inherited authority destruction, Trade/Supply collection and cancellation
clears retain their existing work boundaries; this slice does not qualify the
entire Notebook executor as bounded.

Both Session validated-install and locally verified-accept paths will copy the
cache only after their exact retained-history checks. Continue/import use the
validated-install path; current recording does not create a replay-verified pair.
Presentation will say “Corridor restriction and matching downstream input wait
observed” and expose both exact bookmarks. It will not claim sole causation or
completion of the complete Corridor experiment.

Focused tests must cover real zero-dispatch attempt followed by later matching
MissingInput, positive reduced dispatch, only one operand, same-tick wait,
reversed creator prefix, sibling branch, changed actor/world/system/resource,
stale receipt, expired hazard and unmatched accepted-dispatch quantity. Budget
one and larger budgets must produce identical proofs; cancellation/sticky
failure/NotReady must keep them unavailable. Independent existing wire fixtures
must stay byte-identical.

Current cache qualification: **seven focused integration tests, actual exit 0**,
in `/tmp/nyon-corridor-cache-integration3.log`, and **three pure join tests**
in `/tmp/nyon-corridor-cache-pure2.log`. They cover actual replay-derived
zero-dispatch/later-input-wait pairs, identical proof results at budgets
1/2/17/1024, isolated observations, cancellation/NotReady, malformed locator
binding, and old/present independent wire fixtures. A real tick19 running
inventory intervention leaves both Energy and Ore below the foundry recipe
requirements at tick20; the Ore wait remains paired although Energy is its
first blocked reason. This intervention is recorded in the actual creator
prefix and does not establish a causal Corridor outcome.

The first cache run failed two test fixtures: budget4096 exceeded the accepted
1024 maximum, and destination hub production prevented the intended Energy
shortage. An attempted deferred hub clock was correctly rejected by genesis
clock validation in the second run. The final test uses ordinary supported
creator intervention and preserves the fixed genesis clocks. No production
defect was observed in those failing runs. Live UI and full workflow
qualification remain separate.
