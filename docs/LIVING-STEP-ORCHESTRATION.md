# Living boundary orchestration implementation plan

Status: **Proposed implementation**, 2026-10-01. This completes the approved
responsiveness and atomicity contracts; it introduces no new gameplay, wire
shapes, identities or numerical rules. Existing phase cursors are Current as
individual components. A bounded complete boundary and its resident integration
remain Partial.

## 2026-10-02 resident integration checkpoint

Resident `SandboxSession` now owns one private staged step and at most twenty
outstanding boundary requests. Run preserves fractional integer timing and drops
excess whole requests; interactive Step requests exactly one boundary, completing
through subsequent updates. Each update polls once with a shared 1024-record
budget and completes at most one explicit synchronous suffix. The old
`step_once` and runtime `step_sandbox` APIs remain synchronous compatibility
helpers, but drain the staged executor and publish through its capture fence.

Pause, suspension changes, modal holds, history installation, replacement and
leave cancel prospective work and clear due requests. Successful command
admission invalidates the capture; resident polling cancels/restarts with the
full accepted queue. Publication alone changes Chronicle, decisions and dirty
state. A processing status keeps the committed tick visible, and Step is disabled
while its request is pending. Effective speed uses actual publications rather
than the number of requested ticks.

This integration is **Partial for whole-frame responsiveness**. Due intents,
delegated creator operations, validation, receipt/journal sealing, publication
cache projection and retired-owner destruction still have synchronous work.
These are not counted as record visits or qualified by the poll budget. Due
intents retain their existing position before dispatch. The earlier instruction
to leave the staged executor off the resident path is superseded by the explicit
2026-10-02 request to integrate this staged job; the remaining bounded-executor
work and performance acceptance remain open.

Regression coverage compares state, all three receipt surfaces and exact archive
bytes through tick 55 at budgets 1/64/1024. It also covers newly admitted pending
tails, pause/hold/suspension/replacement cancellation, repeated Step requests,
twenty-request saturation and update visit accounting. See
`LIVING-RESIDENT-EVIDENCE-2026-10-02.md` for gate results and proof limits.

## 2026-10-01 scheduled forecast checkpoint

Private input preparation and the complete 200-boundary scheduled forecast now
have shared-budget cursors, including nested phases, route searches and metadata
retirement. Thirteen focused forecast regressions pass at the current checkpoint;
budget 1/64/1024 outputs match the retained reference. See
`LIVING-FORECAST-EVIDENCE-2026-10-01.md` for source identity and current gate status.
Compatibility callers still drain these jobs synchronously. Intent proposal and
reservation, resident transport, full sealing/history and retirement remain
Partial; this does not change the due-intent staged-executor marker yet.

## 2026-10-01 dispatch integration checkpoint

The staged executor now polls the existing owned dispatch cursor on boundaries
without due intents, sharing the same 1..=1024 visit budget with earlier phases.
An explicit private `DispatchComplete` marker prevents the transitional sealing
path from repeating debits, manifests or events. Every 50th boundary still stops
after diplomacy: due intents must precede dispatch, and that remaining path is
explicitly synchronous. Resident Session still uses the original step path.

The new pre-seal state regression failed before integration and passes afterward.
Fresh `cargo test -p nyon-workshop-core --lib staged_step_` exited 0: 20 passed,
zero failed/ignored. It covers full reference state/receipt parity, shared-budget
visit totals, cancellation/NotReady/stale-tail behavior during dispatch and the
existing 55-boundary cadence cases. Strict workspace all-targets/all-features
Clippy, independent locked fuzz Clippy and formatting, and vertex-layout checks
also exited 0. Updated workspace tests passed 1,353 tests across 101 suites;
release, paired browser builds, wasm checks/lint and artifact checks exited 0.
See `LIVING-DISPATCH-EVIDENCE-2026-10-01.md` for identities and runtime limits. This is
an intermediate executor slice, not complete boundary or responsiveness evidence.

## Implemented partial preparation interface

The first staged slice now exists in `living/simulation.rs`: `begin_step`,
`LivingStepJobV2::poll`, `finish_synchronously`, `commit_step` and
`commit_step_failure`. Poll composes private state copying, source hashing, a partial queued
creator cursor, phase-two lifecycle expiry and phase-three arrivals under a
shared 1..=1024 actual-record budget. The creator cursor
slices per-operation declarations, recovery baseline scans, six core creation
kinds, three scalar edits, sorted insertion source/output records and metadata
retirement. Delegated creator operations explicitly return
`NeedsSynchronousCreatorOperation` before executing their remaining cascade or
other work. That revision resumes from its unapplied suffix with original local
IDs only through the named synchronous completion method. A fully supported
queue passes its candidate and accumulated events into the lifecycle/arrival
composition. Later composed cursors now cover conflict, production, observations and diplomacy.
On non-intent boundaries dispatch also completes privately.
`NeedsSynchronousCompletion` still marks due intents, candidate validation,
receipt sealing and journal preparation as synchronous. Continuation carries
explicit completed-phase markers, including empty phases, and never repeats
their effects.
The delegated creator fallback may drain scheduled phases explicitly inside
its synchronous completion method. Resident transport continues using the original step path.
This does not establish a bounded full tick.

Successful and failed preparation tokens are boxed before commit and have
private fields/constructors. Commit moves prepared owners and returns an opaque
retired-publication owner. Stale commit rejection also retains the boxed token
inside its typed rejection so reclamation does not traverse a discarded
candidate during the publication operation. Caller drop/reclamation remains
synchronous and must stay outside that operation; incremental retirement is a
remaining implementation task.

## Current execution and invariant

`LivingGalaxyAuthorityV2::step` in `living/simulation.rs` still synchronously
executes `run_boundary_candidate`, validates, hashes and seals all three receipt
surfaces, then publishes state, queue consumption and history together. Its
candidate is private and deterministic failures preserve the preceding state
and accepted queue while latching a fault. That invariant must survive the
change.

`LivingBoundaryCaptureV2` already retains immutable state, catalog, accepted
queue, revisions and modes through Arc ownership, along with branch, committed
revision and allocator high-water marks. `boundary_capture_is_current` checks
exact ownership and marks. Rejected operations leave the token current;
successful queue admission, edits, stepping and history navigation invalidate
it. This is the publication fence; a worker's generation alone is insufficient.

Current resumable components include private state copying and state hashing,
phases 2/3, phase 5 production/repair and phase 6 observations. The phase wrappers
still drain all polls synchronously. The observation cursor slices scans but
still revisits entity collections per observer/world; it is not an indexing
performance optimization.

## Interface and ownership

Implement the executor inside core `living/simulation.rs` / `living/step_job.rs`,
where it can inspect the capture's private accepted records without publishing
a public command/state bypass. Root owns Session and Runtime integration.
Suggested additive signatures, finalized against the existing error vocabulary:

```rust
impl LivingGalaxyAuthorityV2 {
    pub fn begin_step(&self) -> Result<LivingStepJobV2, LivingStepJobErrorV2>;
    pub fn commit_step(
        &mut self,
        prepared: Box<LivingPreparedBoundaryV2>,
    ) -> Result<LivingRetiredBoundaryV2, LivingStepCommitRejectionV2>;
    pub fn commit_step_failure(
        &mut self,
        failure: Box<LivingPreparedStepFailureV2>,
    ) -> Result<LivingRetiredBoundaryV2, LivingStepCommitRejectionV2>;
}
impl LivingStepJobV2 {
    pub fn poll(&mut self, visits: usize)
        -> Result<LivingStepProgressV2, LivingStepJobErrorV2>;
    pub fn cancel(&mut self);
    pub fn last_poll_visits(&self) -> usize;
    pub fn total_visits(&self) -> u64;
    pub fn finish_synchronously(self, authority: &LivingGalaxyAuthorityV2)
        -> Result<LivingStepPreparationV2, LivingStepJobErrorV2>;
}
```

The prepared boundary and failure token constructors and fields remain private.
Only executor failure can construct the latter; callers cannot inject an
arbitrary deterministic authority fault. The successful result contains the
original capture, a precreated Arc candidate owner, precreated receipt owners,
exact ordinary/decision/event-facts receipts, prepared journal/mode owners and
the empty replacement queue. It is
not serializable and cannot be assembled from public arbitrary state. A failed,
cancelled or incomplete executor cannot create it. `commit_step` performs the
freshness check before any assignment. It performs no fallible validation,
serialization, traversal, allocation or history extension after the check;
those are prepared in polls. It moves prepared owners into the authority and
makes the already-sealed receipt available through the authority accessor. Retired owner destruction is also work:
return an opaque retired-publication object from commit and reclaim it outside
the publication fence using a separately bounded retirement queue. Do not
silently drop the last Arc of a capacity-sized state or history inside commit.
The exact compatibility return shape must accommodate this ownership before
implementation.

Preserve `step()` as a compatibility adapter that drains this same executor and
uses the same commit path; keep the old synchronous implementation as a test
reference until state, every receipt, IDs and fault boundaries match. Do not
allow a second independent production algorithm to diverge.

## Ordered candidate stages

One job owns one unpublished candidate and one cumulative `LivingPendingEventsV2`.
Move them between cursors rather than cloning the entire candidate at phase
transitions. Arrivals also return the ordered arrival-world list required by
conflict and settlement. Seed, branch and prospective tick remain immutable.

1. Capture in constant time. Preserve MAX-tick arithmetic fault semantics:
   existing capture_boundary rejects checked next-tick overflow before a job
   exists, whereas step latches that arithmetic fault. Add an internal unchecked
   publication capture for executor construction; its checked next-tick stage
   creates an opaque failure token. Public admission/history refusal remains
   non-latching. Do not expose a general fault-token constructor.
2. Copy source state with the existing nested-record copy cursor. Hash the
   immutable source separately with the streaming cursor for the decision
   snapshot reference; share the same poll budget.
3. Apply captured running revisions in accepted sequence order. Emit each
   CreatorIntervention and effect in exactly the current order. Do not sort
   revisions before execution; sorting applied revision IDs is only receipt
   encoding. Stage each operation and its cascade/resource effects through a
   command cursor. One 1024-operation revision cannot count as one visit.
4. Expire lifecycles and resolve arrivals through the existing shared-budget
   composition. Preserve prepaid credit, relation credit and receipt order.
5. Conflict: automatic noncombatant retreat, then due combat, occupation and
   claims. Snapshot combat targets before rounds; retain immediate retreat's
   specified exception. Each fleet/hull/world/path search is separately visited.
6. Production with `LivingProductionJobV2::begin_owned`; move back state/events.
7. Observation refresh with `LivingObservationJobV2`; retain cumulative events
   outside that cursor, which does not emit events.
8. Due diplomacy: exact shared updated snapshot, decay, directed relation and
   bilateral proposals, reservations and effects. Slice snapshot copying,
   connectivity searches and disclosure hashing.
9. Due intents: exact shared snapshot, actor forecasts, candidate generation,
   scoring/ranking and reservation. The 200-boundary forecast must itself run
   scheduled phase children with the remaining budget; one forecast boundary
   is not one visit. Maintain the existing private-information restrictions.
10. Dispatch standing fleet orders and freight. Slice lanes/path selection,
    capacity reservations, manifest creation and resource debits. Preserve
    source/destination ownership and foreign return-credit rules.
11. Set private tick; incrementally validate all collections, foreign keys,
    capacities, nested hulls/observations, references and ordering. Hash candidate
    with the existing streaming state cursor.
12. Incrementally encode/seal ordinary, event facts and decisions. Account for
    resource/fact rows and command/receipt bytes, not just outer event count.
    Prebuild applied-ID ordering and replacement revision/mode journal owners.
13. Ready; publish through `commit_step` only if capture is still current.

Every child receives `remaining = requested - consumed`, and actual child
visits are charged before advancing stages. Empty transitions may cost no
visits but must monotonically advance a finite stage/cursor. Limits are
1..=1024 records per poll. A record limit is not a hard millisecond guarantee:
bounded scalar/string/tree work and encoding byte chunks need measured frame
budgets as a separate qualification surface.

## Cancellation, retry, faults and stale work

Invalid budgets are retryable and consume zero visits. Cancel permanently
prevents finish and publication; dropping the cancelled job reclaims its
private candidate. Cancellation neither consumes nor refunds accepted sequence
IDs, modifies the running queue, changes journals nor latches an authority
fault. Restart takes a fresh capture of the full existing queue.

A deterministic arithmetic/capacity/replay/validation/receipt failure is sticky
inside the job. At the resident boundary, latch it only through `commit_step_failure` with
its executor-created opaque failure token if the capture is still current. A stale job's failure is
irrelevant to the new authority and must be discarded. The authority preserves
its preceding state, receipts and accepted queue until the existing explicit
fault recovery discards that queue. Fault replay/retry remains deterministic.

If a creator adds to the running queue while a job is pending, the old capture
becomes stale. Cancel and restart against the full new queue; never drop only
the stale tail, publish an older prefix, or reinterpret the newer command's
expected cursor. UI policy may serialize acceptance during a pending job, but
core publication must still protect against direct concurrent admission.
Saving, export and presentation see only the committed state. Save-and-leave
must cancel prospective work before recording its exact committed cursor.

## Resident 10 Hz transport

Replace the synchronous loop in `SandboxSession::update` with one resident
prospective step and bounded due-work count. Preserve integer nanoseconds,
1x/4x/20x multipliers, maximum twenty requested ticks and dropped excess debt.
Each render update shares one fixed record budget across all completed/new
jobs; do not give every child or every due tick another full 1024 visits.
Presentation, selection and inspector continue to read the last committed
boundary while a prospective tick is pending.

An explicit Step requests exactly one boundary, then polls it over updates.
Run consumes due requests only upon successful publication. Pause, modal hold,
suspension, history navigation, resident replacement and leave clear timing
debt and due requests and cancel the prospective boundary before it can publish.
Resuming starts timing at that boundary, with no hidden catch-up. Effective pace
is derived from actual publications over elapsed time; a pending candidate is
never reported as a tick. UI must distinguish processing from a fault and avoid
mistaking budget exhaustion for deterministic failure.

Cache Chronicle, notebook witness, guidance and audio changes only after
successful publication, using the returned receipt set. A stale/cancelled job
must produce no audible cue or visible decision row.

## Remaining synchronous surfaces outside the tick

`submit_running` still reconstructs a projected state by cloning current state,
replaying the queued prefix, applying the proposed revision and validating it.
Creator preview and paused apply also clone/apply/validate synchronously.
`check_revision_capacity` scans this authority and archived histories. These
are independent responsiveness work after the step executor; bounded tick
polling alone cannot qualify creator interactions at capacity. Archive replay's
running-boundary finish also calls `run_boundary_candidate`, so it must use the
same executor rather than retaining a whole-tick synchronous escape. Preserve
replay's embedded revision modes and exact paused receipt reconstruction.

## Next actionable slices and checks

First implement the opaque executor/commit fence with state-copy, source hash,
queue-journal preparation and stage skeleton; keep it off the interactive path
until every stage is bounded. A transitional synchronous child can be labeled
explicitly in internal progress, but must not be counted as a bounded visit or
advertised as responsive. Next implement command phase 1 and conflict phase 4,
then diplomacy/forecast/intents/dispatch, then incremental validation/receipt
sealing. Only then connect resident transport.

Required parity uses identical genesis/catalog/queued revisions at budgets
1/64/1024 and mixed polling sequences, comparing state, ordinary receipt,
decisions, event facts, autonomous IDs, journal modes and allocator marks.
Cover pending tails arriving during each stage, stale success and stale failure,
Pause/modal/suspension at every stage, zero/one/many civilizations, faults after
private debits, receipt-capacity failure, creator cascades and repeated forks.
No publication until Ready; every rejected commit preserves all authority
fields. Finally exercise actual native/WebGPU/WebGL2 rendering and persistence
with a capacity-sized galaxy; unit visit accounting is not frame-performance
or browser-responsiveness evidence.

## Verification of the initial partial slice

The 2026-10-01 core checkpoint ran all targets successfully: 398 tests across
24 result suites, zero failures or ignored tests, with strict all-target Clippy
also exiting zero. Four staged-executor regressions cover exact 55-boundary
state/all-receipt/journal/queue parity at budgets 1/64/1024, stale success
preserving a newly accepted tail, cancelled/incomplete non-publication, and
MAX-tick authentic arithmetic failure plus stale-failure refusal. Existing
source-copy/hash cursor tests retain their own nested-visit parity coverage.
Original and additive independent vector verification both exited zero.

The subsequent phase-one focused slice passed seven staged regressions,
including all six core creation kinds with local references, an accepted
1024-scalar-operation revision, mixed core-prefix/delegated-recovery suffix and
an identity-collision fault matching the legacy replay fault. These checks
establish bounded actual visits for the supported operation paths; remaining
creator operations and cascades are explicitly Partial.

The scheduled phase integration passed ten focused staged tests, independently
repeated by the reviewer: mixed budgets preserve WarEnded, FleetArrived and
ShipmentDelivered exactly once with matching state and all receipts; cancellation
and new pending tails inside each scheduled phase preserve committed authority;
a lifecycle arithmetic failure is sticky and atomic. Lifecycle owner/pair
indices, arrival world/war indices and each graph edge/node are reclaimed through
counted visits before Ready. Freight-return search distance/frontier retirement
is likewise counted rather than dropped during a phase transition.

The resulting frozen core checkpoint passed 409 tests across 25 result suites,
with zero failures or ignored tests. Strict all-target core Clippy exited zero
after the final visit-count regression updates. The earlier full-core run caught
two lifecycle visit-count expectations that omitted newly counted retirement;
their corrected formulas include each retained owner and ended pair. These
results cover the core crate and do not replace the workspace or browser gates.

A separate phase-four combat component now bounds actual war, fleet, hull,
facility and target-pair visits. It chooses the same minimum target class and
stable ID as the reference, preserves first-target damage application order,
and reconstructs destroyed facilities/hulls in the reference event order.
Temporary targeting indexes and dead-battery blocked reasons are retired through
counted visits before Ready. Three focused tests and strict all-target core
Clippy exited zero. The component remains separate from the staged executor:
retreat pathfinding, occupation and colony-transfer refunds, and settlement
arbitration must become bounded before phase four can be declared complete or
the executor can continue into production. This is Partial phase-four work.
The resulting combined core checkpoint passed 415 tests across 25 result suites,
zero failures or ignored tests, and strict all-target core Clippy exited zero.
The initial combined run exposed a production-only test that assumed every
sidecar event carried a generic entity; the test now checks that requirement
only for its four production event kinds while verifying the complete sidecar
against the ordinary receipt. Typed bilateral facts retain their actual parties.

This is core correctness evidence for the partial preparation and publication
interfaces. It does not establish full-step polling, incremental retirement,
resident timing, native/browser responsiveness or capacity frame performance.

## Next phase-four implementation slice (Proposed; source frozen)

Keep the production checkpoint frozen while root qualification runs. The next
source slice belongs to `simulation.rs` and the conflict sections of
`step_job.rs`; the delivery-credit producer remains Journey's separate owned
section. Introduce a private `LivingConflictJobV2` with moved candidate state,
existing pending events and phase-three arrival IDs, immutable owned catalog,
next tick, genesis seed and branch. Beginning the job must only move owners.
Its output moves those same owners onward; arrival IDs stay available to the
continuation until claims finish. No new command, receipt or gameplay rule is
part of this slice.

The parent step poll passes its remaining budget to the conflict child, adds
only the child's actual visits, and cannot report conflict completion until all
nested work and temporary retirement finish. A private completed-phase marker
must distinguish phase-four completion even with empty output or no combat
round. The synchronous continuation may then skip phases one through four
exactly once. Production and observation integration follows only after this
phase-four fence and reference parity pass; neither should silently drain a
remaining conflict stage.

Retreat precedes simultaneous combat. Build world/system metadata and active
war-pair indexes one record at a time, then visit fleets in stable fleet-ID
order. Docked foreign-owned worlds require an active war. Visit each hull to
establish catalog damage, return-credit eligibility and nonempty hull count;
any damaging hull or missing credit excludes retreat. For each qualifying
fleet, scan owned colonies in their existing order, excluding its current world,
and compare reachable destinations by `(travel duration, world ID)`. Preserve
same-system local travel and disconnected-graph behavior. Only after finding
and materializing the winning itinerary may the candidate consume every hull's
return credit, assign Return order/location and emit FleetDeparted. A failed
arrival-deadline addition faults the unpublished whole boundary.

Retreat path search must retain the exact legacy lane-ID-vector tie-breaker.
The existing freight-return cost-only search is insufficient. A first faithful
cursor can retain legacy system-selection and lane-relaxation order, with
separate stages for each system candidate, each lane, each copied path element,
each lexicographic comparison element, and each retired replaced path element.
Equal `(cost, path)` selection retains the first stable system-index candidate.
Edge-cost accumulation keeps the reference's saturating arithmetic, while
arrival tick addition keeps its checked arithmetic. Re-running bounded search
per colony is acceptable for initial correctness; selecting a destination with
a different graph algorithm requires equivalence tests before optimization.
Path vectors and replaced frontier records must not clone or destruct a whole
itinerary within a scalar visit.

After the existing bounded combat component finishes, build post-combat
presence indexes by fleet and hull visits. Keep distinct predicates:
`escort_owners` includes an escort regardless of hit points;
`defense_present` requires an owner's living escort or a living enabled battery;
`armed_presence` requires a living catalog-damaging hull or living enabled
battery; `ark_docked` retains the reference's ark-presence predicate. These
must remain distinct on non-combat ticks, when a zero-hit-point hull has not
been removed by combat. Snapshot each `(colony world, original owner)` in
original colony order before occupation changes.

For each occupation colony, scan and deduplicate eligible escort claimants in
stable actor-ID order. Contest is true unless exactly one hostile claimant
remains, there is no defending presence, and no third escort owner is at war
with that claimant. Use the existing matching occupation progress, saturating
increment, and transfer threshold. Rebuild occupations incrementally rather
than invoking sorted-vector insertion or removal that copies a whole array.
Unrelated occupation rows retain their exact keys and values.

A completed transfer changes world and colony ownership, then refunds matching
construction jobs in their stable existing order. Credit paid-alloy halves
using the existing all-or-nothing checked storage credit (ignore a refund
that would exceed the limit), remove each job and emit
ConstructionCancelled for each construction job. Next refund and remove matching
hull jobs without adding cancellation events. The directed previous-owner to
claimant relation preserves base disposition and other reasons while applying
the existing capture reason clamp. Determine whether the previous owner retains
any colony after the transfer; if none, set Dormant and emit CivilizationDormant.
Set claimant Active and emit ColonyCaptured. Refund arrays, blocked-reason
collections, and relation/occupation reconstruction each need per-record work
and retirement. There is no intermediate publication between these effects.

Settlement follows occupation and visits worlds in stable world-ID order.
Armed or already-owned worlds clear their claims through counted record work.
For a claim closing exactly now, collect actors whose matching close-tick claim
still has a docked ark, deduplicate them, and select the minimum independent
claim-rank value with actor-ID tie-breaker. Consume the lowest qualifying fleet's
first stable hull-ID ark; if its fleet becomes empty, remove that fleet using
incremental output. Update world inventory to settlement starting quantities,
insert its colony with checked hub deadlines, conditionally reactivate its
Dormant actor, clear claims and emit SettlementClaimResolved in reference order.
No present claimant merely clears claims.

If the world remains unowned, inspect phase-three arrival IDs in their existing
order. Only a docked Settle order with an ark at this world qualifies. Preserve
the reference's `close_now` snapshot for the entire world-arrival loop: it is
captured after arbitration and before arrival processing. Existing windows
admit each new actor once only while current tick is before close. With no
window in that snapshot, each qualifying arrival follows the original opening
path and emits SettlementClaimOpened; do not introduce deduplication that
changes the reference event stream. Closing or past windows admit nothing.
Any optimized claim index must preserve these branch decisions and insertion
collision behavior.

The conflict child retains cancelled and faulted owners; cancellation must not
consume accepted queue items, allocator marks or published history. A sticky
fault records visits already attempted, and the parent can produce only its
opaque authentic failure token. Cancelled jobs cannot finish. Stale success or
failure still reaches the parent's publication freshness fence. Shared pending
events preserve phase ordering and global ordinal allocation throughout
retreat, combat, occupation and claims. All temporary maps, graph edges/nodes,
claim arrays, path elements and replaced collection records retire by counted
visits before Ready; dropping large final candidate or stale-token owners stays
outside the publication operation under the existing explicit retirement gap.

Regressions must compare the complete candidate and ordinary event stream to
the unchanged phase-four reference at budgets 1/64/1024 and mixed budgets.
Cover foreign noncombatant return credits, no/distant/disconnected home colonies,
equal-cost paths and destination ties, large fleets and long itineraries;
combat target class/ID ties and simultaneous destruction; contested occupations,
zero-hit-point escorts on non-combat ticks, enabled/disabled batteries, capture
at the threshold, rejected oversized refunds followed by smaller credits that
still fit at inventory capacity, multiple captures and
last-colony dormancy; empty and multi-actor claim closure, losing/absent arks,
lowest fleet/hull ark consumption, same-boundary multi-arrival openings, and
reactivation. Inject checked deadline/receipt-capacity faults after private
mutation and verify no publication. Cancel during each nested search/refund/
arbitration/reconstruction stage, then accept a new pending queue tail and
verify stale preparation cannot consume it. Visit counts must include cleanup
and must never exceed the supplied remaining budget. Frame performance remains
unknown until this complete cursor is measured on named native/browser devices.

## Standalone retreat component checkpoint (Partial phase four)

`LivingRetreatJobV2` now implements retreat separately from staged publication.
It moves its candidate/catalog owners at begin, builds immutable graph and world
metadata one record at a time, and evaluates each qualifying fleet and hull.
Each owned-colony candidate uses a resumable faithful array-Dijkstra search:
system candidates, lanes, full-path tie comparisons, copied itinerary elements,
replaced itinerary elements and final search retirement consume separate visits.
Destination selection retains `(duration, world ID)` ordering. All prepaid hull
credits are consumed before the candidate receives its Return order/location
and FleetDeparted event. Replaced standing-order paths and temporary graph,
world, war and colony records are retired before Ready. A checked arrival-tick
fault retains the winning itinerary in the faulted candidate; its reclamation
is outside polling.

Four focused regressions exited zero. They compare state/events to the unchanged
legacy retreat reference at budgets 1/64/1024 and mixed budgets, assert exact
equal-cost lane-path and destination ties, cover local/direct/disconnected travel
and credit/damaging-hull/war eligibility, and exercise a 63-lane itinerary. A
controlled replaced-order fixture demonstrates exactly 512 additional retirement
visits for 512 retained path elements; it is cursor accounting evidence rather
than validated-genesis or player-journey evidence. Cancellation inside search
and a sticky checked deadline fault preserve unpublished ownership. Strict
all-target core Clippy exited zero. The frozen combined core checkpoint passed
420 tests across 25 result suites, with zero failures or ignored tests.
Independent source review found no blocking parity or visit-accounting issue;
its focused execution exited zero with four passed and zero failed or ignored.
The component remains off the
ordinary step and resident path; occupation and settlement were subsequently
implemented as separate bounded components below. Phase-four composition and
production/observation integration require separate qualification.

## Standalone occupation and settlement components (Partial phase four)

`LivingOccupationJobV2` builds post-combat escort and battery indexes by actual
fleet/hull/facility visits, preserves the original colony-owner snapshot, and
counts claimant and third-party hostility checks individually. Its occupation
and relation maps return through ordered per-record output. A transfer filters
construction jobs before hull jobs in stable order, attempts each paid-alloy
half refund as an all-or-nothing checked storage credit, and preserves the exact
ConstructionCancelled, CivilizationDormant and ColonyCaptured event sequence.
An oversized refund is ignored; later smaller refunds can still fit. Presence
indexes and the snapshot are retired by visits before Ready.

`LivingSettlementJobV2` indexes docked fleets and each ark by world/actor and
fleet/hull ID, retaining the distinct reference armed-presence and ark predicates.
It ranks closing claims with the independent claim formula, consumes the lowest
qualifying fleet/hull ark through incremental fleet/hull reconstruction, retires
an emptied fleet's standing-order itinerary, and installs the colony with checked
hub deadlines. Claim clearing, output and nested ark-index retirement each visit
individual records. Arrival processing preserves the world's captured close_now
value, including the existing repeated opening events when several eligible
arrivals share an initially absent window. All prior pending events and ephemeral
creator-proof candidates move through these components without replacement.

The initial focused occupation and settlement runs each passed three tests and
strict all-target core Clippy exited zero. The final additional occupation axis
covers neutral third-party presence, hostility toward the sole claimant, and
multiple hostile claimants. Final combined focused tests exited zero with seven passed; full core exited
zero with 432 passed across 26 result suites, zero failures or ignored tests.
Final strict core Clippy exited zero. Independent review found no blocking issue
and independently passed all four occupation and three settlement regressions.
Controlled fixtures cover refund capacity, multiple captures and last-colony
dormancy, enabled/disabled battery defense, zero-hit-point escort/ark distinctions,
progress reset, closing/open windows and missing arks, lowest ark removal,
Dormant reactivation, cancellation inside refund or hull consumption, an ordinary
event-capacity fault, and checked hub-deadline failure. These components remain
off the ordinary/resident path. Full phase-four composition, shared orchestration
and production/observation integration still require their own parity gate.

## Conflict composition interface checkpoint

`LivingConflictJobV2::begin` takes borrowed catalog, moved phase-three candidate,
existing pending events, next tick, genesis seed and ordered arrival IDs.
`begin_owned` takes the same inputs with an immutable Arc catalog. The public
surface is poll/cancel/visit counters/finish; children and partial candidates
remain private. The composite chains retreat, combat, occupation and settlement
in the normative order, passing only the parent's remaining visit budget to
each child. Ready child temporaries are already retired before their state/event
owners move into the next child. Ordered arrival IDs have exactly one settlement
consumer and are retired before composite Ready. Child failures remain sticky
and retain unpublished owners; cancellation prevents both polling and finish.

This standalone phase-four interface passed two focused composition regressions,
independently repeated with terminal exit zero and no blocking source-review
issue. Full core passed 435 tests across 26 result suites, zero failed or ignored,
and strict all-target core Clippy exited zero. It does not change resident authority or imply
staged whole-step integration. The next integration must add an explicit completed
phase-four marker to the synchronous remainder so retreat/combat/capture/claim
effects and event ordinals cannot be executed twice. Source-copy/hash, queued
creator and scheduled phase visit counts must share the same parent budget with
this child. Publication remains the existing opaque whole-boundary commit fence.

## Read-only phase-five/six integration audit during source freeze

Do not connect production and observation to parent polling until temporary
retirement is complete. Production currently reaches Ready after repair while
retaining catalog plan, hull-definition, world, first-deposit and nested minimum-
deposit indexes. Repair reaches Ready while retaining world and hull-definition
indexes. Recipe commit directly replaces old hub/facility blocked-reason arrays,
and clears its active plan; hub plans can be the last owners of their resource
arrays. Observation commit directly replaces remembered observation records,
which can hold nested facility and hull arrays. These are derived-cache cleanup
gaps, separate from the already tested production/observation semantic parity.

The next owned source slice needs `production_job.rs` as well as `step_job.rs`
and parent `simulation.rs` orchestration. Repair adds RetireIndices between its
last fleet output and Ready, popping each world/definition entry through a visit.
Production adds explicit retirement stages before Ready: each scalar index row,
each nested minimum-deposit entry and then its empty outer world node, and each
remaining order-set entry. Catalog plan retirement must drain input/output
resource records before destroying their last Arc owner. A shared active-plan
handle can be released while the catalog-plan map retains its owner; a hub plan
with no retained catalog owner needs its own resource retirement stages.

Recipe commit retains old or unused blocked-reason owners instead of dropping
them on assignment. Keep output owners in job fields until fallible deadline
checks complete. Retire replaced hub reasons, discarded hub attempt reasons,
and replaced facility reasons one record at a time before recipe continuation.
Facility explained-event emission retains its ordinary ordinal and fact payload;
cleanup stages add no events. Existing output colony/facility/fleet maps and
source iterators should be empty at successful Ready, with any residual nested
owner handled explicitly rather than cleared in bulk.

Observation commit retains a replaced remembered record's facility and stationed-
hull arrays, drains each member through visits, then installs the new observation.
Unseen memories remain moved unchanged; they are not gratuitously rebuilt or
cleared. Add failed-record accounting to observation poll's sticky fault path so
an attempted failing record is charged within the supplied remaining budget.
The visibility and aggregation sets already drain by record in their normal
paths; verify that they are empty before Ready.

After these cleanup checks, the staged parent can retain one unpublished candidate
through conflict, production and observation. Production takes captured catalog,
next tick, genesis seed and branch, preserving autonomous IDs. Observation takes
only the candidate and tick; pending events remain owned by the parent throughout
its event-free refresh. A private completed-stage marker distinguishes completion
through phase six even if events/arrivals/output collections are empty, and the
explicit synchronous remainder starts at diplomacy. It must skip creator,
lifecycle, arrivals, conflict, production and observation exactly once. Copy/hash,
creator, scheduled, conflict, recipe, repair and observation visits all consume the
same parent remaining budget. Delegated creator fallback remains explicitly
synchronous and cannot masquerade as a bounded child.

Regression parity compares source-reference state, ordinary receipt, decision
receipt, event-facts receipt, autonomous IDs, creator-proof binding, accepted
queue, journal modes and allocator marks at budgets 1/64/1024 and mixed budgets.
Exercise due construction/hull completions, inventory/deposit scarcity and recovery,
repairs, repeated observation replacement and unseen memory retention. Count
nested retirement directly, including unique hub-plan resource owners and old
observation rows. Cancel or accept a fresh queued tail inside conflict, recipe,
repair, nested cleanup and observation stages; stale success/failure cannot
publish or consume that tail. Faults after private debits and receipt-capacity
failure preserve the last valid committed boundary. Successful core cursor
accounting remains distinct from named-device native/browser frame evidence.

### Phase-five/six retained-owner cleanup checkpoint (2026-10-01)

Current: production now retires its plan cache, scalar world/hull/deposit indexes,
and each nested minimum-deposit entry before Ready. Last-owner plan input/output
arrays release one resource record per visit; shared arrays release only their
handle, preserving the other owner. Each recipe retains and drains replaced or
unused hub reasons and replaced facility reasons before continuing. Hub deadline
checks occur while the recipe output remains owned by the cursor on failure.
Repair retires both lookup maps before Ready. Observation replacement retains and
drains every old facility/hull detail before continuing; its failed attempted
record now consumes a visit within the requested budget.

Four new focused tests passed with actual terminal exit 0, covering last-owner
resource and nested-index counts, shared-plan preservation/cancellation, old
observation detail counts, and repair cancellation during index retirement.
These are controlled private-stage ownership/accounting fixtures; they do not
establish accepted-genesis or player-journey evidence. Existing production parity
also passed (three focused tests). Full core all-targets passed 439 tests across
26 suites, zero failed/ignored, actual terminal exit 0; strict core all-targets,
all-features Clippy passed with actual terminal exit 0. Logs:
`/tmp/nyon-phase-cleanup-tests-final.log`,
`/tmp/nyon-production-cleanup-tests.log`,
`/tmp/nyon-phase-cleanup-core.log`,
`/tmp/nyon-phase-cleanup-clippy.log`. Independent review is pending.

Partial: the composite conflict and these production/observation components are
not yet joined to the staged Step executor. Ordinary stepping still drives its
existing synchronous phases, and the resident path has not switched. No claim
of bounded phases seven through ten, final sealing, retired committed-owner
reclamation, frame timing, or platform qualification follows from this checkpoint.

### Private composition through observations (2026-10-01)

Current source: the staged Step executor chains scheduled lifecycle/arrivals,
conflict (retreat → combat → occupation → settlement), production, and observation
under the same remaining per-poll record budget as copying, hashing, and supported
creator edits. Each child hands off private state/events only after Ready and its
counted cleanup. Observation leaves the event collector parent-owned. The capture
now includes the immutable genesis seed and checks it at the freshness fence.

A private `LivingCompletedStepPhasesV2` marker distinguishes
`Scheduled { arrivals }` from `ObservationsComplete`. Empty arrivals/events/entity
collections cannot erase the completed-phase boundary. The ordinary synchronous
continuation skips phases two/three for both markers and four/five/six only for
`ObservationsComplete`. The explicit unsupported-creator fallback retains the
scheduled marker and synchronously runs conflict onward exactly once.

Fourteen focused `staged_step_` tests passed with actual terminal exit 0:
`/tmp/nyon-staged-phase6-tests-final2.log`. This includes nonempty combat event and
entity-destruction parity, cancellation and stale queued tails during each new
child, empty-boundary receipt parity, production arithmetic fault preservation,
and existing 55-tick state/ordinary/decision/facts receipt and queue/journal
parity at budgets 1/64/1024. Mixed budgets retain the same total actual visits.
Fresh strict core all-targets/all-features Clippy passed with actual terminal
exit 0 (`/tmp/nyon-staged-phase6-clippy.log`). Full core all-targets passed
446 tests across 26 suites, zero failed/ignored, actual terminal exit 0
(`/tmp/nyon-staged-phase6-core.log`). Independent review remains pending.

Partial: phases seven through ten, creator fallback, validation/sealing and
journal copying remain explicitly synchronous. Committed retired owners still
require separate reclamation. Resident transport has not switched, and no
whole-step boundedness or live platform timing claim follows.

### Standalone diplomacy cursor checkpoint (2026-10-01)

Current source: `LivingDiplomacyJobV2` performs scalar relation decay, incremental
normalized-tick snapshot hashing, record-by-record index construction, bilateral
pair/colony proposals, exact direct-lane adjacency checks, assembled fleet
eligibility, war targeting, ordered agreement/war publication, decision ordering,
and retained-owner cleanup. It uses one immutable owned post-decay state through
all proposal evaluation; snapshot hashing changes only its encoded scalar tick.
This avoids both synchronous state clones in the reference without changing what
any proposal observes. Existing ordinary hash construction keeps its original
behavior. No new wire encoding or gameplay rule is introduced.

Path searches use the existing counted duration search only for existence. Both
reference and cursor reject paths whose saturated total becomes `u64::MAX`; full
lane itinerary tie semantics are irrelevant to this boolean result. Direct
adjacency still tests same-system or a single lane, separately from reachability.
Each search's distance/frontier records retire before the next search. Graph
edges/nodes, scalar caches, old output records, and decision/proposal caches drain
before Ready. Final output ownership moves without a hidden full-state drop.

Seven focused `diplomacy_cursor_` tests passed with actual terminal exit 0:
`/tmp/nyon-diplomacy-tests-final3.log`. Coverage includes trade/nonaggression,
reciprocal war proposals from one common snapshot, decay, truces, existing
agreements, stale observations, zero/one/twenty active actors, disconnected and
saturated-cost paths, energy/order permissions, operational observed defenses,
cancellation during major stages, sticky deadline arithmetic failure, and exact
`Decision(Snapshot)` errors for invalid snapshot ticks after any due decay. State,
ordinary payloads, explained facts and decision records match the unchanged
reference, including mixed 1/3/64/1024 budgets and consistent total visits.

Full core all-targets passed 454 tests across 26 suites, zero failed/ignored,
actual terminal exit 0 (`/tmp/nyon-diplomacy-core.log`). Strict core returned actual
exit 101 with three type-complexity diagnostics in the concurrently root-owned
boundary publication tuple (`simulation.rs`); no diplomacy diagnostic remained.
Root repaired that surface with a private boundary output alias; strict core then
passed with actual exit 0 (`/tmp/nyon-diplomacy-clippy-final.log`). The subsequent
invalid-tick fault specificity regression passed focused verification. Fresh full
core all-targets passed 455 tests across 26 suites, zero failed/ignored, actual
terminal exit 0 (`/tmp/nyon-diplomacy-core-final.log`), and fresh strict core
all-targets/all-features passed with actual exit 0
(`/tmp/nyon-diplomacy-clippy-final2.log`). Independent review remains pending. The cursor
is standalone: neither ordinary rules nor staged Step nor resident transport use it.
Phase-seven integration and phases eight through ten remain Partial, along with
unsupported creator suffixes and final sealing/reclamation.

### Phase-eight implementation order and exact remaining work (read-only plan)

The reference forecast runs **200** future scheduled ticks per active actor.
Each future tick performs lifecycle expiry, arrivals, production/repair, then
fleet/freight dispatch. It deliberately omits speculative conflict, diplomacy,
new intents, and observation refresh. Consequently a fully bounded forecast
cannot be built on a synchronous phase-nine dispatcher. Implement and review a
standalone bounded dispatch cursor before claiming a bounded forecast or phase
eight. Original authority phase ordering remains unchanged; this is dependency
order for implementation only.

The next forecast interface should take an immutable common snapshot, actor,
exact catalog, seed, branch and current tick. Use counted state copying for its
private projection. Preserve the current filtering order: collect actor-owned
idle facility IDs; remove foreign routes, facilities, construction, hull jobs and
colonies; normalize foreign worlds to zero inventory and the actor's observation
owner, otherwise its surviving route contract, otherwise its recorded shipment
counterparty. An unseen current foreign capture must not enter the projection.
Each removed nested owner, lookup, reference and copied child needs counted work.

For each of the exact 200 future ticks, capture due actor repair hulls and due
facility IDs before production. Compose scheduled lifecycle/arrivals and
production with the reviewed dispatcher under one shared remaining budget. Keep
ordinary forecast event ordering/capacity, while using the existing discard-only
collector; drain its payloads and any private ephemeral records incrementally
before advancing the future tick. Preserve idle-facility removal, first-observed
deficit order, route demand tests before dispatch, hazard-adjusted batch demand,
and post-dispatch actor-owned inventory samples. Deficit membership may use an
index, but output order must remain the reference's first occurrence. Keep every
forecast private and retire its copied state/caches explicitly after consumers
finish; a whole actor forecast or future tick cannot count as one visit.

Phase-eight orchestration then needs: one normalized common snapshot digest;
actor-ordered forecasts; receiver-originated demand records only; bounded
construction/hull/supply candidate search and exact ranking; separately evaluated
settlement, ordinary fleet, emergency defense and reinforcement alternatives;
autonomous split identity rewrite with the fixed action ordinals; and category
priority sorting. All alternatives observe the common snapshot. Economy precedes
fleet within stable actor order. Only an accepted category skips its remaining
alternatives; rejected alternatives can fall back to the next ranked proposal.

Reservation requires bounded merging of the six exact delta collections in this
order: worlds, hull jobs, construction jobs, facilities, fleets, routes. A changed
or removed key conflicts when the reserved row differs from its common-snapshot
row. Validate the complete private reservation at the current tick before any
publication, restore the prior tick afterward, and preserve the precise capacity
rejection explanations. Nested fleet hull/order and facility reason comparisons
and copies must be counted. Final explanation ordering and dense ordinal remap,
including any LowerRank reference, need a bounded ordered collector rather than
an uncounted vector sort. Keep event/resource/identity/history ownership atomic
across rejection, cancellation, stale generation and arithmetic faults.

Required regressions include exact 200-tick inventory/deficit/idle forecasts,
foreign privacy and recorded-counterparty fallback, already travelling cargo,
repair/foundry scarcity and recovery, suspended/blocked/expired route contracts,
hazard-adjusted shortages, simultaneous reservation conflicts and fallback,
capacity-rejected alternatives, fixed split IDs, all three receipt bindings,
zero/one/many civilizations, mixed budgets and cancellation inside every nested
forecast/reservation stage. No production edits or new gameplay follow from this
plan; phase eight remains Partial.

### Phase nine dispatch prerequisite: implementation-ready slice (Proposed)

Implement a standalone owned `LivingDispatchJobV2` before phase-eight forecast
integration. Its private constructor receives state, pending events, catalog,
branch, genesis seed and next tick; its public polling surface follows the
existing phase jobs. Finish returns private state/events only after Ready.
Ordinary resident stepping remains unchanged. The reference order is fleets in
stable ID order, then freight routes in stable ID order; launches consume source
inventory immediately, so later fleet/route admission sees earlier spending.
This order must remain intact in both actual and projected dispatch.

Count one record per civilization, world, colony, hull-job, observation, system,
lane, agreement, hazard and catalog-definition index entry. Keep mutable source
inventory in the owned candidate; copied scalar indexes expose ownership,
location, active Trade membership and hazard definitions without repeated whole
collection scans. Derive own-world admission from colony ownership, but derive
route participants from world ownership, preserving the existing distinction.
For fleet dispatch, retain a resumable hull cursor for return-credit counts and
ark presence, count nested hull writes individually, and retire replaced order
lane paths one lane at a time. Reuse the exact retreat path search with full
lexicographic lane-path ties rather than the duration-only cargo search. A
same-system trip has the reference local duration and empty lane path.

Preserve fleet admission order: target hull jobs block departure; travelling,
missing/future orders, missing/equal destinations and unreachable paths skip;
Dormant recovery additionally requires Settle, unowned destination, a remembered
destination and an ark. Return requires an owned destination and every hull's
credit; foreign-origin recovery consumes every hull's credit; fresh foreign
excursions skip. Owned ordinary launch charges hull count plus missing credits
at the existing per-hull price. Preserve source spending, credit updates and
lane-path replacement before checked arrival expiry, even though a failed
candidate remains unpublished. Emit FleetDeparted only after successful expiry.

For freight, retain original route scalars while advancing stored suspension and
next due before later admission checks, including skipped dispatch. Membership
change permanently suspends the stored route; preexisting suspension and missing
travel cannot reopen it. Compute eligible units above reserve from inventory
remaining after fleet dispatch and earlier routes. Hazard scaling considers only
active hazards on direct lanes between endpoint systems, always scales the
original eligible batch, keeps the strictly smallest result, and therefore
retains the first minimum hazard in stable ID order. It does not scale repeatedly
or use hazards on intermediate itinerary lanes. Shipment capacity is checked
before zero-batch Corridor evidence. A zero restricted batch emits only accepted
restriction evidence; positive dispatch spends first, creates the exact
DispatchOrders/ordinal-one shipment identity, checks expiry, inserts the shipment,
emits ShipmentDispatched and finally adds the existing ephemeral restriction.
Forecast mode still collects ordinary events but suppresses explanations and
restriction evidence. Do not silently remove ordinary event-capacity failure.

Use an ordered shipment map prepared one row at a time, preserve the reference
`put` behavior of replacing an existing identical shipment key, and reconstruct
the sorted output with one row per visit. Do not invent an identity-collision
failure where the reference performs replacement; retain replaced rows for
counted retirement. All old/replaced collections and nested
paths must retire within the same remaining poll budget. Source ownership must
remain job-retained on faults/cancellation; returning an opaque failure never
publishes inventory, credits, route due times, shipment IDs or ephemeral evidence.
Catalog indexes, graphs, selected hazard copies and temporary path records also
require counted retirement before Ready. Dropping a nonempty nested owner is an
explicit unbounded cleanup limitation until repaired, not a bounded visit.

Add independently reviewed exact-reference tests for two launches competing for
source energy, mixed credited/uncredited hulls, foreign return and remembered
Dormant ark recovery, busy assembly, future orders, equal-cost path ties,
saturated/disconnected paths, deadline overflow after private spending, several
routes competing above reserve, Internal/Aid/Trade participant and expiry rules,
permanent suspension, direct versus intermediate hazards, stable minimum-hazard
ties, zero-batch evidence, full shipment capacity, identity collision, forecast
explanation suppression, and event-capacity failure. Run budgets 1/64/1024 and
mixed budgets, cancellation inside path and hull mutation, sticky faults, zero
and many actors, and counted cleanup. Integrate dispatch into the 200-tick
forecast only after this standalone slice passes independent review; the actual
phase-nine staged slot remains after phase eight.

### Private staged phase-seven integration checkpoint (Current / Partial)

The staged job now runs reviewed diplomacy only on the existing 100-tick
cadence, sharing the same remaining visit budget with phases one through six.
`ObservationsComplete` is an intermediate marker; `DiplomacyComplete` is the only
normal marker admitted to synchronous completion. A non-due or empty due stage
still reaches the explicit completed marker. The synchronous suffix skips
already-completed diplomacy once, while the explicit unsupported-creator
fallback retains its Scheduled marker and runs phases four through seven once.
The original resident path remains unchanged. Intents, dispatch, final validation,
sealing, journal copying and committed-owner reclamation remain Partial.

Verification: the latest focused staged suite terminated with exit 0, 18 passed,
zero failed/ignored (`/tmp/nyon-staged-phase7-tests4.log`). Strict core Clippy also
terminated with exit 0 (`/tmp/nyon-staged-phase7-clippy.log`). New cases use actual
accepted timelines through ticks 98, 99 and 599, compare ordinary/decision/facts
receipts and both ephemeral proof surfaces, and exercise budgets 1/64/1024 and
mixed sizes. Empty tick-100, cancellation, premature finish, stale queue tails,
and zero-visit repeated completed polls are covered. The arithmetic-failure case
uses controlled test-only boundary injection, confirms the failure occurs inside
diplomacy, and preserves the previous state/queue owners and receipts/proofs.
Full core execution also terminated with exit 0: 463 passed across 27 suite
results, zero failed/ignored (`/tmp/nyon-staged-phase7-core.log`). Independent
latest-integration review remains pending at this checkpoint; these focused results are not resident responsiveness or live
platform qualification.

Dispatch implementation ownership and transition skeleton (Proposed): append the
cursor and focused test module in `living/step_job.rs`; add only the narrow
`cfg(test)` reference wrapper before the first test module in `living/rules.rs`.
Do not change `dispatch_routes`, its Corridor producer, receipt schemas, wire
registry, resident runtime or notebook. Borrowed `begin` and static `begin_owned`
should follow the existing phase-catalog handle pattern. The standalone output
remains `(state, pending_events)`; no new authority publication interface is
needed. Staged/forecast composition is a separate reviewed follow-up.

The initial stages are Worlds (ID to stable row index, system and world owner),
Colonies (world to colony owner), Civilizations (Dormant IDs), HullJobs (busy fleet
IDs), Observations (remembered actor/world pairs), Systems/Lanes (the existing
RetreatGraph, each record counted), Agreements (active Trade pairs),
CatalogHazards (slug to fixed effect) and Shipments (move into an ordered map one
row per visit). Fleet admission stores only scalar order fields rather than
cloning nested paths. FleetHullScan derives missing credits and ark presence one
hull at a time; FleetTravel delegates the exact RetreatPathJob with the shared
remaining budget; FleetSpend applies one admitted source charge; FleetCredits
mutates one hull per visit; FleetRetirePath drains the replaced order path before
FleetDepart checks expiry and emits. Graph search keeps the parent graph owner
alive until its nested distances/frontier paths have retired.

RouteDue snapshots its fixed-size scalar row; RouteTravel resolves duration;
RouteAdvance applies permanent participant suspension then checked cadence;
RouteHazards scans one active hazard per visit using the lane and catalog indexes,
retains the first strictly strongest restriction and the exact original batch;
RouteAdmit checks capacity/zero batch/spending; RouteCreate forms one fixed-size
shipment identity/record; RouteEmit and RouteEvidence preserve the reference
failure order. A skipped route still retires its selected path and hazard copy
before the next row. Fleets and routes can iterate stable source row indices
because dispatch adds/removes neither collection; only shipments need ordered
map reconstruction. Map length plus inserted/replaced admission reproduces the
reference capacity check before `put`. Finish stages emit ordered shipments and
retire every scalar index, graph row, selected path and replaced row through the
same remaining budget. Ordinary event and ephemeral witness vectors stay moved
in the output; forecasting callers must retire discarded events separately.

### Reservation lookup performance checkpoint (Current / Partial)

A delayed-step sample identified repeated linear membership probes in
`reserve_delta` as a dominant synchronous cost. The helper now uses binary
lookup on the existing sorted, unique vectors at its four lookup sites. It keeps
proposal traversal, deletion traversal, row equality, `put`/`remove_key`, first
conflict, partially mutated private candidate, and caller validation order
unchanged. This is a measured hotspot repair, not completion of bounded intent
reservation: changed-row collection rewriting and final validation remain
synchronous. The old linear helper is retained only inside the test module.

Regression source compares 8,192 mixed insertion/change/deletion/conflict cases,
including exact partial candidates when rejection happens after earlier writes.
A separate 1,024-row unchanged-proposal test counts key probes and rejects
quadratic membership growth; actual shared-snapshot reservation tests remain in
the focused run. Independent source review found no blocker. Owner execution terminated with exit 0: 38 focused tests passed, zero failed or
ignored (`/tmp/nyon-reservation-optimization-tests1.log`); strict core Clippy
terminated with exit 0 (`/tmp/nyon-reservation-optimization-clippy1.log`).
Independent execution also terminated with exit 0 for both new regressions:
exhaustive equivalence and logarithmic lookup count, one test each
(`/tmp/nyon-review-reservation-binary.log` and
`/tmp/nyon-review-reservation-logbound.log`). No speedup or live frame-time claim
follows until measured on the newly qualified artifact.

### Standalone phase-nine dispatch checkpoint (Current / Partial)

`LivingDispatchJobV2` now implements fleet-then-freight dispatch on an unpublished
owned candidate with borrowed or owned immutable catalog handles. Polling counts
scalar index construction/retirement, nested hull scans and credit mutations,
exact lane-path Dijkstra work and path retirement, due advancement, hazard scans,
resource spending, shipment identity/creation/replacement retirement, ordinary
emission and Corridor evidence admission. Source inventories remain shared across
fleet and route admission in their original stable order. Finish is Ready-only;
normal Ready drains every temporary graph/index/path/map owner. The existing
Corridor producer, canonical schemas and resident/simulation paths are unchanged.

Latest owner focused execution terminated with exit 0: 10 tests passed, zero
failed/ignored (`/tmp/nyon-dispatch-tests5.log`). Fresh strict core Clippy also
terminated with exit 0 (`/tmp/nyon-dispatch-clippy-final.log`); owned diff whitespace
checks passed. Budgets 1/64/1024 and mixed sizes produce exact unchanged-reference
state and pending events, equal total visits, and never exceed requested visits.
Coverage includes competing launches/freight, mixed credits, foreign Return and
zero-colony Dormant ark recovery, assembly/future/missing/travelling orders,
lexicographic path ties, local/disconnected/saturated paths, Internal/Aid/Trade
contracts and half-open expiry, due advancement and suspension, route reserves,
direct versus intermediate hazards, noncompounding minimum ties, zero batches,
forecast evidence suppression, full shipment admission and identical-key
replacement, arithmetic after private resource changes, ordinary-event capacity,
cancellation during nested stages, and normal cleanup with zero/many actors.

Earlier broad logs two and three failed because test IDs used repeated bytes
while the built-in fixture uses a discriminator only in byte zero. The fixture
was corrected; logs four and five both terminated with exit 0. Component fixtures
and controlled fault injections do not constitute archive or live player journeys.
Independent latest-dispatch review and the parent broad gate are pending.
Cancellation/failure destruction and release of a last catalog owner remain
caller cleanup boundaries, rather than bounded polling guarantees. This cursor
has not been composed into the 200-tick forecast or the private staged executor;
phase eight and full resident bounded-step qualification remain Partial. No
runtime speedup claim is supported by these cursor tests.

### Next ownership boundary after independent dispatch review (Proposed)

The next slice can reuse `LivingDispatchJobV2` without changing public command,
receipt, archive, store or identity schemas. Keep implementation in the existing
`step_job.rs` and narrow test-only reference wrappers in `rules.rs`; do not create
a module or change the facade registry. First implement and review a private
forecast projection/copy cursor, then the standalone 200-tick forecast composer,
then economic/fleet proposal and reservation cursors. `simulation.rs` integration
and resident switching remain separate sequential handoffs. A forecast output
contains only the existing inventories, first-occurrence deficits and initial
idle-facility list; its projected final state remains test evidence, never an
authority publication or explanation receipt.

Projection uses the existing bounded state-copy job. Capture own initial idle
solar/extractor/foundry facility IDs before filtering. Remove foreign routes,
facilities, construction jobs, hull jobs and colonies individually, retiring
nested facility reasons with counted visits. Do not additionally remove foreign
fleets, observations or travelling shipments: the reference retains them. Set
the private state tick to the decision tick. Zero every non-owned world's
inventory and derive its owner from actor-specific observation, then the first
surviving actor-origin route to that world, then the first sorted shipment
matching the recorded counterpart. An observed `None` is authoritative absence
and must not fall through to contracts. Route fallback does not filter suspended
or expired contracts. Never read actual unseen foreign owner/inventory as a
fallback. Preserve original stable-ID choice when building these indexes.

For each offset 1 through 200, check the future tick before constructing any
child. Begin a fresh `for_forecast` pending collector and compose lifecycle,
arrivals, production/repair and dispatch using a single remaining poll budget.
Do not run conflict, observations, diplomacy, new intents or queued creator
commands. Before production, capture own docked hulls with due repairs (fleet,
hull, world, previous HP) and facility IDs with due recipes. After production,
remove initial idle facilities only when that captured due facility is now
Operational. Append repair and facility MissingInput deficits only on their
first occurrence, in the original fleet/hull then facility/reason traversal.
Use a membership set for lookup while preserving the append vector's order.

Route scheduled demand is a distinct pre-dispatch scan. It occurs after
production but **before fleet launch spending**. It uses original route batch
size scaled by endpoint-direct active hazards, not actual eligible dispatch
above reserve. Check actor-origin due/non-suspended routes, owned source colony,
recorded destination owner, reachability and active Trade where needed. Append
scarcity only when current stock is below saturating reserve plus scaled batch.
Do not substitute the dispatch cursor's post-launch admission result for this
predicate. Initially implement a separate bounded demand cursor using the same
path primitive and hazard scaling rules; rebuilding indexes is acceptable
counted work. Sharing immutable topology between children is a later optimization
requiring unchanged ownership semantics and measured need.

After the reviewed dispatch child finishes, set the forecast tick and sample
inventories in world ID order only where projected world owner equals the actor.
Retire discarded ordinary event records individually before the next future
boundary. Forecast explanation suppression does not disable ordinary event
ordering or its 65,536-event capacity. Arrival IDs, temporary recipe outputs,
repair snapshots, due-facility membership, child graphs/indexes and replaced
state owners must also retire within the shared budget. At offset 200 retire the
projected state incrementally before releasing production output; a test-only
final-state return may move it instead. Cancellation or a sticky fault cannot
publish projected actor status, cargo identities, resources or receipt evidence.

Required standalone forecast parity compares the exact 200-tick inventory
sequence, deficit order, idle list and test final state against the unchanged
reference. Include late paid completions, repair and foundry scarcity/recovery,
foreign inventory/capture privacy, observed unowned versus missing observation,
recorded shipment counterpart fallback, incoming cargo, Internal/Aid/Trade
expiry/suspension, direct/intermediate hazard demand, and launch spending before
actual freight. Run mixed budgets and cancellation inside projection, each child,
route demand, sample collection and event/state retirement; future-tick arithmetic
and ordinary event-capacity faults preserve caller authority. Review each slice
before introducing economic candidate or reservation composition.

### Final boundary sealing and retirement work still required (Partial)

After bounded phase eight and dispatch, set the candidate tick, then replace the
synchronous `candidate.validate` with a counted validator preserving exact global
ID/reference/capacity/catalog and temporal checks. Move the final candidate into
its prepared Arc once; reuse the existing incremental state-hash job rather than
serializing the complete state synchronously. Sort/deduplicate applied revision
IDs with a bounded ordered collector, and seal ordinary events incrementally in
canonical ordinal order with the existing formulas. Decision sealing still needs
bounded ordering, dense ordinal and LowerRank remapping, nested reason emission
and canonical hashing. Sparse event-facts admission, omission coalescing, size
measurement and hashing need a separately reviewed cursor owned by the facts
producer maintainer; do not alter its canonical formulas or frozen vectors.
Bind creator recovery and Corridor witnesses to the final ordinary receipt only
after it exists, validating final actor/world and tick predicates per record.
All candidates remain private until every seal succeeds and the capture fence
still matches.

Revision/mode/queue journal construction still clones collections synchronously.
Replace that with counted copying and precreated Arc owners; commit must remain
the existing opaque prepared swap. `LivingRetiredBoundaryV2` currently only
returns old owners; implement a retirement cursor that tries to unwrap each Arc
without cloning shared state, releases shared handles once, and drains unique
state/journal/receipt owners by record, including nested hulls, lane paths,
facility reasons, observed details, decision reasons and facts. Failed, canceled,
stale and abandoned preparations also need an explicit retained cleanup owner;
consuming an error path must not silently drop a large private candidate on the
resident thread. Last immutable catalog-owner reclamation needs a private
catalog handoff if possible; normal committed boundaries retain the authority's
catalog handle, but session replacement can remove that owner. Root owns the
resident/session integration and final facade changes; authority owns staged
composition, validator/hash/journal/retirement internals; facts maintenance owns
its sealing cursor. None of these remaining interfaces grants arbitrary prepared
success or fault construction.
