# NYON Living Galaxy V2: creator operation rules addendum (draft for review)

Date: 2026-09-17

Status: **Draft for the owner's review. Not normative. No code depends on it yet.**
Parent contract: `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` (cited below as `rules`).
Written against `main` at `d12d3cc` (authority Task 5, first slice `04c678f`).

## 0. Scope, reading key and method

- **What this covers.** The 19 variants of `LivingCreatorOperationV2` (`crates/nyon-workshop-core/src/living/command.rs:111-435`) that `is_applied_by_this_slice` (`simulation.rs:639-652`) refuses today with `UnsupportedOperation`:
  - eight creations: `CreateFacility`, `CreateConstructionJob`, `CreateHullJob`, `CreateFleet`, `CreateHull`, `CreateRoute`, `CreateShipment`, `CreateHazard`;
  - eleven edits and removals: `SetWorldOwner`, `CreateColony`, `RemoveColony`, `SetRelationBase`, `SetAgreement`, `RemoveAgreement`, `ForceWar`, `ForcePeace`, `SetFleetOrder`, `SetShipmentDisposition`, `RemoveEntity`.
  - 8 + 11 = 19. The other 9 of the 28 are already applied and are only mentioned where they interact.
- **What this does not change.** The operation schema (field names and order) is taken as frozen, because `revision_id` hashes it (`DECISIONS-PENDING.md` entry 2). Every rule below is expressible over the existing fields. Where the schema cannot express something the spec requires, the rule says so and the gap is listed in section 5, rather than being solved by inventing a field.
- **Tags.** Every numbered rule carries exactly one tag:
  - `[D <source>]` **derived**: follows from the cited spec line, plan line or code line. File line numbers are as of `d12d3cc`.
  - `[P]` **proposed**: a new decision for Donald. Each gives the rejected alternative and why.
- **Abbreviations.** `rules Lnnn` is a line of the rules spec. `auth Lnnn` is `docs/superpowers/plans/2026-09-04-nyon-living-galaxy-authority.md`. `civ Lnnn` is `docs/superpowers/plans/2026-09-04-nyon-living-galaxy-civilizations.md`. `DP#n` is entry n of `docs/superpowers/DECISIONS-PENDING.md`. Source files are under `crates/nyon-workshop-core/src/living/`.
- **Symbols.** `A` is the application boundary of the revision (defined in X1). `op` is the operation's position in its batch.

## 1. Cross-cutting rules

These are stated once. Each per-operation section refers to them by number instead of restating them.

### Application boundary and where validation runs

- **X1** [D command.rs:675-679, simulation.rs:334-340 and 372] `A = revision.tick`. For a running envelope accepted at completed tick `t`, `A = t+1 = T`. For a paused envelope, `A = t`, the unchanged current boundary. Every tick-relative rule below is written against `A` so one rule serves both modes. `A` is not `state.tick`: inside `apply_revision` that is still `t` in both the submission projection and phase 1 replay, because only `CommitBoundary` advances it (`simulation.rs:552`).
- **X2** [D rules L74, simulation.rs:348-353 and 574-579] Every rule in this document is checked inside `apply_revision`, not only in a submission pre-pass. The same function runs at submission (against the private projection holding every earlier queued envelope) and at phase 1 replay, so a rule that stops holding between submission and the boundary surfaces as `LivingDeterministicFaultV2::Replay`, never as a silently different state.
- **X3** [D rules L74, rules L254] A batch applies all of its operations or none. Any rejection of any operation rejects the envelope, consumes no sequence, entity identity, queue position or resource, and leaves the authority byte-identical.
- **X4** [D command.rs:29-33, rules L74] Each operation is checked against the projection as already mutated by the earlier operations of the same batch and by every earlier queued envelope. Batch order is the only order: a `SetWorldOwner` at position 2 is visible to a `CreateColony` at position 3.
- **X5** [D simulation.rs:353, 377, 556] After the last operation, `LivingGalaxyStateV2::validate` still runs on the whole projection. The rules below are the checks `validate` deliberately does not make (`model.rs:1204-1231`); they do not replace it.
- **X6** [D rules L80, rules L313] No rule iterates a collection in any order but ascending stable key, and none reads wall time, insertion order, presentation state or the listed order of a `dependents` array (see X19).

### Resources, refunds and reservations

- **X7** [D rules L43, rules L127, civ L24, civ L1435] Creator operations are free. Phase 1 never deducts a resource, never pays a refund and never emits `refund_discarded`. The only way a creator operation changes an inventory is `SetWorldInventory` (already applied) or a removal that takes an inventory with its world.
- **X8** [D command.rs:188-189, rules L127 and L129] `paid_alloy` on `CreateFacility`, `CreateConstructionJob` and `CreateHullJob` is a declared value. It is not charged now. It is what a later civilization scrap, cancellation or capture half-refunds.
- **X9** [D rules L66, rules L296-298] Creator operations count every outstanding reservation. After every operation, the projection must satisfy all six of:
  1. `facilities + construction_jobs <= 2048`;
  2. `total hulls + hull_jobs <= 2048`;
  3. `fleets + count(hull_jobs where target_fleet is null) <= 256`;
  4. for each fleet `F`, `hulls(F) + count(hull_jobs targeting F) <= 16`;
  5. for each colony world `W`, `sum(slot_cost)` over its facilities and construction jobs `<= 6` (`LIVING_COLONY_INDUSTRY_SLOTS_V2`), and for each industry definition `d`, `count(facilities and construction jobs of d on W) <= d.max_per_colony` (catalog `slot_cost`, `max_per_colony`, `catalog.rs:331-333`);
  6. for each world `W`, `count(hull_jobs on W) <= sum(concurrent_jobs)` over the built `hull_assembly` facilities on `W` (`catalog.rs:303-307`; core pack sets 1).
- **X10** [P] The six X9 checks run in `apply_revision` for the creator path and return a new `LivingCommandRejectionV2::Capacity { operation, collection, limit, found }`. This resolves `DP#7` for the creator path only; the civilization path keeps its own home in `civilization.rs` as the civilizations plan intends, calling the same helper. *Rejected alternative:* enforce them in `LivingGalaxyStateV2::validate`. That would also reject a validly imported genesis that the owner has not yet decided must obey slots, and `model.rs:1208-1215` records the owner-facing reason for keeping them out.
- **X11** [P] Rule 5 of X9 binds colony worlds only. A non-colony world may hold any number of creator facilities up to the global cap (`rules L101`, "creator-created noncolonized industry can exist"). `CreateColony` therefore checks that the world's existing facilities fit before it makes the world a colony (see CC.6). *Rejected alternative:* apply the six-slot rule to every world. It contradicts L101's explicit carve-out for noncolonized industry.

### Rejections

- **X12** [P] Five rejection variants are added to `LivingCommandRejectionV2`, each carrying `operation: usize`:
  - `Capacity { collection, limit, found }` (X10);
  - `ReservationConflict` (a fleet with a pending hull delivery, rules L66);
  - `BlockingDependencies { first_unlisted: LivingEntityIdV2 }` and `UnexpectedDependent { entity: LivingEntityIdV2 }` (RemoveEntity, section 3);
  - `OperationRule { rule: &'static str }` for every other per-operation rule, with the stable rule codes named in the rules below (for example `route_cadence`).
  - The names follow the civilizations plan where it already chose one (`civ L422, L437, L1413`). *Rejected alternative:* reuse `Projection(LivingValidationErrorV2::Range)`. It would make a creator-rule refusal indistinguishable from a state-schema failure in the preview.
- **X13** [D simulation.rs:835-850, model.rs:1738-1746] A target naming an existing entity must be found in the collection the field names. An identity that exists but belongs to another collection is `MissingTarget` with that field's reference name, because identities are globally unique (`model.rs:1324-1352`).
- **X14** [P] An edit that sets a value equal to the current one is accepted and changes nothing but the revision log and its event. *Rejected alternative:* reject no-op edits. A preview built on a slightly stale projection would then fail for no semantic reason. `ForcePeace` and `RemoveAgreement` are the exceptions: they have nothing to act on, so they reject with `MissingTarget` (FP.1, RA.1).

### Events

- **X15** [D simulation.rs:29-30] Today each applied revision emits exactly one `creator_intervention` event with `Creator { revision }` provenance.
- **X16** [P, option A, recommended for this slice] Keep X15 for all 19 operations: one `creator_intervention` per revision, no other event from phase 1. Every retired dependent, cleared order and discarded cargo is recoverable from the revision's own command bytes plus the pre-state, because revisions carry the creator's inputs (rules L317) and application is deterministic. *Rejected alternative (option B):* add one event kind per effect (`fleet_order_retired`, `shipment_discarded`, `entity_removed`, `occupation_reset`, `route_suspended`). Adding a kind is not a format break (`receipt.rs:28-34`), but a kind alone still cannot say *which* entity, and giving events a subject changes `LivingEventPayloadV2` and therefore `canonical_receipt_bytes` and every receipt vector. See contradiction K1; this is open question Q1.

### Civilization status

- **X17** [P] At the end of a batch, for each civilization whose colony count changed during that batch, set `status = Active` if it holds at least one colony and `Dormant` otherwise. A civilization whose colony count did not change keeps the status it had, including a status set directly by `CreateCivilization`. *Rejected alternative 1:* phase 1 never touches status. That leaves a creator-removed last colony with an `Active` owner, which contradicts rules L220. *Rejected alternative 2:* make status a validator invariant. That is a model change and would reject existing genesis fixtures that pair `Active` with zero colonies; it is open question Q4.
- **X18** [D rules L220, civ L1034] A status change made by X17 is the creator's, not gameplay's. Under X16 it emits no `civilization_dormant` or `civilization_reactivated` event (those remain autonomous phase-4 events).

### Removal ordering

- **X19** [P] Wherever a rule removes several records, the result is computed as a set and then applied in reverse state-schema field order (field 23 `occupations` down to field 4 `systems`), ascending stable key inside each collection, with a fleet's hulls removed with their fleet. The order in which the author listed dependents never matters. *Rejected alternative:* remove in listed order. Two listings of the same set would then be two different revision identities for an identical effect, the same defect `command.rs:22-27` rejects for gapped locals.

## 2. Per-operation rules

Each operation has four parts: **Preconditions** (checked at submission and again at replay, per X2), **Effects** in phase 1, **Reservation and refund**, **Later phases**. Cascades are in section 3. Events follow X16 unless stated.

### 2.1 `CreateFacility`

Preconditions:
- **CF.1** [D model.rs:1433-1437] `world` exists; `definition` names a catalog industry.
- **CF.2** [D rules L101, X11] Any world may receive a facility; no colony or owner is required.
- **CF.3** [P] `hit_points` must be `1..=effect.hit_points` for a `defense` definition and exactly `0` otherwise; code `facility_hit_points`. This closes `DP#8` for the creator path. *Rejected alternative:* leave facility HP unchecked. An undamageable facility with nonzero HP, or a battery at 0 HP, has no meaning in rules L111 or L214.
- **CF.4** [P] `next_due` must be `Some(A + period_ticks)` for a `recipe` definition and `None` for `hull_assembly` and `defense`; code `facility_clock`. Grounds: rules L95 says "Creator-created already-built facilities use the same rule" as genesis (clock = creation boundary plus period), and a command is not the "validated import" that L95 exempts. *Rejected alternative:* accept any `next_due > A`, treating the field as an import-supplied clock. That lets an author phase-shift a recipe, which L95 does not grant a command. Open question Q6.
- **CF.5** [P] `status` must be `operational`; code `facility_status`. There has been no due attempt, and `operational` is the chosen stand-in for "never ran" (`DP#4`). *Rejected alternative:* accept `blocked`. A blocked facility must carry its reasons (rules L95), and the operation has no field for them.
- **CF.6** [P] `paid_alloy <= definition.alloy_cost`; code `facility_paid_alloy`. A later scrap half-refunds it (X8), so an unbounded value would mint alloy through a civilization scrap. *Rejected alternative:* require equality. A creator may legitimately place a facility "given for free" with `paid_alloy = 0`.

Effects:
- **CF.7** [D command.rs:861-894, rules L355] Insert `LivingFacilityV2` with the identity from `created_entities`, the given fields, and the fields the operation does not carry set as in CF.8.
- **CF.8** [P] `next_repair = None` when `hit_points` equals the definition maximum or is 0, else `Some(A + 10)`; `blocked_reasons` empty. Grounds by analogy with rules L131 ("first damage at D sets `next_repair=D+10`"), treating creation of a damaged battery as damage at `A`. *Rejected alternative:* `None` always. A damaged battery would then never repair, contradicting L131.

Reservation and refund:
- **CF.9** [D X9 rules 1 and 5] Counts against global facility capacity and, on a colony world, against slots and `max_per_colony`. No deduction (X7).

Later phases:
- **CF.10** [D rules L95, simulation.rs:590-635] Phase 5 evaluates the recipe first at `next_due = A + period`, never at `A`. Phase 4 targets a battery in combat. Phase 6 makes it observable. All of these hooks are empty today, so the facility is inert until the civilizations plan fills them.

### 2.2 `CreateConstructionJob`

Preconditions:
- **CJ.1** [D model.rs:1439-1454] `world`, `owner`, `definition` exist; `accepted_tick < completion_tick`.
- **CJ.2** [P] `world` is a colony and `world.owner == owner`; code `job_requires_colony`. Grounds: a job sits in a colony's building queue (rules L101, civ L23), and rules L101 requires an owned colony for *autonomous* construction; extending that to creator jobs is a decision. *Rejected alternative:* allow creator jobs on any world, as CF.2 allows creator facilities. A job needs an owner who paid (`owner` field) and a queue to sit in, and a non-colony world has neither queue nor, possibly, owner.
- **CJ.3** [D rules L95, rules L105-111] `completion_tick - accepted_tick == definition.build_ticks`; code `job_duration`.
- **CJ.4** [P] `accepted_tick <= A < completion_tick`; code `job_window`. A job cannot have been accepted in the future, and one due at or before `A` would either complete on its creation boundary (running mode, phase 5 follows phase 1) or never (paused mode, phase 5 of `A` already ran). *Rejected alternative:* allow `completion_tick == A` in running mode only. It makes the same batch mean different things in the two modes.
- **CJ.5** [P] At most one outstanding construction job per colony; code `building_queue_full`. rules L101 gives each colony "one building queue" without a length, and each job carries its own `accepted_tick`/`completion_tick` with no queue-wait field, which only a length-one queue makes consistent. *Rejected alternative:* unbounded queue. Jobs would then complete in parallel, which is not a queue. Open question Q5.
- **CJ.6** [P] `paid_alloy == definition.alloy_cost`; code `job_paid_alloy`. A job is by definition paid up front (rules L103), and a later cancellation half-refunds exactly that. *Rejected alternative:* `<=` as in CF.6. A partially paid job does not exist in rules L103.

Effects:
- **CJ.7** [D command.rs:196-211] Insert `LivingConstructionJobV2` with the given fields.

Reservation and refund:
- **CJ.8** [D rules L66, X9 rules 1 and 5] Reserves one global facility slot and the colony slot and role count through completion. No deduction (X7).

Later phases:
- **CJ.9** [D rules L88, rules L95] Phase 5 at `completion_tick` turns it into a facility whose first recipe is due at `completion_tick + period`. Capture in phase 4 cancels it with a half refund of `paid_alloy` (rules L129).

### 2.3 `CreateHullJob`

Preconditions:
- **HJ.1** [D model.rs:1455-1466] `world`, `owner`, `definition` exist; `target_fleet`, if present, exists; `accepted_tick < completion_tick`.
- **HJ.2** [P] `world` is a colony owned by `owner` and holds a built `hull_assembly` facility; code `hull_job_requires_shipyard`. A queued shipyard does not count. The shipyard half follows rules L101 and L242 (a hull needs a shipyard); the colony half is the same decision as CJ.2. *Rejected alternative:* allow a creator hull job at a shipyard on a non-colony world. It is then unclear whose fleet the completed hull joins when `target_fleet` is null and the world is unowned.
- **HJ.3** [D rules L135-139] `completion_tick - accepted_tick == definition.build_ticks`; code `job_duration`.
- **HJ.4** [P] `accepted_tick <= A < completion_tick` and `paid_alloy == definition.alloy_cost`, for the reasons in CJ.4 and CJ.6; codes `job_window`, `job_paid_alloy`.
- **HJ.5** [D rules L66, model.rs:541-543] If `target_fleet` is present: the fleet's owner is `owner`, and it is `Docked` at `world`; code `hull_job_target_fleet`.
- **HJ.6** [P] If `target_fleet` is present and that fleet's `order` is non-null with a kind other than `defend`, reject with `ReservationConflict`. rules L66 makes a hull-delivery reservation mutually exclusive with departure; this is the same exclusion applied from the other side. *Rejected alternative:* accept and let phase 9 hold the departure. rules L66 says to reject such conflicts "before changing its existing order", not to park them.

Effects:
- **HJ.7** [D command.rs:213-230] Insert `LivingHullJobV2`. A `Local` target resolves through `created_entities`, so a batch may create a fleet and then a job for it.

Reservation and refund:
- **HJ.8** [D rules L66, rules L296-298, X9 rules 2, 3, 4 and 6] Reserves one global hull, one fleet slot when `target_fleet` is null, one per-fleet slot when it is present, and one shipyard concurrency slot. No deduction (X7).
- **HJ.9** [D rules L66] While the job exists, the target fleet cannot depart, split or merge (SFO.6).

Later phases:
- **HJ.10** [D rules L88, rules L141, rules L298] Phase 5 at `completion_tick` adds the hull to the target fleet, or creates a new docked fleet at `world`; the hull cannot depart on its completion boundary. Destruction or capture of the target in phase 4 cancels the job with a half refund (rules L66).

### 2.4 `CreateFleet`

Preconditions:
- **FL.1** [D model.rs:1474-1479] `owner` and `world` exist.
- **FL.2** [P] Any world is allowed, owned or not, colony or not. The creator is omniscient (rules L43) and fleets exist away from colonies in play (rules L149). *Rejected alternative:* require an owned colony. Nothing in rules L254 restricts where a creator may place a fleet.
- **FL.3** [P] By the end of the batch, every fleet the batch created holds at least one hull; code `empty_fleet`. *Rejected alternative:* allow empty fleets, which `validate` accepts. An empty fleet counts against the 256-fleet cap, takes part in no rule in sections 4 to 6, and would be a zero-strength actor in the war heuristic of rules L206. Open question Q7.

Effects:
- **FL.4** [D command.rs:231-239, model.rs:665-679] Insert `LivingFleetV2 { location: Docked { world }, order: null, hulls: [] }`, filled by later `CreateHull` operations of the batch.

Reservation and refund:
- **FL.5** [D X9 rule 3] Counts against the fleet cap including new-fleet hull-job reservations.

Later phases:
- **FL.6** [D rules L214-216] A fleet docked at a world where a war enemy is present takes part in the next phase-4 combat round, on a multiple of 10. An unescorted noncombatant fleet may retreat in phase 4.

### 2.5 `CreateHull`

Preconditions:
- **HU.1** [D model.rs:1499-1510] `fleet` exists; `definition` names a catalog hull; `1 <= hit_points <= definition.hit_points`.
- **HU.2** [D rules L141] The fleet is `Docked`; code `hull_fleet_travelling`. "Hulls cannot merge while travelling"; adding a hull to a moving fleet is a merge.
- **HU.3** [D rules L66] The fleet has no pending hull-delivery reservation that the addition would overfill; covered by X9 rule 4.

Effects:
- **HU.4** [D command.rs:240-252] Insert `LivingHullV2` into the fleet's sorted hull set.
- **HU.5** [P] `next_repair = None` at full HP, else `Some(A + 10)`; same reasoning and alternative as CF.8.

Reservation and refund:
- **HU.6** [D X9 rules 2 and 4] Counts against the global hull cap and the fleet's 16.
- **HU.7** [D rules L147] `return_credit` is a creator grant of a prepaid return itinerary. It costs nothing (X7).

Later phases:
- **HU.8** [D rules L131, rules L214] Phase 5 repairs it from `next_repair` when docked at an owned colony; phase 4 may damage or destroy it.

### 2.6 `CreateRoute`

Preconditions:
- **RT.1** [D model.rs:1519-1550] Both worlds and both owners exist; `resource` is in the catalog; `source_world != destination_world`.
- **RT.2** [D rules L171] `1 <= batch_size <= 10`; `cadence_ticks == 10`; `source_reserve >= 20`; codes `route_batch`, `route_cadence`, `route_reserve`. `validate` checks only nonzero (`model.rs:1539-1550`); see K5.
- **RT.3** [D rules L171] The two worlds are in the same system, or their systems are joined by a lane; code `route_connection`.
- **RT.4** [D rules L173, rules L177, civ L658] Permission by kind; code `route_permission`:
  - `internal`: `source_owner == receiver_owner`, and both worlds are currently owned by that civilization;
  - `trade`: `source_owner != receiver_owner`, `source_world.owner == source_owner`, `destination_world.owner == receiver_owner`;
  - `aid`: as `trade` for ownership. It is one-way by construction and cannot draw from a world the source owner does not own.
- **RT.5** [P] A `trade` route additionally requires a trade agreement in force at `A` (`start_tick <= A < end_tick`); code `route_agreement`. rules L177 says a route "records the source and receiver owners whose permissions validated it", which reads as validation at creation. *Rejected alternative:* check only at dispatch (rules L173). Then a creator could create a trade route between enemies that is permanently inert, and the preview would show it as valid.
- **RT.6** [P] `next_due == A + cadence_ticks`; code `route_clock`. Same reasoning as CF.4, applied to the route cadence. *Rejected alternative:* any `next_due > A`. Open question Q6 covers both.

Effects:
- **RT.7** [D command.rs:253-277, rules L177] Insert `LivingFreightRouteV2` with `suspended = false`: RT.4 has just proven the participants equal the current owners.

Reservation and refund:
- **RT.8** [D model.rs:1274] Counts against 2048 routes. No resource moves at creation.

Later phases:
- **RT.9** [D rules L92, rules L171-177] Phase 9 attempts dispatch at `next_due` and every 10 ticks after, revalidating ownership and agreement each time; phase 2 storms halve the batch.

### 2.7 `CreateShipment`

Preconditions:
- **SH.1** [D model.rs:1552-1583] Worlds and owners exist; `1 <= units <= 10000`; `departure_tick < arrival_tick`.
- **SH.2** [D rules L145, rules L175] `arrival_tick - departure_tick` equals the travel duration between the two worlds: 10 ticks in one system, `max(10, ceil(distance_units / 10))` over a direct lane; and RT.3's connection rule holds; codes `shipment_duration`, `route_connection`.
- **SH.3** [P] Timing by disposition; code `shipment_window`:
  - `outbound` and `returning`: `departure_tick <= A < arrival_tick`;
  - `waiting_for_storage`: `arrival_tick <= A`.
  - *Rejected alternative:* the same window for all three. A waiting shipment has by definition already arrived (rules L186).
- **SH.4** [P] `dispatch_owner != intended_receiver` is **not** required. Internal shipments exist (rules L171). *Rejected alternative:* require distinct owners; it would make internal cargo unrepresentable.

Effects:
- **SH.5** [D command.rs:278-300] Insert `LivingShipmentV2` with the given manifest.
- **SH.6** [D rules L43, X7] No inventory is deducted from `source_world`: the cargo is a creator grant.

Reservation and refund:
- **SH.7** [D rules L186, model.rs:1277] Counts against 4096 shipments while it exists, including while waiting.

Later phases:
- **SH.8** [D rules L179-186] Phase 3 resolves it at `arrival_tick` (and every tick while waiting) with the capture, delivery, one-time return and waiting rules. Relationship credit on delivery is undeterminable from the manifest; see K2.

### 2.8 `CreateHazard`

Preconditions:
- **HZ.1** [D model.rs:1696-1703] `lane` exists; `definition` names a catalog hazard; `start_tick < end_tick`.
- **HZ.2** [P] `start_tick` is no earlier than the first boundary whose phase 2 has not yet run: `start_tick >= A` in running mode and `start_tick >= A + 1` in paused mode; code `hazard_window`. Phase 2 then observes both the start and the end. *Rejected alternative:* allow a hazard already in progress. It would be active without ever having started, so a receipt stream would show `hazard_ended` with no matching `hazard_started`. Open question Q8.

Effects:
- **HZ.3** [D command.rs:301-313] Insert `LivingHazardV2`.

Reservation and refund:
- **HZ.4** [D model.rs:1282] Counts against 128 hazards.

Later phases:
- **HZ.5** [D rules L85, rules L175] Phase 2 starts it at `start_tick` and ends it at `end_tick`; phase 9 applies the strongest active reduction to new dispatches only.

### 2.9 `SetWorldOwner`

Preconditions:
- **WO.1** [D model.rs:1377-1379] `world` exists; `owner`, if present, is a civilization.
- **WO.2** [D model.rs:1423-1431] If `world` is a colony, `owner` must be present; code `colony_requires_owner`. An unowned colony is invalid; the author removes the colony first with `RemoveColony`.
- **WO.3** [P] If `world` has any construction or hull job whose `owner` differs from the new owner, reject; code `foreign_jobs_on_world`. The author removes those jobs first with `RemoveEntity` (free, X7). *Rejected alternative 1:* apply capture semantics, cancelling the jobs with a half refund (rules L129). That pays a refund on a creator override, which X7 forbids. *Rejected alternative 2:* leave the jobs. They would later complete into hulls owned by a civilization that no longer owns the shipyard. Open question Q9.

Effects:
- **WO.4** [D command.rs:321-327] Set `world.owner`. If `world` is a colony, set `colony.owner` to the same value (the validator requires equality, `model.rs:1426`). The hub, inventory, deposits and facilities are untouched (rules L129 by analogy with capture).
- **WO.5** [D rules L167] If the world becomes owned, remove every `settlement_claims` record on it: "A world becoming owned through a creator command invalidates all outstanding claims before arbitration."
- **WO.6** [D rules L177] For every route with `world` as source or destination, set `suspended = true` if the current owner of either endpoint now differs from the route's recorded participant for that endpoint. A route is never un-suspended by this operation.
- **WO.7** [D rules L218, rules L210] Remove the `occupations` record on `world`, if any. An ownership change is an invalidating condition.
- **WO.8** [P] No relation counter changes: this is not a capture, so `colony_captured_from_it` does not move. *Rejected alternative:* apply the -30 capture counter. rules L210 forbids presenting a creator override as a gameplay event.
- **WO.9** [D X17] Recompute status for the old and new owner if a colony moved.

Later phases:
- **WO.10** [D rules L177-184] In-flight shipments keep their manifests; phase 3 resolves them against the new owner. Phase 6 updates observations; phase 9 skips suspended routes.

### 2.10 `CreateColony`

Preconditions:
- **CC.1** [D model.rs:1423-1431] `world` and `owner` exist and `world.owner == owner`; code `colony_owner`. If the world is unowned, the batch must set its owner with an earlier `SetWorldOwner`.
- **CC.2** [P] Rejecting CC.1 is preferred to setting the owner implicitly. *Rejected alternative:* `CreateColony` sets `world.owner`. The operation would then also carry every WO.5 to WO.7 side effect without naming them in the preview.
- **CC.3** [D model.rs:454-460] `world` has no colony yet (the key is the world).
- **CC.4** [P] `world` has no open settlement claim; with CC.1 this always holds, because claims exist only on unowned worlds (WO.5).

Effects:
- **CC.5** [P] Insert `LivingColonyV2 { world, owner, hub }` with `energy_next_due = A + 10`, `ore_next_due = A + 100`, `fallback_next_due = A + 100`, and empty `blocked_reasons`. Grounds: rules L95 gives these periods at genesis and applies the genesis rule to creator-created built facilities; the hub is not in the facility table, so extending the rule to it is a decision. *Rejected alternative:* the settlement rule's clocks. rules L165 fixes the settled inventory (20/20/0) but not the clocks, so it gives no better anchor. The inventory is not touched here (world-local, rules L99).
- **CC.6** [D X11, X9 rule 5] The world's existing facilities must fit six slots and each `max_per_colony`; code `colony_slots`.
- **CC.7** [P] No `adjacent_rival_settlement` counter changes (same reasoning as WO.8).
- **CC.8** [D rules L220, X17] Recompute the owner's status; a dormant owner becomes active ("The creator can restore a colony").

Later phases:
- **CC.9** [D rules L117, rules L95] Phase 5 runs hub energy at `A+10`, ore and fallback at `A+100`; phase 8 treats the world as an owned colony.

### 2.11 `RemoveColony`

Preconditions:
- **RC.1** [D simulation.rs:835-850] The world has a colony, else `MissingTarget`.
- **RC.2** [P] The world has no construction job and no hull job; code `colony_has_jobs`. The author removes them first (`RemoveEntity`, free). *Rejected alternative:* cancel them with a half refund, as capture does; that pays a refund on a creator override (X7). The operation has no cascade field, so it cannot name what it would remove, and rules L256 rejects ambiguous deletion.

Effects:
- **RC.3** [D rules L101, model.rs:438-452] Remove the colony record and its hub. The world keeps its owner, inventory, deposits and facilities; noncolonized industry is allowed.
- **RC.4** [D rules L218] Remove the `occupations` record on the world; occupation needs an enemy colony.
- **RC.5** [D X17] Recompute the owner's status; losing the last colony makes it dormant.

Later phases:
- **RC.6** [D rules L101, rules L220] Phase 8 no longer offers construction there. Facilities keep running in phase 5 (they have their own clocks); nothing requires a colony for a recipe.

### 2.12 `SetRelationBase`

Preconditions:
- **RB.1** [D model.rs:1587-1602] `from` and `to` exist, differ, and `base_disposition` is in `[-100, 100]`.

Effects:
- **RB.2** [D rules L190, rules L200] Set the directed record's `base_disposition`. Reason counters and partial delivery remainders are untouched; base disposition never decays.
- **RB.3** [P] If no record exists for `(from, to)`, insert one with all counters and remainders zero. *Rejected alternative:* require the record to exist. `CreateCivilization` creates no relation records, so the first base disposition between two new civilizations would be unsettable. Whether an absent record and an all-zero record are the same state is open question Q10.
- **RB.4** [P] The operation never removes a record, even when it sets base 0 on an all-zero record.

Later phases:
- **RB.5** [D rules L91, rules L202-206] Phase 7, on multiples of 100, uses the new total for agreement and war eligibility.

### 2.13 `SetAgreement`

Preconditions:
- **SA.1** [D model.rs:1631-1646] Both participants exist, `participant_a < participant_b` by identity, `start_tick < end_tick`.
- **SA.2** [P] `end_tick > A`; code `agreement_window`. A record that ends at or before `A` would be expired by phase 2 of the same boundary (running) or would be stale on arrival (paused). *Rejected alternative:* accept it. It would produce an `agreement_expired` without the agreement ever having been in force.
- **SA.3** [P] For `trade` and `nonaggression`: reject if a war or a truce record between the pair has `end_tick > A`; code `agreement_blocked_by_war`. rules L202 says war and truce block new trade and nonaggression. The author ends the war first with `ForcePeace` and the truce with `RemoveAgreement`. *Rejected alternative:* remove the war or truce implicitly. That turns an agreement edit into an undeclared forced peace, which rules L210 requires to be identified as such.
- **SA.4** [P] For `truce`: reject if a war between the pair has `end_tick > A`; code `truce_during_war`. Truce is what a war becomes (rules L208); overlapping both is meaningless. *Rejected alternative:* allow it and let phase 2 reconcile.
- **SA.5** [P] For `truce`: reject if a `trade` or `nonaggression` record between the pair has `end_tick > A`; code `agreement_blocked_by_truce`. This mirrors SA.3 and the "author removes first" pattern of WO.3 and RC.2; rules L202 says a truce blocks *new* trade and nonaggression, not that it removes existing ones. *Rejected alternative:* remove them implicitly as part of setting the truce. That is the same undeclared cascade SA.3 refuses in the other direction.

Effects:
- **SA.6** [D model.rs:910-920, command.rs:364-376] Insert or replace the record keyed `(participant_a, participant_b, kind)`.

Later phases:
- **SA.7** [D rules L85, rules L173, rules L202] Phase 2 expires it at `end_tick`; phase 7 renews only through mutual eligibility; phase 9 checks a trade agreement at each cross-civilization dispatch.

### 2.14 `RemoveAgreement`

- **RA.1** [D simulation.rs:835-850] The record `(participant_a, participant_b, kind)` exists, else `MissingTarget`.
- **RA.2** [D command.rs:377-385] Remove it.
- **RA.3** [D rules L177, rules L182] Trade routes between the pair are not suspended (suspension is about ownership); phase 9 simply stops dispatching them. Cargo already sent is unaffected: "Treaty expiry alone does not invalidate cargo already sent."
- **RA.4** [P] No `agreement_expired` event: the record did not reach its boundary (X16). *Rejected alternative:* emit it; that misreports a creator removal as a lifecycle event.

### 2.15 `ForceWar`

Preconditions:
- **FW.1** [D model.rs:1647-1661] Both participants exist, `participant_a < participant_b`, `declarer` is one of them, `start_tick < end_tick`.
- **FW.2** [P] `end_tick > A`; code `war_window`; same reasoning as SA.2.
- **FW.3** [P] `start_tick <= A`; code `war_window`. A forced war is in force from its application. *Rejected alternative:* allow a scheduled future war. `LivingWarV2` has no "pending" state and phase 4 would treat the record as current, so a future start is not representable faithfully. Open question Q11.

Effects:
- **FW.4** [D rules L210, civ L1435] Remove every `trade`, `nonaggression` and `truce` record between the pair: "atomically resolve incompatible agreements".
- **FW.5** [P] Insert the war, replacing any existing war record for the pair (the key is the pair). *Rejected alternative:* reject when a war exists. Replacement is how a creator extends or re-attributes a war.
- **FW.6** [P] No `war_declared_against_it` counter change. *Rejected alternative:* apply the victim's -30. rules L210 forbids pretending the civilizations negotiated; the creator sets hostility through `SetRelationBase`. Open question Q12.
- **FW.7** [P] Occupation and settlement records are not touched; phase 4 evaluates them against the new war.

Later phases:
- **FW.8** [D rules L183, rules L208, rules L214] Phase 3 returns in-flight friendly cargo between the pair once; phase 4 fights on the next multiple of 10 (at `A` itself if `A` is a multiple of 10 and the envelope is running); phase 2 turns the war into a 600-tick truce at `end_tick`. Newly issued attack orders still depart no earlier than `A + 1` (SFO.4).

### 2.16 `ForcePeace`

Preconditions:
- **FP.1** [P] A war record for the pair exists, else `MissingTarget` with reference `war`. *Rejected alternative:* accept as a no-op (X14). A forced peace with nothing to end is almost certainly an authoring mistake, and the preview should say so.

Effects:
- **FP.2** [D rules L210, civ L1435] Remove the war record. Because phase 1 runs before phase 4, no combat happens at `A` ("stops new damage at its application boundary").
- **FP.3** [D rules L210] Remove every `occupations` record whose world is owned by one participant and whose claimant is the other ("resets occupation progress").
- **FP.4** [P] No truce is created. *Rejected alternative:* create a 600-tick truce as a natural war end does (rules L208). A truce blocks new trade and nonaggression (rules L202), which is the opposite of what a forced peace asks for; the author can add one with `SetAgreement` in the same batch. Open question Q13.
- **FP.5** [D rules L184] Shipments already `returning` keep returning; cargo never bounces.

Later phases:
- **FP.6** [D rules L202-206] Phase 7 may form agreements or re-declare war at the next diplomacy boundary if the pair is eligible.

### 2.17 `SetFleetOrder`

Preconditions:
- **SFO.1** [D model.rs:1491-1498] `fleet` exists; `target_world`, if present, exists; every lane in `lane_path` exists.
- **SFO.2** [P] `kind = null` requires `target_world = null` and `lane_path = []`; code `idle_order_shape`. The null order is idle (`command.rs:407-419`); extra fields would be unhashed noise that changes the revision identity.
- **SFO.3** [P] Shape by kind; code `order_shape`:
  - `move`, `settle`, `attack`, `defend`, `return` require `target_world`;
  - `explore` requires `target_world = null` and `lane_path = []` (phase 9 chooses, rules L159).
- **SFO.4** [D rules L92, civ L579] `not_before_tick >= A + 1`; code `order_not_before`.
- **SFO.5** [D rules L145] `lane_path` is a connected sequence of lanes from the system of the fleet's current world (the leg's destination world if travelling) to the system of `target_world`; empty exactly when both are in the same system; code `order_path`.
- **SFO.6** [D rules L66] If any hull job targets the fleet and `kind` is anything but `null` or `defend`, reject with `ReservationConflict`.
- **SFO.7** [P] Kind-specific legality; code `order_legality`:
  - `settle`: the fleet holds at least one ark hull, and `target_world` is unowned (rules L165);
  - `attack`: a war between the fleet owner and `target_world.owner` has `end_tick > A` (rules L214);
  - `defend` and `return`: `target_world` is a colony of the fleet owner (rules L147, rules L216).
  - *Rejected alternative:* no legality check for creator orders. A creator order that phase 9 can never execute would sit idle forever with no reason recorded. The omniscient creator changes world state first, in the same batch, if it wants the order to be legal.
- **SFO.8** [P] `lane_path` must equal the canonical path of rules L145: minimum total travel ticks, ties broken by the lexicographically smallest lane-identity path; code `order_canonical_path`. *Rejected alternative:* accept any connected path. rules L145 defines one path for all fleets; two paths for the same trip would diverge in arrival time from the rule every civilization order obeys. Open question Q14 (this forbids scenic creator routes).

Effects:
- **SFO.9** [D model.rs:646-663] Set `fleet.order`. A travelling fleet keeps its current leg unchanged: "Movement on an already departed leg retains its endpoints and duration" (rules L149).

Reservation and refund:
- **SFO.10** [P] No energy is charged or granted now. Phase 9 charges launch energy at departure under rules L147 exactly as for any order, because `LivingFleetOrderV2` carries no funding flag. A creator who wants a free launch grants energy with `SetWorldInventory` in the same batch. *Rejected alternative:* creator orders depart free. Phase 9 cannot tell them apart without a schema change (K3), and civ L1386-1394 only asserts that the submission itself charges nothing, which this rule keeps. Open question Q2.

Later phases:
- **SFO.11** [D rules L92, rules L149] Phase 9 departs no earlier than `not_before_tick`, charges fuel, and at each waypoint revalidates the remaining path, holding with No route when disconnected.

### 2.18 `SetShipmentDisposition`

Preconditions:
- **SD.1** [D simulation.rs:835-850] The shipment exists.
- **SD.2** [P] Only one transition is legal, `waiting_for_storage -> returning`; code `shipment_transition`. Every other change is rejected:
  - `outbound -> returning` changes an in-flight duration, which rules L175 forbids, and the operation cannot supply the new arrival tick;
  - `outbound -> waiting_for_storage` claims an arrival that has not happened;
  - `returning -> anything` is a bounce, which rules L184 forbids.
  - *Rejected alternative:* accept any change and let phase 3 cope. Phase 3 would then act on a disposition whose ticks contradict it.
- **SD.3** [D X14] Setting the current disposition is accepted as a no-op.

Effects:
- **SD.4** [P] For `waiting_for_storage -> returning`: set `departure_tick = A` and `arrival_tick = A + (old arrival_tick - old departure_tick)`: "return once over the same duration, at no additional resource cost" (rules L183). The departure/arrival rewrite is required because the record has no separate return fields. *Rejected alternative:* keep the old ticks. Phase 3 would find the return already overdue at `A`.
- **SD.5** [D rules L186] Discarding cargo "with a loss receipt" is not this operation (the enum has no discarded state); it is `RemoveEntity` on the shipment (RE rules in section 3). See K4.

Later phases:
- **SD.6** [D rules L184] Phase 3 unloads the returning cargo at its source at the new `arrival_tick`.

### 2.19 `RemoveEntity`

Section 3 is its full rule set.

## 3. Removal cascades (`RemoveEntity`)

### 3.1 Reference classes

`LivingCascadeDispositionV2::RemoveListed.dependents` is a list of entity targets (`command.rs:95-98`), but colonies, relations, agreements, wars, observations, settlement claims and occupations have no entity identity and so cannot be listed (rules L469). Every reference to a removed entity is therefore in exactly one of three classes.

- **RE.1** [P] **Class a, identity-bearing dependents.** Records with an entity identity that cannot exist without the target. They must be listed. The table below defines them per target kind, and the relation is transitive. *Rejected alternative:* let every dependent be implicit. rules L256 requires "an explicit reviewed cascade" and rejects "ambiguous deletion".
- **RE.2** [P] **Class b, keyed records.** Records without an identity that reference the target. They are removed implicitly with it. They cannot be named on the wire, so this is the only deterministic option short of a schema change.
- **RE.3** [P] **Class c, scalar references.** Optional fields that point at the target. They are cleared, not removed:
  - `fleet.order` becomes `null` when its `target_world` or any `lane_path` lane is removed (rules L304: "valid target removal retires affected pending orders with reasons");
  - `observation.owner` becomes `null` when the observed owner is removed.

### 3.2 Dependents by target kind

| Target | Class a (must be listed, transitive) | Class b (implicit) | Class c (cleared) |
| --- | --- | --- | --- |
| System | stars, worlds and lanes in or touching the system, and their class a | none directly | none directly |
| Star | none | none | none |
| World | its deposits, facilities, construction jobs, hull jobs; fleets docked at it or travelling with it as origin or destination; routes and shipments with it as either endpoint | its colony, observations of it, settlement claims on it, its occupation | orders targeting it |
| Lane | hazards on it; routes whose endpoint systems are joined by no other lane | none | orders whose path contains it |
| Civilization | worlds it owns (unless released earlier in the batch), construction and hull jobs it owns, fleets it owns, routes naming it as either owner, shipments naming it as either party | relations from and to it, agreements and wars naming it, its observations, its settlement claims, occupations it claims | `observation.owner` of other civilizations' observations |
| Deposit | none | none | none |
| Facility | if it is the world's last `hull_assembly` facility, the hull jobs on its world | none | none |
| Construction job | none | none | none |
| Hull job | none | none | none |
| Fleet | hull jobs targeting it | none | none |
| Hull | none | none | none |
| Route | none (shipments hold no route reference, `model.rs:763-768`) | none | none |
| Shipment | none | none | none |
| Hazard | none | none | none |

- **RE.4** [D rules L149] Travelling fleets whose leg touches a removed world are class a. rules L149 requires the batch to "explicitly resolve affected travellers", and no operation relocates a fleet, so removal is the only explicit resolution available (K6).
- **RE.5** [D rules L186] Shipments touching a removed world are class a. Listing one is the "recorded loss"; the "or reroutes it" branch of L186 has no operation (K6).
- **RE.6** [D rules L149] Removing a lane never touches a travelling fleet: "Movement on an already departed leg retains its endpoints and duration." Only orders are cleared (class c).
- **RE.7** [P] A world owned by a removed civilization is class a unless an earlier operation of the batch made it unowned (`RemoveColony` then `SetWorldOwner(null)`). *Rejected alternative:* clear `world.owner` implicitly. That would also silently remove the colony (class b) and change another record's meaning without naming it, which is the ambiguous deletion rules L256 rejects.
- **RE.8** [D model.rs:677-678, rules L141] A fleet's hulls are contained, not referenced; they go with the fleet and must not be listed.
- **RE.9** [P] Removing the last hull of a fleet rejects with `empty_fleet` unless the fleet is removed instead. Same reasoning as FL.3.

### 3.3 Validation of the cascade

- **RE.10** [D command.rs:795-816] Structural checks already present: a `RemoveListed` list is non-empty and has no repeats.
- **RE.11** [P] `target` and every listed dependent must be `Existing`; code `cascade_local`. Removing something the same batch created is a no-op the author should not write, and a `Local` cannot be sorted by identity before the revision exists (RE.12). *Rejected alternative:* allow `Local`; it complicates the ordering rule for no authoring value.
- **RE.12** [P] Listed dependents must be sorted strictly ascending by identity; this is a structural check in `LivingCommandV2::validate`, code `cascade_order`. Same reasoning as X19 and `command.rs:22-27`: one set, one byte encoding. *Rejected alternative:* accept any order and sort internally. Two encodings of one cascade would be two revision identities.
- **RE.13** [D auth L159, command.rs:88-101] Under `RemoveListed`, the listed set must equal the transitive class a closure of the target, computed on the projection at this operation's position (X4):
  - a missing member rejects with `BlockingDependencies`, naming the lowest missing identity in schema field order then identity order;
  - an extra member rejects with `UnexpectedDependent`.
  - The plan's words are "complete exact cascade dispositions".
- **RE.14** [D command.rs:100-101] `RejectIfReferenced` rejects when the target has any class a, b or c reference ("Refuse the removal if the target has any dependent at all"); otherwise it removes the target alone. Rejection code `BlockingDependencies` with the lowest identity-bearing reference, or `OperationRule { rule: "keyed_reference" }` when only class b or c references exist. Note the consequence: once phase 6 runs, nearly every world has been observed by someone and nearly every civilization appears as some `observation.owner`, so `RejectIfReferenced` on a world or civilization will almost always reject. Whether observations should count as blocking references is open question Q19.

### 3.4 Effects

- **RE.15** [P] Remove the target and its class a closure, then class b, then apply class c, as one set in X19 order. The result does not depend on the listed order.
- **RE.16** [D rules L127, X7] No refund for removed facilities or jobs; no inventory moves except that a removed world's inventory goes with it.
- **RE.17** [D rules L186] A removed shipment is lost cargo: its units reach no inventory, and no trade or aid credit is added (rules L200: "Destroyed/lost freight does not count").
- **RE.18** [D rules L66] A removed hull job releases its fleet and capacity reservations with it.
- **RE.19** [D X17] Status is recomputed for every civilization whose colony count changed.
- **RE.20** [P] A removed active hazard produces no `hazard_ended` event (X16); phase 2 simply stops seeing it.
- **RE.21** [D rules L256] A civilization is normally made dormant by play, not deleted. Nothing here forbids deleting one, but the class a table makes it a large explicit list by design.

### 3.5 Later phases

- **RE.22** [D rules L149, rules L165] Phase 9 re-plans nothing for a fleet whose order was cleared; it is idle. Phase 4 drops a settlement claimant whose ark was removed at the close boundary ("claimants still present and eligible").

## 4. Test cases (TDD order)

Each test states the expectation that must fail first. Bytes-unchanged means `authority_bytes()` (state, revision log, cursor, allocator mark) is identical before and after a rejection.

**Stage 1: structure and plumbing**
1. `every_operation_is_now_applied`: the existing `operations_this_slice_does_not_apply_are_refused_rather_than_skipped` is replaced; no operation returns `UnsupportedOperation`.
2. `cascade_dependents_must_be_existing_and_sorted`: `Local` target, `Local` dependent, and unsorted dependents are rejected in `LivingCommandV2::validate` (RE.11, RE.12).
3. `a_target_of_the_wrong_kind_is_a_missing_target`: a facility identity passed as `fleet` rejects `MissingTarget` (X13).
4. `a_rule_failure_rejects_the_whole_batch_and_changes_no_bytes`: a valid op followed by an invalid op; bytes unchanged, sequence not consumed (X3).

**Stage 2: capacity with reservations (X9), each at cap accepted and cap+1 rejected with `Capacity`**
5. `facilities_plus_construction_jobs_cap_at_2048`.
6. `hulls_plus_hull_jobs_cap_at_2048`: the rules L297 fixture; with 2047 hulls, the first `CreateHullJob` is accepted and a second in the next envelope is rejected, with bytes unchanged.
7. `fleets_plus_new_fleet_reservations_cap_at_256`: the rules L296 fixture, creator path.
8. `per_fleet_hulls_plus_targeting_jobs_cap_at_16`.
9. `colony_slots_and_role_limits_count_queued_jobs`: six slots, second shipyard, second battery.
10. `hull_jobs_per_world_cap_at_shipyard_concurrency`.

**Stage 3: creations, happy path plus each coded rejection**
11. `create_facility_clock_status_hit_points_and_paid_alloy` (CF.3 to CF.8).
12. `create_construction_job_requires_colony_duration_window_and_empty_queue` (CJ.2 to CJ.6).
13. `create_hull_job_requires_shipyard_and_a_docked_owned_target` (HJ.2 to HJ.6).
14. `a_batch_creates_a_fleet_then_a_hull_job_for_it_by_local_reference` (HJ.7).
15. `a_fleet_created_without_a_hull_is_rejected` (FL.3).
16. `create_hull_rejects_a_travelling_fleet_and_sets_the_repair_clock` (HU.2, HU.5).
17. `create_route_enforces_batch_cadence_reserve_connection_and_permission` (RT.2 to RT.6).
18. `create_shipment_timing_depends_on_disposition_and_deducts_nothing` (SH.2, SH.3, SH.6).
19. `create_hazard_must_start_where_phase_two_can_see_it`: running `start_tick = A` accepted, paused `start_tick = A` rejected (HZ.2).

**Stage 4: ownership and colonies**
20. `set_world_owner_clears_claims_suspends_routes_and_resets_occupation` (WO.5 to WO.7).
21. `an_owned_colony_cannot_be_made_unowned_without_removing_the_colony` (WO.2).
22. `set_world_owner_rejects_foreign_jobs_on_the_world` (WO.3).
23. `set_world_owner_then_create_colony_in_one_batch` (X4, CC.1, CC.5 clocks).
24. `create_colony_on_an_overfull_world_is_rejected` (CC.6).
25. `remove_colony_keeps_owner_and_facilities_and_requires_no_jobs` (RC.2, RC.3).
26. `status_follows_colony_count_only_for_touched_civilizations` (X17): removing the last colony makes the owner dormant; an untouched `Active` civilization with no colony stays `Active`.

**Stage 5: diplomacy**
27. `set_relation_base_creates_a_zero_counter_record_and_never_decays` (RB.2, RB.3).
28. `set_agreement_is_blocked_by_war_and_truce_and_a_truce_is_blocked_by_trade` (SA.3 to SA.5).
29. `force_war_removes_incompatible_agreements_atomically_and_replaces_a_war` (FW.4, FW.5).
30. `force_peace_prevents_same_boundary_combat_and_resets_pair_occupation`: war, enemy escort at a colony, `A` a multiple of 10; no damage at `A` (FP.2, FP.3). Needs phase 4; until then assert state only and mark the combat half `#[ignore]` with a reason.
31. `force_peace_without_a_war_is_a_missing_target` (FP.1).
32. `remove_agreement_leaves_in_flight_cargo_alone` (RA.3).

**Stage 6: orders and dispositions**
33. `set_fleet_order_not_before_is_at_least_the_next_boundary` (SFO.4), in both modes.
34. `a_reserved_fleet_rejects_a_departing_order_with_bytes_unchanged` (SFO.6; rules L298).
35. `order_path_must_be_connected_and_canonical` (SFO.5 `order_path`, SFO.8 `order_canonical_path`), including an equal-duration tie resolved by lane identity independent of insertion order.
36. `order_legality_by_kind` (SFO.7).
37. `a_travelling_fleet_keeps_its_leg_when_reordered` (SFO.9).
38. `only_waiting_to_returning_is_a_legal_disposition_change` (SD.2, SD.4).

**Stage 7: cascades**
39. `incomplete_world_removal_changes_nothing`: civ L1408-1416; `BlockingDependencies` names the lowest missing identity (RE.13).
40. `over_listed_removal_is_rejected` (RE.13, `UnexpectedDependent`).
41. `complete_world_removal_leaves_no_dangling_reference_and_clears_orders` (RE.3, RE.15; rules L304).
42. `listed_order_does_not_change_the_result`: two batches with the same set; one is rejected by RE.12 and the sorted one is accepted. With RE.12 declined, instead assert identical post-state digests.
43. `lane_removal_keeps_travelling_fleets_and_retires_paths` (RE.6).
44. `civilization_removal_requires_owned_worlds_released_or_listed` (RE.7).
45. `reject_if_referenced_refuses_keyed_references_too` (RE.14).
46. `removing_a_shipment_adds_no_inventory_and_no_credit` (RE.17).
47. `removing_a_hull_job_releases_its_fleet_for_departure` (RE.18 then SFO.6 accepted).

**Stage 8: determinism and replay**
48. Extend `two_authorities_fed_the_same_history_agree_byte_for_byte` with one batch using all 19 operations.
49. `a_queued_operation_whose_rule_stops_holding_faults_as_replay`: submit `CreateColony` against a projection, then corrupt the committed owner (unit test, as `simulation.rs:907-912` does); the boundary faults with `Replay` and commits nothing (X2).
50. `paused_and_running_application_agree_after_the_same_boundary`: the same batch applied paused at `t` and running at `t+1` differs only where a rule is written against `A`.

## 5. Contradictions and gaps found

- **K1. Events cannot name anything.** `LivingEventPayloadV2` is `{ordinal, provenance, kind}` and every `LivingEventKindV2` variant is a unit variant (`receipt.rs:80-150`). The following all assume events carry data:
  - rules L127: a discard "with a receipt";
  - rules L250: accepted and rejected intents record actor, target, score and reason;
  - rules L304: removal "retires affected pending orders with reasons";
  - civ L484: `LivingEventKindV2::Refunded { .. }` with fields;
  - civ L791: `ClaimResolved` carrying a reason;
  - civ L1132: `AutonomousIntentRejected` with a structured reason;
  - civ L1241: every decision event carries actor, target, action, tier, score, observation tick and reason;
  - civ L1423: `creator_events_name_all_retired_dependents(&receipt)`.

  civ L101 also claims the frozen kind list includes intent, repair, observation, retreat and capacity-rejection variants. `receipt.rs` has none of them, and civ L101 forbids the civilizations plan from adding any. This blocks the civilizations plan as written, not only this addendum. (X16, Q1)
- **K2. Shipments do not record whether they are trade or aid.** `LivingShipmentV2` (`model.rs:769-792`) has no route-kind field and deliberately no route reference. rules L194-195 credit aid at +10 and trade at +1 per 100 delivered units, so phase 3 cannot choose the counter at delivery. `CreateShipment` cannot express it either. (Q3)
- **K3. Fleet orders do not record who pays.** rules L256 distinguishes "a direct creator grant from a resource-funded civilization order". `LivingFleetOrderV2` (`model.rs:650-663`) has no provenance or funding field, so phase 9 cannot apply rules L147 differently. civ L1386-1394 expects creator operations to leave actor resources untouched. (SFO.10, Q2)
- **K4. The discard path is not what the spec describes.** rules L186 says the creator "can explicitly discard it with a loss receipt". `LivingShipmentDispositionV2` has no discarded state, so discard is only `RemoveEntity`, whose event (under X16) is a plain `creator_intervention`. `SetShipmentDisposition` cannot carry the new arrival tick a return needs, which leaves it one legal transition (SD.2). Separately, rules L177 and L179 call the shipment manifest immutable and list its departure and arrival ticks as part of it, yet `LivingShipmentV2` has no return-leg fields, so both SD.4 and phase 3's own war return (rules L183) must rewrite those two ticks. Either the manifest's tick fields are not immutable, or the record needs return fields (a format break). (SD.4, Q18)
- **K5. The state validator is weaker than the rules it cites.** `model.rs:1539-1550` checks route batch and cadence only for nonzero, not rules L171's `batch <= 10`, `cadence == 10`, `reserve >= 20`. Facility HP is unchecked (`DP#8`). Reservation capacity is unchecked (`DP#7`, `model.rs:1216-1229`). Civilization status is never checked against colony count, though rules L220 ties them. This addendum closes the creator path only (RT.2, CF.3, X10, X17). A decoded archive or genesis can still carry these states.
- **K6. The 28-operation schema cannot express several rules-spec edits.** The civilizations plan's list (civ L103) is different: it has 31 variants, including `SplitFleet`, `MergeFleets`, `EditRoute`, `SetRouteSuspended`, `CancelConstructionJob`, `ReorderConstructionJob`, `EditFleet` and `SetFacilityEnabled`. Its rejection names (`ReservationConflict`, `Capacity`, `BlockingDependencies`, civ L437, L422, L1413) do not exist in `simulation.rs`. Consequences here:
  - no operation relocates a fleet or reroutes cargo, so rules L149 and L186's "or reroutes it" reduce to removal (RE.4, RE.5);
  - no operation resumes a suspended route; rules L177's "newly validated adjustment" is only remove-and-recreate, which issues a new route identity;
  - no operation splits or merges a fleet, which rules L141 and L246 assume a civilization can do. That path will need either new operations or a non-creator intent path.
- **K7. `CreateCivilization` accepts any status.** It is already applied and outside the 19, but an `Active` civilization with zero colonies validates, so X17's "recompute only when touched" preserves a state rules L220 says cannot arise in play. (Q4)
- **K8. The hull catalog has no role.** `LivingHullDefinitionV2` (`catalog.rs:349-362`) has cost, build ticks, HP and damage, but no ark, escort or scout role. rules L165 (only arks settle), L206 (escorts count toward strength) and L216 (unescorted noncombatants retreat) all need a role. SFO.7 can only identify an ark by the slug `colony_ark` or by `damage == 0`, and scouts also deal 0 damage. (Q15)
- **K9. Construction queue length is unspecified.** rules L101 says "one building queue"; the hull queue's concurrency is in the catalog (`concurrent_jobs`), but the building queue's length is not. civ L23 and L443 ("full building queue") assume a finite length. (CJ.5, Q5)
- **K10. `CreateFacility.next_due` and `CreateRoute.next_due` are fields rules L95 says a creator does not choose.** L95 applies the genesis clock rule to creator-created facilities "unless a validated import supplies canonical clocks". (CF.4, RT.6, Q6)
- **K11. Absent and all-zero relation records are two encodings of one meaning.** The model does not require a record per ordered pair, so two histories reaching the same relations can digest differently depending on whether a zero record was ever written. (RB.3, Q10)
- **K12. The whole addendum rests on unratified field names.** `DP#2` is still open. Every rule above names `command.rs` fields; overruling one renames a rule but does not change its logic.

## 6. Open questions for Donald

1. **Q1 (events, K1).** Option A: one `creator_intervention` per revision, with details recoverable from the command (recommended for this slice). Option B: add effect kinds now. Option C: give events a subject and reason, which re-derives every receipt vector and is what the civilizations plan actually assumes. The civilizations plan cannot meet its own tests without C or an equivalent.
2. **Q2 (fuel, K3).** Do creator fleet orders pay launch energy at dispatch (recommended, no schema change), or depart free (needs a funding field on `LivingFleetOrderV2`, a format break)?
3. **Q3 (freight credit, K2).** Add a route-kind field to `LivingShipmentV2` (format break), derive aid versus trade from something else, or drop the distinction at delivery?
4. **Q4 (status, K7).** Recompute status only for touched civilizations (X17, recommended), make it a validator invariant, or never touch it in phase 1?
5. **Q5 (queue, K9).** Is the building queue length 1 (CJ.5, recommended)? If longer, jobs need a queue-wait rule and a length constant, ideally in the catalog next to `concurrent_jobs`.
6. **Q6 (clocks, K10).** Must creator-supplied `next_due` equal `A + period` (CF.4, RT.6, recommended), or may it be any future boundary?
7. **Q7 (empty fleets).** Reject a fleet with no hulls at batch end (FL.3, RE.9, recommended), or allow it?
8. **Q8 (hazards).** Require a hazard to start where phase 2 can observe it (HZ.2, recommended), or allow one already in progress with no start event?
9. **Q9 (jobs on transferred worlds).** Reject `SetWorldOwner` over foreign jobs (WO.3, recommended), cancel them with a refund, or leave them?
10. **Q10 (relations, K11).** Is an absent relation equal to an all-zero one? If so, should the canonical form forbid all-zero records, or require every ordered pair to exist?
11. **Q11 (future wars).** Forbid a forced war that starts after `A` (FW.3, recommended), or add a scheduled-war notion?
12. **Q12 (forced-war penalty).** Should `ForceWar` apply the victim's -30 counter? Recommended no (FW.6).
13. **Q13 (forced-peace truce).** Should `ForcePeace` create the 600-tick truce a natural war end creates? Recommended no (FP.4).
14. **Q14 (creator paths).** Must creator order paths be the canonical shortest path (SFO.8, recommended), or may the creator choose any connected path?
15. **Q15 (hull roles, K8).** Add a role to `LivingHullDefinitionV2` (a catalog format break that changes `catalog_hash`), or identify arks and escorts by slug?
16. **Q16 (enable order).** Enable all 19 operations now, while phases 2 to 9 are empty hooks and every created object is inert (recommended: the state rules are testable now), or enable each only alongside the phase that consumes it?
17. **Q17 (missing operations, K6).** Should the operation schema gain fleet split and merge, route adjustment and resume, and a relocation or reroute operation before `DP#2` is ratified, while adding them is still free?
18. **Q18 (manifest ticks, K4).** Are a shipment's departure and arrival ticks allowed to change on a return (SD.4 and phase 3 both need it), or should `LivingShipmentV2` gain return-leg fields?
19. **Q19 (observations as references, RE.14).** Should `RejectIfReferenced` ignore observations (and perhaps all class b and c references), counting only gameplay-bearing records?
