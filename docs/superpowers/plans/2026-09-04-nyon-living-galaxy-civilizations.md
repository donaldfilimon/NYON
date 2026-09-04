# NYON Living Galaxy Civilizations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the complete deterministic Living Galaxy V2 civilization loop: local-resource economy, construction and repair, freight, fleets and intelligence, autonomous settlement, explainable strategy, diplomacy, simultaneous combat, occupation, and reversible dormancy.

**Architecture:** The authority plan owns Living V2 canonical types, identity, command/history, wire formats, receipts, and the atomic ten-phase tick; this plan supplies platform-free phase functions under `nyon-workshop-core::living` and deterministic fixtures that operate only through that authority. Every phase mutates the authority-owned candidate through `LivingStepContextV2`, emits schema-frozen events through `LivingStepContextV2::emit`, and returns a typed fault so the outer authority can commit the whole boundary or preserve the last valid state.

**Tech Stack:** Rust nightly-2026-09-01, edition 2024, `serde`, `serde_json`, `sha2`, `thiserror`, ordered `Vec<T>` canonical collections, checked integer arithmetic, native and `wasm32-unknown-unknown` pure-core targets.

**Spec:** `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` (with product boundaries in `2026-09-04-nyon-living-galaxy-design.md`, fixture requirements in `2026-09-04-nyon-living-galaxy-qualification.md`, and starter witnesses in `2026-09-04-nyon-living-galaxy-experience.md`)

## Global Constraints

- Execute `docs/superpowers/plans/2026-09-04-nyon-living-galaxy-authority.md` first. Do not fork, replace, or reorder its canonical schema, wire identities, queue/history rules, or ten tick phases.
- Keep Classic RulesV1 and WorkshopV1 types, phase order, bytes, limits, behavior, and golden digests unchanged.
- Keep `nyon-workshop-core` `#![forbid(unsafe_code)]`, platform-free, offline, and independent of winit, wgpu, Tokio, Web APIs, wall-clock time, audio, presentation preferences, and advisory output.
- Run at exactly 10 authoritative ticks per second. A step from completed tick `t` evaluates boundary `T=t+1` and commits every state/event/counter change or none.
- Preserve the phase order: creator commands; lifecycle/hazard boundaries; travel/freight arrivals; retreat/combat/occupation/claims; construction/economy/repair; observations; diplomacy every 100 ticks; strategy every 50 ticks; fleet/freight dispatch; receipts/counters/hash/commit.
- Use only checked integer arithmetic, stable ID traversal, schema-defined ordinal tables, sorted logical maps, and domain-separated authority identity. Never use Rust enum layout, hash-map insertion order, rendering order, thread scheduling, or floating point as authority.
- Enforce the hard caps: 16 civilizations; 64 systems; 128 stars; 512 worlds; 256 lanes; 1,024 deposits; 2,048 facilities; 2,048 routes; 4,096 shipments; 256 fleets; 2,048 total hulls; 16 hulls per fleet; 128 hazards; 64 branches; 10,000 creator revisions.
- Use exactly energy, ore, and alloy. Each colony inventory caps each resource at 10,000. A recipe consumes all declared inputs and produces all outputs atomically, or consumes nothing and records every current blocked reason.
- A colony has one persistent hub, six facility slots, one construction queue, and at most one shipyard with one hull queue. Outstanding jobs reserve global capacity and their destination slot/fleet until completion or cancellation.
- Civilizations pay alloy up front. Scrapping/cancellation/capture refunds `floor(paid_alloy/2)` to the local inventory and emits any overflow loss; creator overrides remain free and explicitly creator-provenanced.
- Civilizations know static topology but learn dynamic ownership, deposits, facilities, and hulls only through observations. Rival private inventories are never visible to autonomous logic.
- There is no victory threshold or terminal civilization count. Zero, one, or many active civilizations continue stepping; a civilization without colonies becomes Dormant and retains identity, policy, relations, history, and surviving fleets.
- All autonomous accepted and rejected intents record actor, target, action, tier/score, relevant observed tick, and a structured reason. Presentation may quote those receipts but must never infer or fabricate motive.
- Preserve the user's unrelated dirty and untracked work. Stage only the exact files named by each task, and do not run a second native process against an active save.

## Cross-Plan Authority Contract

This plan consumes, without redefining, these authority-plan interfaces:

```rust
pub struct LivingGalaxyAuthorityV2;

impl LivingGalaxyAuthorityV2 {
    pub fn from_genesis(
        catalog: ValidatedLivingCatalogPackV2,
        seed: [u8; 32],
        generator: LivingGenesisGeneratorV2,
        genesis: ValidatedLivingGenesisV2,
    ) -> Result<Self, LivingValidationErrorV2>;

    pub fn published_cursor(&self) -> LivingCommandCursorV2;
    pub fn submit(
        &mut self,
        envelope: LivingCommandEnvelopeV2,
    ) -> Result<LivingAcceptedCommandV2, LivingCommandRejectionV2>;
    pub fn step(&mut self) -> Result<LivingTickReceiptV2, LivingDeterministicFaultV2>;
    pub fn state(&self) -> &LivingGalaxyStateV2;
}

pub fn encode_living_archive_v2(
    history: &LivingHistoryV2,
) -> Result<CanonicalLivingArchiveV2, LivingArchiveErrorV2>;

pub fn decode_living_archive_v2(
    catalog: ValidatedLivingCatalogPackV2,
    bytes: &[u8],
    work_budget: u16,
) -> Result<LivingHistoryV2, LivingArchiveErrorV2>;

pub(crate) struct LivingStepContextV2<'a> {
    pub(crate) catalog: &'a ValidatedLivingCatalogPackV2,
    pub(crate) genesis_seed: [u8; 32],
    pub(crate) branch: LivingBranchIdV2,
    pub(crate) tick: LivingTickV2,
    pub(crate) state: &'a mut LivingGalaxyStateV2,
    pub(crate) events: &'a mut Vec<LivingEventV2>,
}

impl LivingStepContextV2<'_> {
    pub(crate) fn emit(
        &mut self,
        provenance: LivingProvenanceV2,
        kind: LivingEventKindV2,
    ) -> Result<(), LivingDeterministicFaultV2>;
}

pub struct LivingTickReceiptV2 {
    pub tick: LivingTickV2,
    pub applied_revisions: Vec<LivingRevisionIdV2>,
    pub events: Vec<LivingEventV2>,
    pub state_digest: LivingStateDigestV2,
    pub receipt_digest: LivingReceiptDigestV2,
}

pub struct LivingEventV2 {
    pub id: LivingEventIdV2,
    pub ordinal: u16,
    pub provenance: LivingProvenanceV2,
    pub kind: LivingEventKindV2,
}
```

`LivingGalaxyStateV2` keeps these fields in frozen wire order: `tick`, `accepted_sequence`, `branch_sequence`, `systems`, `stars`, `worlds`, `lanes`, `civilizations`, `deposits`, `colonies`, `facilities`, `construction_jobs`, `hull_jobs`, `fleets`, `routes`, `shipments`, `relations`, `agreements`, `wars`, `observations`, `hazards`, `settlement_claims`, `occupations`, and `counters`. Logical maps are sorted `Vec<T>` records, never generic JSON maps. This plan may not add, remove, or reorder a canonical field.

The authority plan also freezes `LivingInventoryV2`, `LivingCivilizationV2`, `LivingCivilizationStatusV2`, `LivingPolicyV2`, `LivingColonyV2`, `LivingHubV2`, `LivingFacilityV2`, `LivingFacilityStatusV2`, `LivingBlockedReasonV2`, `LivingConstructionJobV2`, `LivingHullJobV2`, `LivingFleetV2`, `LivingHullV2`, `LivingFleetLocationV2`, `LivingFleetOrderV2`, `LivingFreightRouteV2`, `LivingRouteKindV2`, `LivingShipmentV2`, `LivingShipmentDispositionV2`, `LivingObservationV2`, `LivingRelationV2`, `LivingRelationReasonsV2`, `LivingAgreementV2`, `LivingAgreementKindV2`, `LivingWarV2`, `LivingSettlementClaimV2`, `LivingOccupationV2`, `LivingHazardV2`, and `LivingCountersV2`. Extend behavior around those exact types; do not create a second civilization model.

The schema-frozen `LivingEventKindV2` contains creator/autonomous intent, construction/hull, recipe/block, repair, observation, agreement/war/truce, fleet, retreat, combat/destruction, occupation/capture, claim, shipment/route, dormancy/reactivation, hazard, refund, and capacity-rejection variants. Add no new discriminant in this plan. Emit through `LivingStepContextV2::emit`; never construct event IDs or ordinals in a subsystem.

Creator integration consumes `LivingCommandV2::CreatorBatch { operations: Vec<LivingCreatorOpV2> }`. Its exact civilization variants are `EstablishColony`, `TransferColony`, `UnownColony`, `PlaceFacility`, `SetFacilityEnabled`, `ScrapFacility`, `CreateConstructionJob`, `CancelConstructionJob`, `ReorderConstructionJob`, `CreateHullJob`, `CancelHullJob`, `CreateCivilization`, `EditCivilization`, `SetCivilizationPolicy`, `SetBaseRelation`, `SetAgreement`, `EndAgreement`, `ForceWar`, `ForcePeace`, `CreateFleet`, `EditFleet`, `SplitFleet`, `MergeFleets`, `RemoveFleet`, `IssueFleetOrder`, `CancelFleetOrder`, `CreateRoute`, `EditRoute`, `SetRouteSuspended`, `RemoveRoute`, and `RemoveWithCascade`. Creation operations carry `batch_local_id: u16`; `RemoveWithCascade` carries every explicit dependent disposition.

This plan produces these exact phase hooks for `living::simulation`:

```rust
pub(crate) fn advance_lifecycles(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2>;

pub(crate) fn resolve_arrivals(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2>;

pub(crate) fn resolve_combat_occupation_and_claims(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2>;

pub(crate) fn complete_jobs_and_run_recipes(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2>;

pub(crate) fn refresh_observations(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2>;

pub(crate) fn update_diplomacy(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2>;

pub(crate) fn generate_and_reserve_intents(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2>;

pub(crate) fn dispatch_orders_and_freight(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2>;
```

## File Map

- `crates/nyon-workshop-core/src/living/civilization.rs`: lifecycle transitions, stable sorted-record helpers, capacity/reservation invariants, and phase 2 coordination.
- `crates/nyon-workshop-core/src/living/economy.rs`: atomic inventories/recipes, hub clocks, construction/hull completion, refunds, and repair.
- `crates/nyon-workshop-core/src/living/fleet.rs`: deterministic pathing, launch energy/return credits, orders, departures, arrivals, splitting/merging constraints, and phase 9 coordination.
- `crates/nyon-workshop-core/src/living/freight.rs`: route permission/reserve checks, storm-adjusted dispatch, shipment waiting/delivery/capture/one-time return, and route suspension.
- `crates/nyon-workshop-core/src/living/intelligence.rs`: visibility-derived observations and stale-information-safe query helpers.
- `crates/nyon-workshop-core/src/living/diplomacy.rs`: directed reasons/decay, agreements, war/truce lifecycle and proposal resolution.
- `crates/nyon-workshop-core/src/living/combat.rs`: automatic noncombatant retreat, simultaneous rounds, occupation/capture, settlement arbitration, and dormancy/reactivation.
- `crates/nyon-workshop-core/src/living/strategy.rs`: 200-tick integer forecasts, economic/fleet candidates, stable central reservation, policy scoring, demand requests, and decision reasons.
- `crates/nyon-workshop-core/tests/common/living.rs`: shared pure fixture builders and event assertions; no privileged production mutation path.
- `crates/nyon-workshop-core/tests/living_*.rs`: one focused integration binary per independently reviewable rules subsystem.
- `crates/nyon-workshop-core/tests/fixtures/living-v2/*.json`: canonical validated manifests and frozen ordered witnesses/digests for First Expansion, Commerce, Frontier Friction, and Interrupted Corridor.
- `crates/nyon-workshop-core/examples/freeze_living_fixtures.rs`: deterministic review tool that derives fixture witness/digest files through the same public authority path and refuses to overwrite without `--write`.

---

### Task 1: Lock the Civilization Phase Boundary and Empty-Galaxy Continuity

**Files:**
- Modify: `crates/nyon-workshop-core/src/living/mod.rs`
- Modify: `crates/nyon-workshop-core/src/living/simulation.rs`
- Create: `crates/nyon-workshop-core/src/living/civilization.rs`
- Create: `crates/nyon-workshop-core/src/living/diplomacy.rs`
- Create: `crates/nyon-workshop-core/tests/common/living.rs`
- Create: `crates/nyon-workshop-core/tests/living_civilization.rs`

**Interfaces:**
- Consumes: authority-owned `LivingGalaxyStateV2`, `LivingStepContextV2`, `LivingDeterministicFaultV2`, sorted canonical records, and schema-frozen event kinds.
- Produces: `advance_lifecycles(&mut LivingStepContextV2<'_>)`, stable lookup/capacity helpers used by later phase modules, and public-test fixture construction through `LivingGalaxyAuthorityV2::from_genesis` only.

- [ ] **Step 1: Write the failing empty/one/many civilization tests**

```rust
mod common;

use nyon_workshop_core::{
    living::{LivingCivilizationStatusV2, LivingTickV2},
};

#[test]
fn empty_galaxy_advances_without_a_terminal_state() {
    let mut authority = common::living::empty_authority(0x454d_5054_5900_0001);
    let receipt = authority.step().unwrap();
    assert_eq!(receipt.tick, LivingTickV2(1));
    assert!(receipt.events.is_empty());
    assert!(common::living::has_no_terminal_outcome(authority.state()));
}

#[test]
fn active_and_dormant_status_never_ends_the_sandbox() {
    for active_count in [0_usize, 1, 16] {
        let mut authority = common::living::status_fixture(active_count, 16 - active_count);
        authority.step().unwrap();
        assert!(common::living::has_no_terminal_outcome(authority.state()));
        assert_eq!(
            authority
                .state()
                .civilizations
                .iter()
                .filter(|civilization| civilization.status == LivingCivilizationStatusV2::Active)
                .count(),
            active_count,
        );
    }
}
```

- [ ] **Step 2: Run the focused test and verify the missing phase module**

Run: `cargo test -p nyon-workshop-core --test living_civilization`

Expected: FAIL to compile because `living::civilization` and `tests/common/living.rs` do not exist.

- [ ] **Step 3: Add sorted-record and checked-capacity helpers**

```rust
pub(crate) fn sorted_index_by_id<T>(
    records: &[T],
    id: LivingEntityIdV2,
    key: impl Fn(&T) -> LivingEntityIdV2,
) -> Result<usize, LivingDeterministicFaultV2> {
    records
        .binary_search_by_key(&id, key)
        .map_err(|_| LivingDeterministicFaultV2::Invariant)
}

pub(crate) fn ensure_capacity(
    current: usize,
    reserved: usize,
    limit: usize,
) -> Result<(), LivingDeterministicFaultV2> {
    current
        .checked_add(reserved)
        .filter(|total| *total < limit)
        .map(|_| ())
        .ok_or(LivingDeterministicFaultV2::Capacity)
}
```

Use `partition_point`/`binary_search_by_key` for every read or insertion into canonical record vectors. Never sort after emitting events; validation establishes sorted uniqueness before the phase begins.

- [ ] **Step 4: Implement lifecycle boundary evaluation without terminal state**

`advance_lifecycles` expires no diplomacy object itself; it invokes the diplomacy lifecycle helper added in Task 7 and the authority-owned hazard boundary helper in exact phase-2 order. Its immediately testable behavior is that it neither deletes a civilization nor introduces a terminal outcome when the civilization collection is empty or entirely dormant.

```rust
pub(crate) fn advance_lifecycles(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    crate::living::diplomacy::expire_due_records(ctx)?;
    crate::living::simulation::update_hazard_boundaries(ctx)?;
    Ok(())
}
```

Until Task 7 lands in the same implementation sequence, add `expire_due_records` with the correct empty-record behavior and a direct unit test; Task 7 replaces only its non-empty implementation, not this signature.

- [ ] **Step 5: Run the focused and V1 regression tests**

Run: `cargo test -p nyon-workshop-core --test living_civilization && cargo test -p nyon-workshop-core --test simulation`

Expected: PASS; one Living tick commits for zero/one/many active civilizations, and WorkshopV1 simulation fixtures remain unchanged.

- [ ] **Step 6: Commit the phase boundary**

```bash
git add crates/nyon-workshop-core/src/living/mod.rs \
  crates/nyon-workshop-core/src/living/simulation.rs \
  crates/nyon-workshop-core/src/living/civilization.rs \
  crates/nyon-workshop-core/src/living/diplomacy.rs \
  crates/nyon-workshop-core/tests/common/living.rs \
  crates/nyon-workshop-core/tests/living_civilization.rs
git commit -m "feat(living): establish civilization phase boundary"
```

### Task 2: Implement Atomic Recipes, Hub Recovery, and Stored Cadence

**Files:**
- Create: `crates/nyon-workshop-core/src/living/economy.rs`
- Modify: `crates/nyon-workshop-core/src/living/mod.rs`
- Modify: `crates/nyon-workshop-core/src/living/simulation.rs`
- Create: `crates/nyon-workshop-core/tests/living_economy.rs`

**Interfaces:**
- Consumes: `LivingInventoryV2`, `LivingColonyV2`, `LivingHubV2`, `LivingFacilityV2`, `LivingBlockedReasonV2`, catalog recipes, `LivingEventKindV2::{RecipeProduced,RecipeBlocked}`, and phase-5 `LivingStepContextV2`.
- Produces: `complete_jobs_and_run_recipes(&mut LivingStepContextV2<'_>)`, crate-private `try_atomic_recipe`, and exact operation-status reasons for inspection.

- [ ] **Step 1: Write the five failing rule fixtures**

```rust
#[test]
fn due_foundry_is_atomic_and_advances_when_blocked() {
    let mut fixture = common::living::foundry_fixture(4, 3, 0, 20);
    let before = fixture.inventory();
    fixture.step_to(20).unwrap();
    assert_eq!(fixture.inventory(), before);
    assert_eq!(fixture.foundry_next_due(), LivingTickV2(40));
    assert_eq!(
        fixture.foundry_blocked_reasons(),
        &[LivingBlockedReasonV2::MissingEnergy { required: 4, available: 3 }],
    );
    fixture.set_inventory(LivingInventoryV2 { energy: 4, ore: 4, alloy: 0 });
    fixture.step_to(39).unwrap();
    assert_eq!(fixture.inventory().alloy, 0);
    fixture.step_to(40).unwrap();
    assert_eq!(fixture.inventory(), LivingInventoryV2 { energy: 0, ore: 0, alloy: 2 });
}

#[test]
fn output_capacity_blocks_every_input_atomically() {
    let mut fixture = common::living::foundry_fixture(4, 4, 9_999, 20);
    fixture.step_to(20).unwrap();
    assert_eq!(fixture.inventory(), LivingInventoryV2 { energy: 4, ore: 4, alloy: 9_999 });
    assert!(fixture.foundry_blocked_reasons().contains(
        &LivingBlockedReasonV2::OutputFull { resource: LivingResourceV2::Alloy }
    ));
}

#[test]
fn hub_recovery_first_fabricates_alloy_at_boundary_400() {
    let mut fixture = common::living::empty_hub_fixture();
    fixture.step_to(399).unwrap();
    assert_eq!(fixture.inventory().alloy, 0);
    fixture.step_to(400).unwrap();
    assert_eq!(fixture.inventory().alloy, 1);
}
```

Add `building_clock_starts_at_genesis_plus_period` and `extractor_leaves_a_reserve_below_four_unused`. The genesis test asserts first hub-energy/solar/extractor attempts at 10, foundry at 20, and hub-ore/fallback at 100.

- [ ] **Step 2: Run the focused suite and verify missing behavior**

Run: `cargo test -p nyon-workshop-core --test living_economy`

Expected: FAIL because the phase-5 economy hook is absent; after only the hook is wired, the assertions fail because no due clock changes inventory.

- [ ] **Step 3: Implement an atomic checked recipe primitive**

```rust
fn try_atomic_recipe(
    inventory: LivingInventoryV2,
    inputs: LivingInventoryV2,
    outputs: LivingInventoryV2,
) -> Result<Result<LivingInventoryV2, Vec<LivingBlockedReasonV2>>, LivingDeterministicFaultV2> {
    let mut blocked = Vec::new();
    blocked.extend(inventory.missing_inputs(inputs));
    blocked.extend(inventory.full_outputs(outputs, 10_000)?);
    if !blocked.is_empty() {
        return Ok(Err(blocked));
    }
    let next = inventory.checked_sub(inputs)?.checked_add(outputs)?;
    Ok(Ok(next))
}
```

`LivingInventoryV2::checked_sub` and `checked_add` must return `LivingDeterministicFaultV2::Arithmetic`; they never saturate. Apply a recipe result to the candidate only after all linked deposit, input, and output checks succeed.

- [ ] **Step 4: Implement exact hub/facility cadence**

At each due clock, advance `next_due` by exactly one period whether the operation runs or blocks. On shared hub boundary: add 2 energy, then add 1 ore, then try `4 energy + 4 ore -> 1 alloy` only while alloy is below 120. Solar produces 8 energy every 10; extractor consumes 2 energy and exactly 4 deposit units for 4 ore every 10; foundry consumes 4 energy and 4 ore for 2 alloy every 20. A newly completed facility sets its first due boundary to completion plus its period.

```rust
fn advance_due(next_due: &mut LivingTickV2, period: u64) -> Result<(), LivingDeterministicFaultV2> {
    next_due.0 = next_due.0.checked_add(period)
        .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
    Ok(())
}
```

Emit `RecipeProduced` only for a committed recipe and `RecipeBlocked` with the complete sorted reason set for a blocked due attempt.

- [ ] **Step 5: Run focused, pure-core wasm, and format gates**

Run: `cargo test -p nyon-workshop-core --test living_economy && cargo check -p nyon-workshop-core --target wasm32-unknown-unknown && cargo fmt --all --check`

Expected: PASS; all five exact economy fixtures pass and the pure authority compiles for wasm.

- [ ] **Step 6: Commit economy behavior**

```bash
git add crates/nyon-workshop-core/src/living/economy.rs \
  crates/nyon-workshop-core/src/living/mod.rs \
  crates/nyon-workshop-core/src/living/simulation.rs \
  crates/nyon-workshop-core/tests/living_economy.rs
git commit -m "feat(living): add atomic colony economy"
```

### Task 3: Implement Construction, Hull Reservations, Refunds, and Repair

**Files:**
- Modify: `crates/nyon-workshop-core/src/living/economy.rs`
- Create: `crates/nyon-workshop-core/tests/living_construction.rs`

**Interfaces:**
- Consumes: authority command validation for civilization-funded construction/hull intents; `LivingConstructionJobV2`, `LivingHullJobV2`, `LivingFacilityStatusV2`, reservation counters, and schema-frozen completion/refund/repair/capacity events.
- Produces: atomic job acceptance helpers used by strategy, phase-5 job completion before recipes, cancellation/capture refund helpers used by combat, and stable repair scheduling.

- [ ] **Step 1: Write failing construction and reservation tests**

```rust
#[test]
fn solar_completion_and_first_recipe_use_distinct_boundaries() {
    let mut fixture = common::living::solar_job_fixture(LivingTickV2(0));
    fixture.step_to(99).unwrap();
    assert!(!fixture.has_completed_solar());
    fixture.step_to(100).unwrap();
    assert!(fixture.has_completed_solar());
    assert_eq!(fixture.solar_energy_produced(), 0);
    fixture.step_to(109).unwrap();
    assert_eq!(fixture.solar_energy_produced(), 0);
    fixture.step_to(110).unwrap();
    assert_eq!(fixture.solar_energy_produced(), 8);
}

#[test]
fn one_remaining_hull_slot_is_reserved_once_without_double_charge() {
    let mut fixture = common::living::hull_capacity_fixture(2_047, 40);
    assert!(fixture.accept_escort_job().is_ok());
    let after_first = fixture.inventory();
    assert_eq!(after_first.alloy, 20);
    assert_eq!(fixture.accept_escort_job(), Err(LivingCommandRejectionV2::Capacity));
    assert_eq!(fixture.inventory(), after_first);
    fixture.step_to_job_completion().unwrap();
    assert_eq!(fixture.hull_count(), 2_048);
}

#[test]
fn pending_hull_delivery_blocks_depart_split_and_merge_atomically() {
    let mut fixture = common::living::reserved_fleet_fixture();
    let before = fixture.authority_bytes();
    for order in [
        common::living::depart_command(),
        common::living::split_command(),
        common::living::merge_command(),
    ] {
        assert_eq!(fixture.submit(order), Err(LivingCommandRejectionV2::ReservationConflict));
        assert_eq!(fixture.authority_bytes(), before);
    }
}
```

Add `full_building_queue_and_six_reserved_slots_reject_before_payment`, `cancellation_refunds_floor_half_and_records_overflow`, and `repair_clock_attempts_at_damage_plus_ten_and_advances_when_energy_blocked`.

- [ ] **Step 2: Run the suite and confirm the first missing rule**

Run: `cargo test -p nyon-workshop-core --test living_construction`

Expected: FAIL at `solar_completion_and_first_recipe_use_distinct_boundaries`; the facility either never completes or incorrectly produces at 100.

- [ ] **Step 3: Implement validate-reserve-pay acceptance**

```rust
fn reserve_construction(
    state: &mut LivingGalaxyStateV2,
    request: ConstructionRequestV2,
) -> Result<LivingEntityIdV2, LivingCommandRejectionV2> {
    validate_target_slot_and_role(state, &request)?;
    validate_global_capacity_with_reservations(state, &request)?;
    let paid_alloy = request.catalog_cost;
    require_local_alloy(state, request.world, paid_alloy)?;
    let job_id = request.preallocated_id;
    let job = build_construction_job(request, paid_alloy)?;
    deduct_alloy_and_insert_job_atomically(state, job)?;
    Ok(job_id)
}
```

Validation order is target/reference, queue/role, local/global reservation capacity, then affordability; no rejected job changes inventory, jobs, counters, fleet order, fuel credit, or IDs.

- [ ] **Step 4: Implement completion, refund, and repair order**

Complete building jobs, then hull jobs, then run hub/facility recipes, then repair damaged docked owned hulls and batteries in ascending entity-ID order. A destroyed/captured target cancels its job, releases every reservation, returns half paid alloy into the current world inventory up to 10,000, and emits `Refunded` for kept and discarded units.

```rust
pub(crate) fn cancel_paid_job(
    ctx: &mut LivingStepContextV2<'_>,
    world: LivingEntityIdV2,
    paid_alloy: u64,
    provenance: LivingProvenanceV2,
) -> Result<(), LivingDeterministicFaultV2> {
    let refund = paid_alloy / 2;
    let accepted = add_with_cap(&mut colony_mut(ctx.state, world)?.inventory.alloy, refund, 10_000)?;
    ctx.emit(provenance, LivingEventKindV2::Refunded {
        world,
        resource: LivingResourceV2::Alloy,
        accepted,
        discarded: refund.checked_sub(accepted).ok_or(LivingDeterministicFaultV2::Arithmetic)?,
    })
}
```

- [ ] **Step 5: Run the economy and construction suites**

Run: `cargo test -p nyon-workshop-core --test living_economy --test living_construction`

Expected: PASS; seven construction/reservation/repair tests and all economy fixtures pass.

- [ ] **Step 6: Commit construction and repair**

```bash
git add crates/nyon-workshop-core/src/living/economy.rs \
  crates/nyon-workshop-core/tests/living_construction.rs
git commit -m "feat(living): enforce construction reservations and repair"
```

### Task 4: Implement Deterministic Fleet Routing, Fuel, Orders, and Arrivals

**Files:**
- Create: `crates/nyon-workshop-core/src/living/fleet.rs`
- Modify: `crates/nyon-workshop-core/src/living/mod.rs`
- Modify: `crates/nyon-workshop-core/src/living/simulation.rs`
- Create: `crates/nyon-workshop-core/tests/living_fleet.rs`

**Interfaces:**
- Consumes: `LivingFleetV2`, `LivingHullV2`, `LivingFleetLocationV2`, `LivingFleetOrderV2`, sorted systems/worlds/lanes, hull-delivery reservations, catalog hull definitions, and autonomous ID allocation owned by authority.
- Produces: shortest deterministic path/travel helpers, phase-3 fleet-arrival processing, `dispatch_orders_and_freight` phase-9 coordinator, validated split/merge/order primitives, and return-credit-safe launches.

- [ ] **Step 1: Write failing fuel, travel, tie, and no-same-boundary tests**

```rust
#[test]
fn distance_101_takes_eleven_ticks() {
    assert_eq!(living::fleet::direct_leg_ticks(101).unwrap(), 11);
    let mut fixture = common::living::travelling_fleet_fixture(101, LivingTickV2(50));
    fixture.step_to(60).unwrap();
    assert!(fixture.fleet_is_travelling());
    fixture.step_to(61).unwrap();
    assert!(fixture.fleet_is_docked_at_destination());
}

#[test]
fn fresh_two_hull_launch_costs_eight_and_seven_rejects_atomically() {
    let mut fixture = common::living::two_hull_launch_fixture(7, [false, false]);
    let before = fixture.authority_bytes();
    assert_eq!(fixture.launch(), Err(LivingCommandRejectionV2::InsufficientResource));
    assert_eq!(fixture.authority_bytes(), before);

    fixture.set_energy(8);
    fixture.launch().unwrap();
    assert_eq!(fixture.energy(), 0);
    assert_eq!(fixture.return_credits(), [true, true]);
}

#[test]
fn existing_return_credits_reduce_two_hull_launch_to_four() {
    let mut fixture = common::living::two_hull_launch_fixture(4, [true, true]);
    fixture.launch().unwrap();
    assert_eq!(fixture.energy(), 0);
    assert_eq!(fixture.return_credits(), [true, true]);
}
```

Add `equal_cost_paths_choose_lexicographically_smallest_lane_id_sequence`, `hull_completion_cannot_depart_until_the_following_boundary`, `waypoint_revalidates_removed_remaining_route`, and `fleet_capacity_blocks_split_but_not_existing_fleet_move`.

- [ ] **Step 2: Run the fleet test and verify failure**

Run: `cargo test -p nyon-workshop-core --test living_fleet`

Expected: FAIL to compile because `living::fleet::direct_leg_ticks` and fleet phase hooks do not exist.

- [ ] **Step 3: Implement exact integer travel and stable pathing**

```rust
pub(crate) fn direct_leg_ticks(distance_units: u64) -> Result<u64, LivingDeterministicFaultV2> {
    Ok((distance_units
        .checked_add(9)
        .ok_or(LivingDeterministicFaultV2::Arithmetic)? / 10)
        .max(10))
}
```

Same-system world transfer is exactly 10 ticks. Dijkstra state compares `(total_ticks, lane_id_path, system_id)` and visits sorted adjacency; equal total duration selects the lexicographically smallest complete lane-ID path. Endpoint world travel adds no hidden duration.

- [ ] **Step 4: Implement launch payment, credits, and immutable legs**

For each hull, charge 2 energy for the outbound itinerary plus 2 if it lacks a return credit. A normal outward launch preserves an existing return credit or creates the paid credit. A return departure consumes exactly one credit per hull and charges no energy. A departed leg retains endpoints and duration; at each waypoint, validate the remaining graph and hold with No route if disconnected.

Split/merge only stationary co-located same-owner hulls, preserve each hull ID/HP/credit, and reject during combat or reservation. Newly accepted orders carry `not_before_tick = current + 1`; dispatch uses only orders whose boundary is due.

- [ ] **Step 5: Run focused and insertion-order tests**

Run: `cargo test -p nyon-workshop-core --test living_fleet && cargo test -p nyon-workshop-core --test living_authority insertion_order`

Expected: PASS; all seven fleet tests pass, including identical paths and post-arrival state under reversed input collection insertion.

- [ ] **Step 6: Commit fleet movement**

```bash
git add crates/nyon-workshop-core/src/living/fleet.rs \
  crates/nyon-workshop-core/src/living/mod.rs \
  crates/nyon-workshop-core/src/living/simulation.rs \
  crates/nyon-workshop-core/tests/living_fleet.rs
git commit -m "feat(living): add deterministic fleet travel"
```

### Task 5: Implement Reserve-Preserving Freight and In-Flight Disposition

**Files:**
- Create: `crates/nyon-workshop-core/src/living/freight.rs`
- Modify: `crates/nyon-workshop-core/src/living/mod.rs`
- Modify: `crates/nyon-workshop-core/src/living/fleet.rs`
- Modify: `crates/nyon-workshop-core/src/living/simulation.rs`
- Create: `crates/nyon-workshop-core/tests/living_freight.rs`

**Interfaces:**
- Consumes: `LivingFreightRouteV2`, `LivingRouteKindV2`, `LivingShipmentV2`, `LivingShipmentDispositionV2`, ownership, agreements/wars, hazards, phase-3/phase-9 hooks, and authority autonomous entity IDs.
- Produces: `resolve_arrivals(&mut LivingStepContextV2<'_>)`, `dispatch_due_routes`, route-suspension handling, and exactly-once delivery/capture/return/wait semantics.

- [ ] **Step 1: Write the seven failing freight disposition fixtures**

```rust
#[test]
fn route_keeps_twenty_at_source_and_storm_halves_batch_with_floor() {
    let mut fixture = common::living::freight_fixture(27, 10, 20);
    fixture.add_ion_storm(LivingTickV2(10), LivingTickV2(20));
    fixture.step_to(10).unwrap();
    assert_eq!(fixture.source_units(), 22);
    assert_eq!(fixture.last_dispatched_units(), 5);
    fixture.step_to(20).unwrap();
    assert_eq!(fixture.last_dispatched_units(), 2);
}

#[test]
fn waiting_cargo_rechecks_capture_before_capacity() {
    let mut fixture = common::living::waiting_shipment_fixture(50, 10_000);
    fixture.step_to(50).unwrap();
    assert!(fixture.shipment_is_waiting());
    fixture.capture_destination_at(51);
    fixture.free_destination_space_at(52, 10);
    let receipt = fixture.step_to(52).unwrap();
    assert_eq!(common::living::count_events(&receipt, "shipment_captured"), 1);
    assert_eq!(fixture.trade_credit(), 0);
}

#[test]
fn war_turns_waiting_delivery_into_one_return_even_when_destination_is_full() {
    let mut fixture = common::living::waiting_shipment_fixture(50, 10_000);
    fixture.declare_war_at(51);
    fixture.step_to(51).unwrap();
    assert!(fixture.shipment_is_returning());
    fixture.step_to_return().unwrap();
    assert!(!fixture.has_shipment());
    assert_eq!(fixture.return_event_count(), 1);
}
```

Add `captured_destination_delivers_once_without_duplication`, `captured_source_suspends_route_before_dispatch`, `full_receiver_waits_then_delivers_atomically`, and `treaty_expiry_does_not_recall_already_sent_cargo`.

- [ ] **Step 2: Run the focused suite and capture the first failure**

Run: `cargo test -p nyon-workshop-core --test living_freight`

Expected: FAIL because route dispatch does not enforce reserve 20, shipment disposition is absent, or the phase-3 arrival handler is missing.

- [ ] **Step 3: Implement route eligibility and dispatch**

Validate same-system or direct-lane endpoints, batch `1..=10`, cadence 10, source reserve at least 20, immutable participant owners, current source ownership, and current bilateral permission. Internal routes require same owner, Trade requires a current Trade agreement, and Aid must be explicitly one-way from the source owner without taking foreign stock.

```rust
let above_reserve = source_units.saturating_sub(route.source_reserve.max(20));
let hazard_batch = strongest_ion_storm_ratio(ctx, route)?
    .map_or(route.batch_units, |(n, d)| route.batch_units.checked_mul(n).ok_or(
        LivingDeterministicFaultV2::Arithmetic,
    ).map(|units| units / d))?;
let units = route.batch_units.min(above_reserve).min(hazard_batch);
if units == 0 { return Ok(()); }
```

At shipment capacity, reject dispatch before deduction. Phase-3 removals free capacity before phase-9 dispatch, so exactly one route may use a slot freed at the same boundary.

- [ ] **Step 4: Implement disposition-before-capacity arrival logic**

At arrival and every waiting retry, decide in order: changed-owner capture, unchanged friendly delivery, unchanged now-at-war return, or returning unload at original source. A required return starts immediately even if destination storage is full. Only after choosing the recipient check storage. Waiting shipments stay allocated and retry every tick. Returning shipments never bounce again. Captured/discarded cargo earns no trade/aid credit.

Suspend a route when either endpoint owner differs from the recorded participants. Only a newly validated adjustment by the current source owner reactivates it.

- [ ] **Step 5: Run freight, economy, and shipment-capacity tests**

Run: `cargo test -p nyon-workshop-core --test living_freight --test living_economy`

Expected: PASS; the seven freight fixtures pass and a phase-3 removal permits only one phase-9 replacement at the 4,096-shipment cap.

- [ ] **Step 6: Commit freight behavior**

```bash
git add crates/nyon-workshop-core/src/living/freight.rs \
  crates/nyon-workshop-core/src/living/fleet.rs \
  crates/nyon-workshop-core/src/living/mod.rs \
  crates/nyon-workshop-core/src/living/simulation.rs \
  crates/nyon-workshop-core/tests/living_freight.rs
git commit -m "feat(living): implement authoritative freight"
```

### Task 6: Implement Limited Intelligence and Settlement Arbitration

**Files:**
- Create: `crates/nyon-workshop-core/src/living/intelligence.rs`
- Create: `crates/nyon-workshop-core/src/living/combat.rs`
- Modify: `crates/nyon-workshop-core/src/living/mod.rs`
- Modify: `crates/nyon-workshop-core/src/living/simulation.rs`
- Create: `crates/nyon-workshop-core/tests/living_intelligence.rs`
- Create: `crates/nyon-workshop-core/tests/living_settlement.rs`

**Interfaces:**
- Consumes: `LivingObservationV2`, authoritative static topology, dynamic ownership/deposits/facilities/hulls, `LivingSettlementClaimV2`, arks and fleet arrivals, `LivingIdentityV2` claim ranking, and observation/claim receipt variants.
- Produces: `refresh_observations(&mut LivingStepContextV2<'_>)`, observation query functions used by strategy/diplomacy, and the settlement portion of `resolve_combat_occupation_and_claims`.

- [ ] **Step 1: Write failing visibility and hidden-information tests**

```rust
#[test]
fn colony_and_scout_refresh_only_their_allowed_visibility() {
    let mut fixture = common::living::three_system_observation_fixture();
    fixture.step_once().unwrap();
    assert_eq!(fixture.observed_worlds_for_actor(), fixture.home_system_worlds());

    fixture.place_scout_at_remote_world();
    fixture.step_once().unwrap();
    assert!(fixture.observed_worlds_for_actor().contains(&fixture.remote_world()));
    assert!(!fixture.observed_worlds_for_actor().contains(&fixture.hidden_world()));
}

#[test]
fn unseen_rival_change_does_not_change_autonomous_choice() {
    let base = common::living::hidden_rival_fixture();
    let mut hidden_change = base.clone();
    hidden_change.creator_add_hidden_battery();
    let left = base.next_strategy_receipt().unwrap();
    let right = hidden_change.next_strategy_receipt().unwrap();
    assert_eq!(common::living::actor_decision(&left), common::living::actor_decision(&right));
}
```

Add `old_observation_retains_original_tick_after_visibility_ends`, `rival_stockpile_is_absent_from_observation`, and `fresh_hostile_target_is_eligible_through_age_100_but_not_101`.

- [ ] **Step 2: Write failing claim-window tests**

```rust
#[test]
fn claim_window_is_half_open_and_arbitrates_at_close() {
    let mut fixture = common::living::claim_fixture(LivingTickV2(100));
    fixture.arrive_ark_at(LivingTickV2(100), fixture.civilization_a());
    fixture.arrive_ark_at(LivingTickV2(149), fixture.civilization_b());
    fixture.arrive_ark_at(LivingTickV2(150), fixture.civilization_c());
    let receipt = fixture.step_to(150).unwrap();
    assert_eq!(fixture.closed_claimants(), [fixture.civilization_a(), fixture.civilization_b()]);
    assert!(common::living::has_event(&receipt, "claim_resolved"));
    assert!(fixture.ark_survives(fixture.civilization_c()));
}

#[test]
fn claim_rank_is_independent_of_collection_insertion_order() {
    let forward = common::living::two_ark_claim_fixture(false).resolve().unwrap();
    let reverse = common::living::two_ark_claim_fixture(true).resolve().unwrap();
    assert_eq!(forward.owner(), reverse.owner());
    assert_eq!(forward.state_digest(), reverse.state_digest());
    assert_eq!(forward.losing_ark_count(), 1);
}
```

Add `hostile_armed_presence_clears_claim_window`, `creator_ownership_invalidates_claim_before_arbitration`, and `winning_ark_is_consumed_once_for_20_20_0_colony`.

- [ ] **Step 3: Run both focused suites and verify failures**

Run: `cargo test -p nyon-workshop-core --test living_intelligence --test living_settlement`

Expected: FAIL because phase 6 does not refresh observations and phase 4 does not open or resolve claim windows.

- [ ] **Step 4: Implement exact observation refresh**

Build one visibility set per civilization in stable actor order. A colony or scout reveals every world in its current system; another fleet reveals only its current world. Replace an observation only for currently visible worlds, retaining prior records and their ticks elsewhere. Copy owner, remaining deposits, facility identities/states, and stationed hull identities/HP; never copy rival inventory.

```rust
pub(crate) fn observation_is_fresh(
    observation: &LivingObservationV2,
    now: LivingTickV2,
    maximum_age: u64,
) -> Result<bool, LivingDeterministicFaultV2> {
    Ok(now.0.checked_sub(observation.observed_tick.0)
        .ok_or(LivingDeterministicFaultV2::Invariant)? <= maximum_age)
}
```

Emit `ObservationRefreshed` only for a newly observed or changed visible record; unchanged visibility still updates the canonical observed tick as required by the current snapshot.

- [ ] **Step 5: Implement claim windows and deterministic rank**

First eligible arrival at `A` opens `[A,A+50)` and stores close `A+50`. Arrivals through close-minus-one join; close-boundary arrivals do not. At close, keep one claimant per civilization that is still present, peaceful, observed, unowned, and ark-capable. Choose the lexicographically smallest full claim rank, with civilization ID as collision tie-breaker. Consume one winning ark, create one colony/hub with inventory `{energy:20, ore:20, alloy:0}`, leave losing arks intact, and emit `ClaimResolved` once.

If armed hostility appears or a creator establishes ownership, clear the claim with `ClaimResolved` carrying the frozen non-success reason. Capacity or arithmetic faults leave the entire boundary unchanged.

- [ ] **Step 6: Run focused and phase-order suites**

Run: `cargo test -p nyon-workshop-core --test living_intelligence --test living_settlement`

Expected: PASS; all ten tests pass, and arrivals/claims precede economy while observation refresh follows both.

- [ ] **Step 7: Commit intelligence and settlement**

```bash
git add crates/nyon-workshop-core/src/living/intelligence.rs \
  crates/nyon-workshop-core/src/living/combat.rs \
  crates/nyon-workshop-core/src/living/mod.rs \
  crates/nyon-workshop-core/src/living/simulation.rs \
  crates/nyon-workshop-core/tests/living_intelligence.rs \
  crates/nyon-workshop-core/tests/living_settlement.rs
git commit -m "feat(living): add intelligence and settlement"
```

### Task 7: Implement Directed Relations, Agreements, War, and Truce Lifecycles

**Files:**
- Create: `crates/nyon-workshop-core/src/living/diplomacy.rs`
- Modify: `crates/nyon-workshop-core/src/living/civilization.rs`
- Modify: `crates/nyon-workshop-core/src/living/mod.rs`
- Modify: `crates/nyon-workshop-core/src/living/simulation.rs`
- Create: `crates/nyon-workshop-core/tests/living_diplomacy.rs`

**Interfaces:**
- Consumes: `LivingRelationV2`, `LivingRelationReasonsV2`, partial 100-unit delivery counters, `LivingAgreementV2`, `LivingAgreementKindV2`, `LivingWarV2`, current observations/fleets/colonies, and agreement/war/truce event variants.
- Produces: phase-2 `expire_due_records`, phase-7 `update_diplomacy`, bilateral permission queries used by freight, and hostile-state queries used by combat/fleets.

- [ ] **Step 1: Write failing directed-reason and decay tests**

```rust
#[test]
fn relationship_reasons_are_directed_bounded_and_decay_separately() {
    let mut fixture = common::living::relation_fixture(-5, 10);
    fixture.deliver_aid(200);       // recipient toward sender: +20 cap
    fixture.deliver_trade(2_500);   // receiver toward sender: +20 cap
    fixture.declare_war();          // victim toward declarer: -30 floor
    assert_eq!(fixture.recipient_reasons().aid, 20);
    assert_eq!(fixture.recipient_reasons().trade, 20);
    assert_eq!(fixture.victim_reasons().war_declared, -30);
    assert_eq!(fixture.declarer_reasons().war_declared, 0);
    fixture.step_boundaries(600).unwrap();
    assert_eq!(fixture.recipient_reasons().aid, 19);
    assert_eq!(fixture.base_dispositions(), (-5, 10));
}
```

Add `adjacent_settlement_floors_at_minus_20`, `captures_floor_at_minus_60`, and `lost_or_captured_freight_adds_no_delivery_credit`.

- [ ] **Step 2: Write failing treaty and war-boundary tests**

```rust
#[test]
fn end_exclusive_agreement_prevents_war_only_through_1199() {
    let mut fixture = common::living::war_eligible_fixture();
    fixture.add_nonaggression(LivingTickV2(0), LivingTickV2(1_200));
    fixture.step_to(1_199).unwrap();
    assert!(!fixture.at_war());
    let receipt = fixture.step_to(1_200).unwrap();
    assert!(fixture.at_war());
    assert!(common::living::event_before(&receipt, "agreement_expired", "war_declared"));
}

#[test]
fn war_expires_to_exact_six_hundred_tick_truce() {
    let mut fixture = common::living::active_war_fixture(100, 1_300);
    fixture.step_to(1_300).unwrap();
    assert!(!fixture.at_war());
    assert_eq!(fixture.truce_interval(), (1_300, 1_900));
    fixture.step_to(1_899).unwrap();
    assert!(fixture.in_truce());
    fixture.step_to(1_900).unwrap();
    assert!(!fixture.in_truce());
}
```

Add `trade_requires_both_relations_at_least_zero`, `nonaggression_requires_both_at_least_twenty`, `one_war_proposal_uses_lowest_relation_then_world_then_opponent`, `war_requires_fresh_age_100_observation_and_three_halves_strength`, and `duplicate_bilateral_proposals_create_one_record`.

- [ ] **Step 3: Run the focused suite and verify the boundary failure**

Run: `cargo test -p nyon-workshop-core --test living_diplomacy`

Expected: FAIL because no agreement expiry/renewal or war proposal phase exists.

- [ ] **Step 4: Implement exact reasons and permission queries**

Clamp base-plus-reasons to `[-100,100]`. Aid adds +10 per completed 100-unit tranche to +20; trade adds +1 per completed 100-unit tranche to +20; adjacent rival settlement subtracts 10 to -20; war declared against actor subtracts 30 to -30; each captured colony subtracts 30 to -60. Preserve partial delivery counters. Every 600 ticks move each event counter one point toward zero; never decay base disposition.

```rust
pub(crate) fn total_relation(
    relation: &LivingRelationV2,
) -> Result<i16, LivingDeterministicFaultV2> {
    let total = i32::from(relation.base_disposition)
        .checked_add(relation.reasons.checked_sum_i32()?)
        .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
    i16::try_from(total.clamp(-100, 100))
        .map_err(|_| LivingDeterministicFaultV2::Arithmetic)
}

pub(crate) fn may_trade(state: &LivingGalaxyStateV2, a: LivingEntityIdV2, b: LivingEntityIdV2) -> bool {
    bilateral_agreement(state, a, b, LivingAgreementKindV2::Trade).is_some()
        && !is_at_war(state, a, b)
}
```

Every reason is independently range-validated canonical state, and the intermediate still uses checked arithmetic so invalid in-memory state faults atomically.

- [ ] **Step 5: Implement lifecycle and shared-snapshot proposals**

At phase 2, expire agreements/truces/wars at their end-exclusive boundary. A war expiry inserts exactly one 600-tick truce and resets occupation. At phase 7 on multiples of 100, apply delivered counters and scheduled decay, build one immutable post-update snapshot, resolve mutually eligible Trade/Nonaggression proposals, then at most one war proposal per actor. Deduplicate bilateral proposals by ordered actor pair. A war lasts exactly 1,200 ticks.

War eligibility requires relation at most -20, no agreement/truce/war, adjacent rival colony, target observation age at most 100, at least one assembled attacking escort, and `2 * attack_strength >= 3 * observed_defense_strength`, using 10 per escort and 20 per operational battery.

- [ ] **Step 6: Run diplomacy, freight, and phase-order suites**

Run: `cargo test -p nyon-workshop-core --test living_diplomacy --test living_freight`

Expected: PASS; all eleven diplomacy tests pass and freight sees permissions established/expired at the same exact boundary.

- [ ] **Step 7: Commit diplomacy**

```bash
git add crates/nyon-workshop-core/src/living/diplomacy.rs \
  crates/nyon-workshop-core/src/living/civilization.rs \
  crates/nyon-workshop-core/src/living/mod.rs \
  crates/nyon-workshop-core/src/living/simulation.rs \
  crates/nyon-workshop-core/tests/living_diplomacy.rs
git commit -m "feat(living): add bounded diplomacy and war"
```

### Task 8: Implement Simultaneous Combat, Retreat, Occupation, Capture, and Dormancy

**Files:**
- Modify: `crates/nyon-workshop-core/src/living/combat.rs`
- Modify: `crates/nyon-workshop-core/src/living/economy.rs`
- Modify: `crates/nyon-workshop-core/src/living/fleet.rs`
- Modify: `crates/nyon-workshop-core/src/living/freight.rs`
- Create: `crates/nyon-workshop-core/tests/living_combat.rs`
- Create: `crates/nyon-workshop-core/tests/living_occupation.rs`

**Interfaces:**
- Consumes: war/truce queries, fleets/hulls/batteries, return credits/pathing, `LivingOccupationV2`, colonies/facilities/jobs/routes, and schema-frozen retreat/combat/occupation/capture/dormancy events.
- Produces: the complete `resolve_combat_occupation_and_claims` phase-4 hook, atomic capture/cancellation handling, and Active/Dormant lifecycle transitions.

- [ ] **Step 1: Write failing simultaneous-combat and retreat tests**

```rust
#[test]
fn equal_escorts_destroy_each_other_on_boundary_fifty() {
    let mut fixture = common::living::two_escort_battle_fixture();
    for boundary in [10_u64, 20, 30, 40] {
        fixture.step_to(boundary).unwrap();
        assert_eq!(fixture.hull_hp(), [10 - (boundary / 10) * 2; 2]);
    }
    let receipt = fixture.step_to(50).unwrap();
    assert_eq!(fixture.hull_count(), 0);
    assert_eq!(common::living::count_events(&receipt, "asset_destroyed"), 2);
}

#[test]
fn unescorted_noncombatant_retreats_before_target_snapshot() {
    let mut fixture = common::living::retreat_fixture(true);
    let receipt = fixture.step_to_next_combat().unwrap();
    assert!(fixture.noncombatant_is_travelling_home());
    assert_eq!(common::living::count_events(&receipt, "combat_damage"), 0);
}

#[test]
fn blocked_retreat_spends_no_credit_and_remains_targetable() {
    let mut fixture = common::living::retreat_fixture(false);
    let receipt = fixture.step_to_next_combat().unwrap();
    assert!(common::living::has_event(&receipt, "retreat_blocked"));
    assert!(!fixture.return_credit_was_spent());
    assert!(common::living::has_event(&receipt, "combat_damage"));
}
```

Add `retreat_tie_chooses_lowest_world_then_lexicographic_lane_path` and `only_declared_enemies_exchange_damage`.

- [ ] **Step 2: Write failing occupation/capture/dormancy tests**

```rust
#[test]
fn one_hundred_consecutive_eligible_boundaries_capture_colony() {
    let mut fixture = common::living::occupation_fixture();
    fixture.step_to_occupation_progress(99).unwrap();
    assert_eq!(fixture.owner(), fixture.defender());
    fixture.step_once().unwrap();
    assert_eq!(fixture.owner(), fixture.attacker());
    assert!(fixture.hub_persisted());
    assert!(fixture.inventory_and_surviving_facilities_preserved());
}

#[test]
fn third_claimant_at_ninety_nine_resets_occupation() {
    let mut fixture = common::living::occupation_fixture();
    fixture.step_to_occupation_progress(98).unwrap();
    fixture.arrive_third_claimant();
    fixture.step_once().unwrap();
    assert_eq!(fixture.occupation_progress(), 0);
    assert_eq!(fixture.owner(), fixture.defender());
}

#[test]
fn final_colony_loss_preserves_dormant_actor_and_running_sandbox() {
    let mut fixture = common::living::last_colony_fixture();
    fixture.capture_colony().unwrap();
    assert_eq!(fixture.old_owner_status(), LivingCivilizationStatusV2::Dormant);
    assert!(fixture.old_owner_identity_policy_relations_and_fleets_persist());
    assert!(fixture.step_once().is_ok());
}
```

Add `treaty_or_truce_resets_occupation`, `capture_cancels_jobs_with_half_refund_and_suspends_routes`, `multiple_foreign_claimants_are_contested`, and `credited_dormant_ark_can_settle_known_neutral_and_reactivate`.

- [ ] **Step 3: Run both suites and verify the first mismatch**

Run: `cargo test -p nyon-workshop-core --test living_combat --test living_occupation`

Expected: FAIL because damage is not accumulated from a pre-round snapshot and occupation/dormancy are not advanced.

- [ ] **Step 4: Implement pre-round retreat and simultaneous damage**

On global multiples of 10, first evaluate unescorted scout/ark fleets at hostile worlds. A valid retreat chooses minimum travel ticks, lowest owned world ID, then lexicographic lane path and consumes existing credits; it departs immediately. Next snapshot all remaining attackers/targets. Each armed asset targets lowest-ID hostile escort, then battery, then noncombatant. Sum checked damage by target, apply all totals, then remove zero-HP assets without retargeting overkill.

```rust
let snapshot = CombatSnapshotV2::from_state(ctx.state, ctx.tick)?;
let damage = collect_simultaneous_damage(&snapshot)?;
for (target, amount) in damage {
    apply_damage(ctx.state, target, amount)?;
}
remove_destroyed_and_emit(ctx)?;
```

- [ ] **Step 5: Implement occupation and atomic capture**

A unique foreign escort claimant with no defending escort/battery increments once per completed eligible boundary. Any rival claimant, hostile third-party fleet, agreement/truce, or restored defense resets to zero. Transfer at exactly 100 consecutive increments.

Capture preserves hub, inventory, deposits, and surviving facilities; changes ownership once; cancels building/hull jobs with half refund into the captured inventory; releases reservations; suspends ownership-bound routes; resets claims/occupation; adds the directed capture reason; and reevaluates both civilizations' status. Emit the ordered individual events before `ColonyCaptured`, then `CivilizationDormant` or `CivilizationReactivated` if status changed.

- [ ] **Step 6: Run focused, economy, freight, and diplomacy regressions**

Run: `cargo test -p nyon-workshop-core --test living_combat --test living_occupation --test living_construction --test living_freight --test living_diplomacy`

Expected: PASS; twelve combat/occupation tests and all affected subsystem tests pass.

- [ ] **Step 7: Commit conflict and lifecycle behavior**

```bash
git add crates/nyon-workshop-core/src/living/combat.rs \
  crates/nyon-workshop-core/src/living/economy.rs \
  crates/nyon-workshop-core/src/living/fleet.rs \
  crates/nyon-workshop-core/src/living/freight.rs \
  crates/nyon-workshop-core/tests/living_combat.rs \
  crates/nyon-workshop-core/tests/living_occupation.rs
git commit -m "feat(living): add combat occupation and dormancy"
```

### Task 9: Implement the 200-Tick Economic Forecast and Stable Intent Reservation

**Files:**
- Create: `crates/nyon-workshop-core/src/living/strategy.rs`
- Modify: `crates/nyon-workshop-core/src/living/mod.rs`
- Modify: `crates/nyon-workshop-core/src/living/simulation.rs`
- Create: `crates/nyon-workshop-core/tests/living_strategy_economy.rs`

**Interfaces:**
- Consumes: one immutable post-observation strategy snapshot, exact due clocks/routes/repairs/paid jobs, own inventories, contracted freight, catalog definitions, policies, and Task 2/3 construction/route acceptance primitives.
- Produces: `generate_and_reserve_intents(&mut LivingStepContextV2<'_>)`, a pure 200-boundary integer forecast, scored economic candidates, and centrally validated/reserved accepted or rejected intents.

- [ ] **Step 1: Write failing forecast/emergency tests**

```rust
#[test]
fn forecast_replays_paid_jobs_without_charging_them_twice() {
    let fixture = common::living::paid_foundry_forecast_fixture();
    let forecast = fixture.forecast_200().unwrap();
    assert_eq!(forecast.alloy_spent_at_acceptance(), 40);
    assert_eq!(forecast.alloy_spent_during_projection(), 0);
    assert_eq!(forecast.first_foundry_output_tick(), fixture.completion_tick() + 20);
}

#[test]
fn actionable_emergency_precedes_scored_growth_but_impossible_one_does_not() {
    let actionable = common::living::economic_choice_fixture(true, true).choose().unwrap();
    assert_eq!(actionable.kind(), EconomicIntentKindV2::RestorePower);

    let impossible = common::living::economic_choice_fixture(true, false).choose().unwrap();
    assert_eq!(impossible.kind(), EconomicIntentKindV2::BuildScout);
}

#[test]
fn solar_requires_projected_use_and_foundry_requires_projected_inputs() {
    assert!(common::living::idle_full_storage_fixture().economic_candidates().unwrap().is_empty());
    assert!(!common::living::supplied_demand_fixture()
        .economic_candidates().unwrap().is_empty());
}
```

Add `defense_target_counts_reserved_escorts`, `missing_hull_shipyard_yields_shipyard_candidate`, `full_slots_use_explicit_safe_scrap_then_build`, and `policy_bonus_never_makes_illegal_candidate_legal`.

- [ ] **Step 2: Write failing scoring and stable-conflict tests**

```rust
#[test]
fn economic_ties_use_score_target_then_action_ordinal() {
    let choices = common::living::tied_economic_candidates();
    assert_eq!(choices.selected_target(), choices.lowest_target_id());
    assert_eq!(choices.selected_kind(), choices.lowest_kind_ordinal_at_target());
}

#[test]
fn shared_resource_conflict_uses_actor_id_not_iteration_order() {
    let forward = common::living::shared_resource_strategy_fixture(false).step().unwrap();
    let reverse = common::living::shared_resource_strategy_fixture(true).step().unwrap();
    assert_eq!(forward.state_digest, reverse.state_digest);
    assert_eq!(common::living::accepted_actor(&forward), common::living::lowest_actor_id(&forward));
}
```

- [ ] **Step 3: Run the strategy suite and verify missing forecast**

Run: `cargo test -p nyon-workshop-core --test living_strategy_economy`

Expected: FAIL because the private forecast and phase-8 candidate/reservation logic do not exist.

- [ ] **Step 4: Implement the exact forecast**

Clone only the projected integer inventory/clocks/reservations needed for 200 boundaries. At each projected boundary use the real phase order for committed arrivals, construction completion, hub/solar/extractor/foundry, repairs, existing route dispatch, and already-paid queues. Do not include speculative trade, unknown rival inventory, unaccepted job cost, or a queued facility before its actual completion plus first-recipe period.

An emergency is a projected deficit against known scheduled demand, ordered power, inaccessible ore, missing alloy, and considered only if a legal affordable remedy exists.

- [ ] **Step 5: Implement candidate scores and central reservation**

Use base scores exactly: supply 70; extractor 50; foundry 50; ark 50; escort 50; solar 40; scout 40; missing shipyard 55; threatened battery 55. Add +20 only for the matching Expansionist, Industrialist, Guardian, or Trader policy. Sort by emergency tier, descending score, target ID, fixed action-kind ordinal. Neutral adds zero.

Generate candidates for every active actor from the same immutable snapshot. In actor-ID order, revalidate each winner against a private shared reservation ledger and candidate state. Accept at most one economic intent per actor; a conflict rejection spends nothing and emits `AutonomousIntentRejected` with the structured conflict reason.

- [ ] **Step 6: Run focused and economy/construction suites**

Run: `cargo test -p nyon-workshop-core --test living_strategy_economy --test living_economy --test living_construction`

Expected: PASS; all nine strategy-economy tests and all resource/job regression tests pass.

- [ ] **Step 7: Commit economic strategy**

```bash
git add crates/nyon-workshop-core/src/living/strategy.rs \
  crates/nyon-workshop-core/src/living/mod.rs \
  crates/nyon-workshop-core/src/living/simulation.rs \
  crates/nyon-workshop-core/tests/living_strategy_economy.rs
git commit -m "feat(living): add explainable economic strategy"
```

### Task 10: Implement Fleet Strategy, Demand Exchange, and Explainable Decisions

**Files:**
- Modify: `crates/nyon-workshop-core/src/living/strategy.rs`
- Modify: `crates/nyon-workshop-core/src/living/fleet.rs`
- Modify: `crates/nyon-workshop-core/src/living/freight.rs`
- Modify: `crates/nyon-workshop-core/src/living/intelligence.rs`
- Create: `crates/nyon-workshop-core/tests/living_strategy_fleet.rs`
- Create: `crates/nyon-workshop-core/tests/living_receipts.rs`

**Interfaces:**
- Consumes: current observations, policies, wars/colonies/fleets, demand requests derived from the receiver's own state, validated split/order/route primitives, and schema-frozen autonomous intent event payloads.
- Produces: one reserved fleet intent per eligible actor per 50-tick boundary, dormant fleet-only recovery, AI-created internal/trade/aid routes, and complete structured decision receipts.

- [ ] **Step 1: Write failing cadence and priority tests**

```rust
#[test]
fn active_actor_accepts_at_most_one_economic_and_one_fleet_intent_at_fifty() {
    let mut fixture = common::living::many_candidate_strategy_fixture();
    let receipt = fixture.step_to(50).unwrap();
    assert_eq!(common::living::accepted_economic_intents(&receipt), 1);
    assert_eq!(common::living::accepted_fleet_intents(&receipt), 1);
}

#[test]
fn war_at_one_hundred_orders_attack_but_departure_waits_until_101() {
    let mut fixture = common::living::war_cadence_fixture();
    let at_100 = fixture.step_to(100).unwrap();
    assert!(common::living::has_event(&at_100, "war_declared"));
    assert!(common::living::has_event(&at_100, "autonomous_intent_accepted"));
    assert!(!common::living::has_event(&at_100, "fleet_departed"));
    let at_101 = fixture.step_to(101).unwrap();
    assert!(common::living::has_event(&at_101, "fleet_departed"));
}

#[test]
fn dormant_actor_runs_only_existing_fleet_recovery() {
    let mut fixture = common::living::dormant_credited_ark_fixture();
    let receipt = fixture.step_to_strategy_boundary().unwrap();
    assert_eq!(common::living::accepted_economic_intents(&receipt), 0);
    assert_eq!(common::living::accepted_fleet_intents(&receipt), 1);
    assert!(fixture.has_no_new_asset_job());
}
```

Add `defense_precedes_retreat_then_settlement_attack_explore_reinforce`, `scout_prefers_never_observed_then_oldest_then_route_then_system`, `attack_rejects_stale_or_unobserved_target`, and `role_split_and_order_share_one_reservation`.

- [ ] **Step 2: Write failing demand/privacy and receipt tests**

```rust
#[test]
fn trader_uses_receiver_demand_request_not_private_inventory() {
    let base = common::living::trade_demand_fixture(25, 9_000);
    let mut hidden_changed = base.clone();
    hidden_changed.change_rival_private_inventory(1);
    assert_eq!(base.next_trader_decision().unwrap(), hidden_changed.next_trader_decision().unwrap());
}

#[test]
fn accepted_and_rejected_intents_record_complete_nonfabricated_reasons() {
    let receipt = common::living::contended_strategy_fixture().step().unwrap();
    let accepted = common::living::accepted_decision(&receipt);
    assert_eq!(accepted.score(), 70);
    assert_eq!(accepted.observed_tick(), Some(LivingTickV2(49)));
    assert_eq!(accepted.reason_code(), "receiver_below_needed_resource");

    let rejected = common::living::rejected_decision(&receipt);
    assert_eq!(rejected.reason_code(), "shared_reservation_lost");
    assert_ne!(accepted.actor(), rejected.actor());
}
```

Add `chronicle_event_provenance_distinguishes_autonomous_from_creator` and `no_reason_field_is_populated_when_authority_recorded_none`.

- [ ] **Step 3: Run both focused suites and verify failures**

Run: `cargo test -p nyon-workshop-core --test living_strategy_fleet --test living_receipts`

Expected: FAIL because fleet candidate ordering, exchanged demand requests, and complete structured reasons are absent.

- [ ] **Step 4: Implement fleet candidates and stable reservations**

First take actionable defense of an attacked owned colony, then automatic noncombatant retreat. Otherwise score settlement 60, authorized attack 60, exploration 50, and reinforcement 40, applying only relevant +20 policy bonuses. Sort ties by target ID and fixed action ordinal. A chosen role may split stationary co-located eligible hulls and issue the resulting order as one atomic reserved intent.

Dormant actors skip economy and asset creation. They may complete/return current travel and, only with an ark, known unowned destination, and sufficient existing return credits, reserve one settlement itinerary. This spends no created energy or new credit.

- [ ] **Step 5: Implement explicit receiver demand exchange and decision records**

A receiver derives a demand request from its own projected state. Only that request, contracted freight, published observations, and bilateral permission enter the sender snapshot. Internal routes need a useful source surplus above reserve and receiver deficit. Trade needs a current agreement. Autonomous Aid requires Trader policy and a nonhostile neighbor.

Every accepted/rejected decision event carries actor, target, action kind, tier, score, relevant observation tick or null, and one schema-defined structured reason. Never derive reason text from names, animations, later results, or hidden creator truth.

- [ ] **Step 6: Run strategy, intelligence, diplomacy, and receipt suites**

Run: `cargo test -p nyon-workshop-core --test living_strategy_fleet --test living_receipts --test living_intelligence --test living_diplomacy`

Expected: PASS; all eleven fleet/receipt tests pass, and hidden rival inventory changes no actor decision.

- [ ] **Step 7: Commit autonomous fleet strategy and receipts**

```bash
git add crates/nyon-workshop-core/src/living/strategy.rs \
  crates/nyon-workshop-core/src/living/fleet.rs \
  crates/nyon-workshop-core/src/living/freight.rs \
  crates/nyon-workshop-core/src/living/intelligence.rs \
  crates/nyon-workshop-core/tests/living_strategy_fleet.rs \
  crates/nyon-workshop-core/tests/living_receipts.rs
git commit -m "feat(living): add explainable fleet strategy"
```

### Task 11: Freeze the Four Strategic Authority Fixtures

**Files:**
- Modify: `crates/nyon-workshop-core/tests/common/mod.rs`
- Modify: `crates/nyon-workshop-core/tests/common/living.rs`
- Create: `crates/nyon-workshop-core/tests/fixtures/living-v2/first-expansion-v2.json`
- Create: `crates/nyon-workshop-core/tests/fixtures/living-v2/commerce-v2.json`
- Create: `crates/nyon-workshop-core/tests/fixtures/living-v2/frontier-friction-v2.json`
- Create: `crates/nyon-workshop-core/tests/fixtures/living-v2/interrupted-corridor-v2.json`
- Create: `crates/nyon-workshop-core/examples/freeze_living_fixtures.rs`
- Create: `crates/nyon-workshop-core/tests/living_strategic_fixtures.rs`

**Interfaces:**
- Consumes: `assets/living/core-pack-v2.json`, `LivingGenesisManifestV2`, `LivingGalaxyAuthorityV2`, and canonical state/receipt digests.
- Produces: four strict canonical fixtures with catalog hash, seed, generator, full genesis, ordered witnesses, and frozen digest checkpoints; `freeze_living_fixtures --check|--write`.

- [ ] **Step 1: Define the strict envelope and failing test**

```rust
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct LivingAuthorityFixtureV2 {
    fixture_id: String,
    fixture_version: u32,
    seed_hex: String,
    catalog_hash: LivingCatalogHashV2,
    generator: LivingGenesisGeneratorV2,
    genesis_manifest: LivingGenesisManifestV2,
    witnesses: Vec<LivingFixtureWitnessV2>,
    checkpoints: Vec<LivingFixtureCheckpointV2>,
}

#[test]
fn strategic_fixtures_are_canonical_and_self_verifying() {
    for bytes in common::living::strategic_fixture_bytes() {
        let fixture = common::living::decode_strategic_fixture(bytes).unwrap();
        common::living::replay_and_verify_fixture(&fixture).unwrap();
    }
}
```

Reject unknown/duplicate fields, non-2 version, noncanonical seed/hash, unsorted/duplicate witnesses or checkpoints, catalog mismatch, invalid genesis, missing witness, and replayed digest mismatch.

- [ ] **Step 2: Run and verify missing fixtures**

Run: `cargo test -p nyon-workshop-core --test living_strategic_fixtures`

Expected: FAIL because the four files and loader do not exist.

- [ ] **Step 3: Encode these exact manifests and witnesses**

| Fixture | Seed | Exact content/result |
| --- | --- | --- |
| `nyon.first-expansion.v2` | `0x4E594F4E5F455850` | Two systems, distance-100 lane, one Neutral actor, one home and one unowned frontier. Home: hub/solar/extractor/shipyard, 100 energy, 100 ore, 120 alloy, 20,000 ore reserve, scout plus two escorts, no foundry/ark. Foundry accept/complete 50/250; scout order/depart/arrive-observe 50/51/61; ark accept/complete 100/300; settle order/depart/arrive/claim 300/301/311/361; verify through 411. |
| `nyon.commerce.v2` | `0x4E594F4E5F434F4D` | Adjacent actors, both relations 20, Trader surplus above reserve and receiver ore demand, no agreement/route/hazard/war. Trade starts by 100; first cross-actor unload and delivery credit by 200. |
| `nyon.frontier-friction.v2` | `0x4E594F4E5F574152` | Adjacent colonies, relations -40, fresh observations, no agreement/truce/war, eligible assembled attack. War by 100, departure no earlier than 101, hostile arrival/combat by 200, continued simulation through 500. |
| `nyon.interrupted-corridor.v2` | `0x4E594F4E5F494F4E` | Direct-lane ore route batch 10/reserve 20 feeds a dependent foundry. Ion storm begins before 50. By 200, affected dispatch is halved/zero and a due foundry records input wait without compensation. |

All unrelated actors, repairs, and recipes are absent or disabled. Use ordinary built-in rules, never fixture-only switches.

- [ ] **Step 4: Implement and run the freeze tool**

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = Mode::parse(std::env::args().skip(1))?;
    for spec in fixture_specifications()? {
        let frozen = run_and_capture(spec)?;
        match mode {
            Mode::Check => compare_existing_canonical_bytes(&frozen)?,
            Mode::Write => write_exact_canonical_bytes(&frozen)?,
        }
    }
    Ok(())
}
```

`run_and_capture` validates through `from_genesis`, steps the public authority, selects only actual ordered receipt witnesses, and records digests at every witness/final boundary.

Run: `cargo run -p nyon-workshop-core --example freeze_living_fixtures -- --write`

Expected: four canonical files are created. Inspect their complete diff and exact table contents.

- [ ] **Step 5: Verify bytes, witnesses, and absence of scripted client events**

Run: `cargo run -p nyon-workshop-core --example freeze_living_fixtures -- --check && cargo test -p nyon-workshop-core --test living_strategic_fixtures`

Expected: PASS with unchanged bytes and reproduced witnesses/digests.

Run: `rg -n "foundry job accepted|claim completed|trade agreement|war declared|shipment delivered" src assets/living crates/nyon-workshop-core/tests/fixtures/living-v2`

Expected: app/UI code never constructs `LivingEventV2`, alters receipts, or enqueues autonomous commands.

- [ ] **Step 6: Commit fixtures**

```bash
git add crates/nyon-workshop-core/tests/common/mod.rs \
  crates/nyon-workshop-core/tests/common/living.rs \
  crates/nyon-workshop-core/tests/fixtures/living-v2/first-expansion-v2.json \
  crates/nyon-workshop-core/tests/fixtures/living-v2/commerce-v2.json \
  crates/nyon-workshop-core/tests/fixtures/living-v2/frontier-friction-v2.json \
  crates/nyon-workshop-core/tests/fixtures/living-v2/interrupted-corridor-v2.json \
  crates/nyon-workshop-core/examples/freeze_living_fixtures.rs \
  crates/nyon-workshop-core/tests/living_strategic_fixtures.rs
git commit -m "test(living): freeze strategic civilization fixtures"
```

### Task 12: Prove Creator and Civilization Authority Stay Distinct

**Files:**
- Modify: `crates/nyon-workshop-core/src/living/command.rs`
- Modify: `crates/nyon-workshop-core/src/living/civilization.rs`
- Modify: `crates/nyon-workshop-core/src/living/economy.rs`
- Modify: `crates/nyon-workshop-core/src/living/fleet.rs`
- Modify: `crates/nyon-workshop-core/src/living/freight.rs`
- Modify: `crates/nyon-workshop-core/src/living/diplomacy.rs`
- Create: `crates/nyon-workshop-core/tests/living_creator_civilizations.rs`

**Interfaces:**
- Consumes: authority-frozen `LivingCommandV2` and validate-project-commit plus Tasks 2-10 validation primitives.
- Produces: free creator semantics for civilization objects and complete atomic dependent cascades.

- [ ] **Step 1: Write table-driven free-but-valid tests**

```rust
#[test]
fn creator_operations_are_free_but_never_bypass_structure_or_capacity() {
    for case in common::living::creator_civilization_cases() {
        let mut fixture = case.fixture_with_zero_actor_resources();
        let accepted = fixture.submit_creator(case.valid_command.clone()).unwrap();
        assert_eq!(fixture.last_creator_event_provenance(), LivingProvenanceV2::Creator);
        assert_eq!(
            fixture.actor_resources(),
            LivingInventoryV2 { energy: 0, ore: 0, alloy: 0 },
        );
        let before = fixture.authority_bytes();
        assert_eq!(fixture.submit_creator(case.invalid_command), Err(case.expected_rejection));
        assert_eq!(fixture.authority_bytes(), before);
    }
}
```

Cases cover colony ownership, facilities/queues, policy/relation/agreements, fleets/hulls/orders, routes, and waiting cargo. Invalid cases pin slot, cap, reference, co-location, transit, reservation, intelligence, ownership, and bilateral errors.

- [ ] **Step 2: Add cascade atomicity tests**

```rust
#[test]
fn incomplete_world_removal_changes_nothing() {
    let mut fixture = common::living::world_with_all_dependents_fixture();
    let before = fixture.authority_bytes();
    assert!(matches!(
        fixture.submit_creator(common::living::incomplete_world_removal()),
        Err(LivingCommandRejectionV2::BlockingDependencies { .. })
    ));
    assert_eq!(fixture.authority_bytes(), before);
}

#[test]
fn complete_world_removal_records_every_disposition() {
    let mut fixture = common::living::world_with_all_dependents_fixture();
    let receipt = fixture.submit_creator(common::living::complete_world_removal()).unwrap();
    assert!(fixture.has_no_dangling_references());
    assert!(common::living::creator_events_name_all_retired_dependents(&receipt));
}
```

- [ ] **Step 3: Run and verify the first missing operation**

Run: `cargo test -p nyon-workshop-core --test living_creator_civilizations`

Expected: FAIL on an unimplemented domain operation, creator path charging civilization resources, or incomplete prevalidation.

- [ ] **Step 4: Reuse validation without payment**

Factor structural/reference/capacity validation from application. Creator application uses authority-preallocated IDs and `CreatorIntervention` without deduction/refund or autonomous reason. Civilization application reuses validation, then reserve/pay/apply with autonomous provenance. Forced bilateral edits remove incompatible records atomically; forced peace prevents same-boundary damage and resets occupation.

- [ ] **Step 5: Run and commit**

Run: `cargo test -p nyon-workshop-core --test living_creator_civilizations --test living_command_queue --test living_history --test living_economy --test living_fleet --test living_freight --test living_diplomacy --test living_occupation`

Expected: PASS; invalid cases preserve bytes/tail, and accepted revisions replay identically.

```bash
git add crates/nyon-workshop-core/src/living/command.rs \
  crates/nyon-workshop-core/src/living/civilization.rs \
  crates/nyon-workshop-core/src/living/economy.rs \
  crates/nyon-workshop-core/src/living/fleet.rs \
  crates/nyon-workshop-core/src/living/freight.rs \
  crates/nyon-workshop-core/src/living/diplomacy.rs \
  crates/nyon-workshop-core/tests/living_creator_civilizations.rs
git commit -m "feat(living): separate creator and civilization authority"
```

### Task 13: Run Determinism, Replay, Capacity, Endurance, and Legacy Gates

**Files:**
- Create: `crates/nyon-workshop-core/tests/living_phase_order.rs`
- Create: `crates/nyon-workshop-core/tests/living_capacity.rs`
- Create: `crates/nyon-workshop-core/tests/living_determinism.rs`
- Create: `crates/nyon-workshop-core/tests/living_endurance.rs`
- Modify: `crates/nyon-workshop-core/tests/living_strategic_fixtures.rs`

**Interfaces:**
- Consumes: completed authority/civilization plans, canonical archive/replay, exact rule fixtures, and preexisting V1 goldens.
- Produces: automated LG-07 through LG-13 civilization evidence plus relevant LG-17/LG-20/LG-21/LG-24/LG-26/LG-27 evidence; no live runtime claim.

**Exact rules-fixture coverage:**

| Rules Section 9 fixture | Owning test task |
| --- | --- |
| Building timing; Hub recovery; Atomic shortage; Blocked cadence; Storage full; Genesis clocks | Task 2 `living_economy` |
| Capacity reservation; Hull delivery reservation; Repair clock | Task 3 `living_construction` |
| Fleet fuel; Travel; Fleet/ship capacity; Retreat target | Task 4 `living_fleet` and Task 13 `living_capacity` |
| Captured freight; Suspended route; Waiting capture; Waiting war return; War return; Full receiver | Task 5 `living_freight` |
| Hidden information; Claim race; Claim close boundary | Task 6 `living_intelligence` and `living_settlement` |
| Treaty boundary; AI cadence | Task 7 `living_diplomacy` and Task 10 `living_strategy_fleet` |
| Simultaneous battle; Occupation; Dormancy; Dormant recovery | Task 8 `living_combat` and `living_occupation` |
| Creator atomicity | Task 12 `living_creator_civilizations` |
| Replay | Task 13 `living_determinism` |
| Legacy | Task 13's explicit V1 suites |

- [ ] **Step 1: Add ten-phase order and rollback tests**

```rust
#[test]
fn one_boundary_observes_the_normative_phase_order() {
    let receipt = common::living::all_phases_due_fixture().step_once().unwrap();
    assert!(common::living::event_kinds_are_in_order(&receipt, &[
        "creator_intervention", "agreement_expired", "fleet_arrived",
        "shipment_delivered", "combat_damage", "occupation_advanced",
        "construction_completed", "recipe_produced", "repair_applied",
        "observation_refreshed", "agreement_started",
        "autonomous_intent_accepted", "fleet_departed", "shipment_dispatched",
    ]));
}

#[test]
fn capacity_or_invariant_fault_preserves_the_complete_boundary() {
    for mut fixture in common::living::fault_fixtures() {
        let before = fixture.authority_bytes();
        assert!(fixture.step_once().is_err());
        assert_eq!(fixture.authority_bytes(), before);
    }
}
```

The queue-fault case proves neither tick nor queued revision commits, the diagnostic queue remains, and no partial event publishes.

- [ ] **Step 2: Add pacing, insertion-order, and split-replay tests**

```rust
#[test]
fn pacing_and_insertion_order_do_not_change_36000_tick_result() {
    let direct = common::living::run_mixed_fixture(36_000, &[1]).unwrap();
    let paced = common::living::run_mixed_fixture(36_000, &[1, 4, 20, 4, 1, 20]).unwrap();
    let reversed = common::living::run_reversed_manifest_fixture(36_000).unwrap();
    assert_eq!(direct.state_digest(), paced.state_digest());
    assert_eq!(direct.receipt_chain_digest(), paced.receipt_chain_digest());
    assert_eq!(direct.state_digest(), reversed.state_digest());
}

#[test]
fn save_at_18000_then_continue_equals_uninterrupted_36000() {
    let uninterrupted = common::living::run_mixed_fixture(36_000, &[20]).unwrap();
    let halfway = common::living::run_mixed_fixture(18_000, &[20]).unwrap();
    let resumed = common::living::continue_to(
        common::living::encode_then_decode_authority(&halfway).unwrap(), 36_000,
    ).unwrap();
    assert_eq!(uninterrupted.canonical_state_bytes(), resumed.canonical_state_bytes());
    assert_eq!(uninterrupted.state_digest(), resumed.state_digest());
}
```

Compare queues/reservations, observations, relations/partial deliveries, agreements/wars/truces, fleet legs/credits/orders, shipment disposition, claims, occupations, clocks, sequences, and decision cadence.

- [ ] **Step 3: Add cap and 100,000-tick endurance tests**

Test each civilization collection immediately below and at its cap. Existing objects still work while a new reservation/dispatch rejects uncharged. Run the 64-system/512-world/16-civilization capacity fixture for 100,000 ticks in release mode, validate invariants every 100 ticks, and record bounded peak event/history retention.

Run: `cargo test --release -p nyon-workshop-core --test living_endurance -- --nocapture`

Expected: PASS and print host, fixture/catalog/seed, 100,000 completed ticks, final state/receipt-chain digests, and peak counts. Timing is measurement evidence, not cross-host 5 ms p95 proof.

- [ ] **Step 4: Run every exact rule and strategic suite**

```bash
cargo test -p nyon-workshop-core \
  --test living_economy --test living_construction --test living_fleet \
  --test living_freight --test living_intelligence --test living_settlement \
  --test living_diplomacy --test living_combat --test living_occupation \
  --test living_strategy_economy --test living_strategy_fleet \
  --test living_receipts --test living_strategic_fixtures \
  --test living_creator_civilizations --test living_phase_order \
  --test living_capacity --test living_determinism
```

Expected: PASS, covering every rules Section 9 fixture, four strategic witnesses, atomic creator/civilization distinction, and zero-terminal-state behavior.

- [ ] **Step 5: Run full source, wasm, fuzz-workspace, and V1 gates**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
cargo fmt --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all --check
cargo clippy --locked --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all-targets -- -D warnings
cargo test -p nyon-workshop-core --test pack --test archive --test simulation --test two_system_forge
cargo test --test rules_v1_facade --test campaign --test scenario
git diff --check
```

Expected: PASS with current counts recorded. V1 bytes/digests remain unchanged. Fuzz commands establish compile/lint only; wasm establishes compile only.

- [ ] **Step 6: Commit deterministic qualification tests**

```bash
git add crates/nyon-workshop-core/tests/living_phase_order.rs \
  crates/nyon-workshop-core/tests/living_capacity.rs \
  crates/nyon-workshop-core/tests/living_determinism.rs \
  crates/nyon-workshop-core/tests/living_endurance.rs \
  crates/nyon-workshop-core/tests/living_strategic_fixtures.rs
git commit -m "test(living): qualify civilization determinism"
```

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-09-04-nyon-living-galaxy-civilizations.md`. The authority plan lands first. The client/experience plan then consumes the schema-frozen receipts, inspection state, and four fixtures; whole-product qualification follows both.

Two execution options:

1. **Subagent-Driven (recommended)** - Dispatch one fresh worker per task, review its exact diff and focused tests, and merge into canonical `main` before the next dependent task.

2. **Inline Execution** - Use superpowers:executing-plans in the canonical checkout, execute tasks in order, and pause after each commit for contract and regression review.
