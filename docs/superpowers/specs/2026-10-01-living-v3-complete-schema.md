# Living V3 complete schema and compatibility supplement

Date: 2026-10-03. Status: **PROPOSED coordinated review submission2; independent integrated review, fresh complete vectors and human approval pending**.
Parent: `2026-10-01-living-expansion-wire.md`. This document is a written design,
not codec code, V2 changes, or acceptance. Existing society/ecology/diplomacy
numerical formulas remain in their respective Proposed specifications.

## Normative notation, scalar grammar and canonical ordering

`ID` is exactly 16 bytes/32 lowercase hex digits. `Digest`, `Seed`, and
`RevisionID` are exactly 32 bytes/64 lowercase hex digits. `BranchID` and
`EventID` are ID-width but separately typed. `Tick` and every u64 encode as
canonical unsigned decimal strings. `Score` and i32 encode canonical signed
decimal strings in their exact Rust-width range, never -0. u16/u32 encode JSON
integers. bool is a JSON boolean. `Option<T>` is explicitly null or T; every
field is required, even when null/false/empty. There are no omitted defaults.
`Vec` preserves order; `Sorted` is a strictly sorted unique-key JSON array.
Records encode fields in declaration order. A tagged enum encodes `type` first,
then its listed payload fields; tags are snake_case of the listed variant name.
Unit enums encode just their snake_case string. Tuple enum payloads use the
field `value`. No unspecified variants, maps, arbitrary JSON or extension bags
are accepted. ASCII tags sort by bytes; ID keys sort by decoded bytes.

Slug: `[a-z][a-z0-9_]{0,63}`. Name: 1..64 Unicode scalars and at most256 UTF-8
bytes, no controls U+0000..001F/U+007F..009F, no leading/trailing U+0020 or doubled
U+0020. No normalization. Canonical JSON writes literal Unicode UTF-8; escapes
only quote/backslash and control bytes as lowercase `\u00xx`. Reject alternate
escape spellings by byte-identical re-encoding. Name's Unicode allowance is a
new Proposed V3 decision, not a relaxation of V2's ASCII contract.

All entity references have a statically required record kind. Colony identity
is its world ID; relation/agreement/observation/claim/occupation keys are typed
composite keys, never fabricated extra entity IDs. A foreign kind with matching
bytes rejects. All arithmetic and length sums are checked before allocation.

## Capacities and exact keys

| Collection | Maximum | Strict key |
| --- | ---: | --- |
| systems/stars/worlds/lanes | 64/128/512/256 | id |
| civilizations/deposits/facilities | 16/1024/2048 | id |
| colonies |512 | world |
| construction_jobs/hull_jobs |4096/512 | id |
| fleets/hulls |256/2048 globally;16 hulls/fleet | id |
| routes/shipments |2048/4096 | id |
| relations |240 | (from,to) |
| agreements |256 shared typed treaty union (see below) | (participant_a,participant_b,kind) |
| wars |120 | (participant_a,participant_b) |
| observations |8192 | (civilization,world) |
| hazards/stellar_events |128 each pending/active | id |
| settlement_claims/occupations |8192/512 | (world,civilization)/world |
| offers |64 globally,1 outgoing/actor | id |
| commitments |256 shared typed treaty union (see below) | id |
| demand_requests |1024,1 per(actor,world,resource) | (actor,world,resource) |
| defense_requests |256,1 per commitment | id |
| shared_intelligence |4096 | (recipient,originating_actor,world) |
| branches/revisions/bookmarks/experiments |128/10000/1024/256 | id/id/ordinal/ordinal |

Completed facilities and construction-job reservations count toward the eight
colony industrial slots. Hull jobs reserve shipyard concurrency, global hull
capacity and target-fleet hull/destination capacity separately; they consume no
additional industrial slot. A staffed shipyard in a full eight-facility colony
can admit a hull job if those separate reservations permit it. Completion moves
construction reservation to facility occupancy without a second slot; hull
completion moves its reserved hull/fleet capacity to actual occupancy. Cancellation
releases only the respective reservation and applies the inherited half-refund
rule. A shipyard accepts at most one hull job. Paths contain at most256 unique lanes;
resource arrays at most4 entries; staff arrays at most8 unique facility IDs;
technology sets/progress at most64 entries. Catalog maxima: star/world archetypes
64 each; resources exactly energy/ore/alloy/food; industry/hull/hazard definitions
64/32/32; policies32; technologies64. Pending creator batches at most64, each
1..1024 operations and at most8 MiB canonical bytes. A queue's retained command
bytes cap is16 MiB. A cascade/evidence cannot exceed its enclosing byte cap.
The newly selected capacities are technical proposals, listed in the parent.
No capacity exhaustion silently truncates canonical records. Actor-only
KnownWorldGeometry key id/cap512, own_deposits id/1024, own_settlement_claims
(world,civilization)/8192 and own_occupations world/512 are declared below; they
add no State collection. The agreements/commitments256 limit is the typed union
with derived future Truce keys, not two independent allowances.

The V3 state field order is the parent's roster with `demand_requests` immediately
after `offers`. `tick:Tick`, `branch_sequence:u64`, `next_sequence:u64` lead it;
the following fields are Sorted arrays of the corresponding declared records.
`counters` is `{}`. `domain` in all domain fields is the integer3.
Sorted known event arrays key by id, research progress by technology,
metric/evidence/reference arrays by their declared ordinal or full Anchor key.
An EventReference sorts by (phase,ordinal,id). Each record family is defined by the frozen
inventory plus the mandatory overrides below; unused optional values remain null.

## Frozen inherited field inventory: catalog

The following is a copied declarative inventory, not an import or a Rust API.
V3-specific overrides below replace named records or append fields in the exact
stated location. `Living` prefixes are notation only. This frozen text does not
follow later V2 source edits. Source basis: `crates/nyon-workshop-core/src/living/catalog.rs`, inspected 2026-10-02.

```text
enum LivingIntentKindV3 {
    BuildArk,
    BuildBattery,
    BuildEscort,
    BuildExtractor,
    BuildFoundry,
    BuildScout,
    BuildShipyard,
    BuildSolarArray,
    Defend,
    EstablishTradeRoute,
    Settle,
    Supply,
}

record LivingStarArchetypeV3 {
    id: Slug,
    name: Name,
}

record LivingWorldArchetypeV3 {
    id: Slug,
    name: Name,
}

record LivingResourceDefinitionV3 {
    id: Slug,
    name: Name,
    storage_limit: u32,
}

record LivingResourceAmountV3 {
    resource: Slug,
    quantity: u32,
}

enum LivingIndustryEffectV3 {
    Recipe {
        period_ticks: u32,
        deposit_units: u32,
        inputs: Sorted<LivingResourceAmountV3>,
        outputs: Sorted<LivingResourceAmountV3>,
    },
    HullAssembly {
        concurrent_jobs: u32,
    },
    Defense {
        hit_points: u32,
        damage_per_round: u32,
    },
}

record LivingIndustryDefinitionV3 {
    id: Slug,
    name: Name,
    alloy_cost: u32,
    build_ticks: u32,
    slot_cost: u32,
    max_per_colony: u32,
    effect: LivingIndustryEffectV3,
}

record LivingHullDefinitionV3 {
    id: Slug,
    name: Name,
    alloy_cost: u32,
    build_ticks: u32,
    hit_points: u32,
    damage: u32,
}

enum LivingHazardEffectV3 {
    FreightBatchScale {
        numerator: u32,
        denominator: u32,
    },
}

record LivingHazardDefinitionV3 {
    id: Slug,
    name: Name,
    effect: LivingHazardEffectV3,
}

record LivingPolicyBonusV3 {
    intent: LivingIntentKindV3,
    bonus: u32,
}

record LivingPolicyDefinitionV3 {
    id: Slug,
    name: Name,
    bonuses: Sorted<LivingPolicyBonusV3>,
}
```

## Frozen inherited field inventory: model

The following is a copied declarative inventory, not an import or a Rust API.
V3-specific overrides below replace named records or append fields in the exact
stated location. `Living` prefixes are notation only. This frozen text does not
follow later V2 source edits. Source basis: `crates/nyon-workshop-core/src/living/model.rs`, inspected 2026-10-02.

```text
enum LivingResourceV3 {
    Alloy,
    Energy,
    Ore,
}

record LivingInventoryV3 {
    energy: u32,
    ore: u32,
    alloy: u32,
}

enum LivingBlockedReasonV3 {
    Disabled,
    MissingDeposit {
        required: u32,
        available: u32,
    },
    MissingInput {
        resource: LivingResourceV3,
        required: u32,
        available: u32,
    },
    OutputFull {
        resource: LivingResourceV3,
    },
}

record LivingSystemV3 {
    id: ID,
    name: Name,
}

record LivingStarV3 {
    id: ID,
    system: ID,
    archetype: Slug,
    name: Name,
}

record LivingWorldV3 {
    id: ID,
    system: ID,
    archetype: Slug,
    name: Name,
    owner: Option<ID>,
    inventory: LivingInventoryV3,
}

record LivingLaneV3 {
    id: ID,
    system_a: ID,
    system_b: ID,
    distance_units: u64,
}

enum LivingCivilizationStatusV3 {
    Active,
    Dormant,
}

record LivingCivilizationV3 {
    id: ID,
    name: Name,
    policy: Slug,
    status: LivingCivilizationStatusV3,
}

record LivingDepositV3 {
    id: ID,
    world: ID,
    resource: LivingResourceV3,
    remaining_units: u64,
}

record LivingHubV3 {
    energy_next_due: Tick,
    ore_next_due: Tick,
    fallback_next_due: Tick,
    blocked_reasons: Sorted<LivingBlockedReasonV3>,
}

record LivingColonyV3 {
    world: ID,
    owner: ID,
    hub: LivingHubV3,
}

enum LivingFacilityStatusV3 {
    Operational,
    Blocked,
}

record LivingFacilityV3 {
    id: ID,
    world: ID,
    definition: Slug,
    status: LivingFacilityStatusV3,
    paid_alloy: u32,
    hit_points: u32,
    next_due: Option<Tick>,
    next_repair: Option<Tick>,
    blocked_reasons: Sorted<LivingBlockedReasonV3>,
    disabled: bool,
}

record LivingConstructionJobV3 {
    id: ID,
    world: ID,
    owner: ID,
    definition: Slug,
    accepted_tick: Tick,
    completion_tick: Tick,
    paid_alloy: u32,
}

record LivingHullJobV3 {
    id: ID,
    world: ID,
    owner: ID,
    definition: Slug,
    accepted_tick: Tick,
    completion_tick: Tick,
    paid_alloy: u32,
    target_fleet: Option<ID>,
}

record LivingHullV3 {
    id: ID,
    definition: Slug,
    hit_points: u32,
    return_credit: bool,
    next_repair: Option<Tick>,
    passengers: Passengers,
}

enum LivingFleetLocationV3 {
    Docked {
        world: ID,
    },
    Travelling {
        origin_world: ID,
        destination_world: ID,
        departure_tick: Tick,
        arrival_tick: Tick,
    },
}

enum LivingFleetOrderKindV3 {
    Attack,
    Defend,
    Explore,
    Move,
    Return,
    Settle,
    Blockade,
}

record LivingFleetOrderV3 {
    kind: LivingFleetOrderKindV3,
    target_world: Option<ID>,
    not_before_tick: Tick,
    lane_path: Vec<ID>,
    objective: Option<ObjectiveKey>,
    defense_request: Option<DefenseActionBinding>,
}

record LivingFleetV3 {
    id: ID,
    owner: ID,
    location: LivingFleetLocationV3,
    order: Option<LivingFleetOrderV3>,
    hulls: Sorted<LivingHullV3>,
    supply: Supply,
    blockade: Option<Blockade>,
}

enum LivingRouteKindV3 {
    Aid,
    Internal,
    Trade,
}

record LivingFreightRouteV3 {
    id: ID,
    kind: LivingRouteKindV3,
    source_world: ID,
    destination_world: ID,
    resource: LivingResourceV3,
    batch_size: u32,
    cadence_ticks: u32,
    source_reserve: u32,
    source_owner: ID,
    receiver_owner: ID,
    next_due: Tick,
    suspended: bool,
    commitment: Option<ID>,
}

enum LivingShipmentDispositionV3 {
    Outbound,
    Returning,
    WaitingForStorage,
}

record LivingShipmentV3 {
    id: ID,
    dispatch_owner: ID,
    intended_receiver: ID,
    source_world: ID,
    destination_world: ID,
    resource: LivingResourceV3,
    units: u32,
    departure_tick: Tick,
    arrival_tick: Tick,
    original_travel_ticks: Tick,
    disposition: LivingShipmentDispositionV3,
    dispatch_kind: Option<LivingRouteKindV3>,
    returning_to_source: bool,
    dispatch_provenance: ManifestProvenance,
    creator_disposition_override: Option<CreatorCapture>,
}

record LivingRelationReasonsV3 {
    delivered_aid: i32,
    successful_trade: i32,
    adjacent_rival_settlement: i32,
    war_declared_against_it: i32,
    colony_captured_from_it: i32,
    aid_units_remainder: u32,
    trade_units_remainder: u32,
}

record LivingRelationV3 {
    from: ID,
    to: ID,
    base_disposition: i32,
    reasons: LivingRelationReasonsV3,
}

enum LivingAgreementKindV3 {
    Nonaggression,
    Trade,
    Truce,
}

record LivingAgreementV3 {
    kind: LivingAgreementKindV3,
    participant_a: ID,
    participant_b: ID,
    start_tick: Tick,
    end_tick: Tick,
}

record LivingWarV3 {
    participant_a: ID,
    participant_b: ID,
    declarer: ID,
    start_tick: Tick,
    end_tick: Tick,
}

record LivingObservedFacilityV3 {
    definition: Slug,
    count: u32,
}

record LivingObservedHullV3 {
    definition: Slug,
    count: u32,
}

record LivingObservationV3 {
    civilization: ID,
    world: ID,
    observed_tick: Tick,
    owner: Option<ID>,
    remaining_deposit: u64,
    facilities: Sorted<LivingObservedFacilityV3>,
    stationed_hulls: Sorted<LivingObservedHullV3>,
    operational_defense_batteries: Option<u16>,
}

record LivingHazardV3 {
    id: ID,
    definition: Slug,
    lane: ID,
    start_tick: Tick,
    end_tick: Tick,
}

record LivingSettlementClaimV3 {
    world: ID,
    civilization: ID,
    arrival_tick: Tick,
    close_tick: Tick,
}

record LivingOccupationV3 {
    world: ID,
    claimant: ID,
    progress_ticks: u32,
}

record LivingCountersV3 {}
```

## Frozen inherited field inventory: command

The following is a copied declarative inventory, not an import or a Rust API.
V3-specific overrides below replace named records or append fields in the exact
stated location. `Living` prefixes are notation only. This frozen text does not
follow later V2 source edits. Source basis: `crates/nyon-workshop-core/src/living/command.rs`, inspected 2026-10-02.

```text
enum LivingCommandTargetV3 {
    Local {
        local: u16,
    },
    Existing {
        id: ID,
    },
}

enum LivingCascadeDispositionV3 {
    RemoveListed {
        dependents: Vec<LivingCommandTargetV3>,
    },
    RejectIfReferenced,
    RemoveReviewed {
        dependents: Vec<LivingCommandTargetV3>,
        derived: Vec<LivingDerivedRemovalV3>,
    },
}

enum LivingDerivedRemovalV3 {
    Agreement {
        a: ID,
        b: ID,
        agreement_kind: LivingAgreementKindV3,
    },
    Colony {
        world: ID,
        owner: ID,
    },
    Observation {
        civilization: ID,
        world: ID,
    },
    Occupation {
        world: ID,
        claimant: ID,
    },
    Order { fleet: ID },
    Relation {
        from: ID,
        to: ID,
    },
    SettlementClaim {
        world: ID,
        civilization: ID,
    },
    War {
        a: ID,
        b: ID,
    },
}

enum LivingCreatorOperationV3 {
    CreateSystem {
        local: u16,
        name: Name,
    },
    CreateStar {
        local: u16,
        system: LivingCommandTargetV3,
        archetype: Slug,
        name: Name,
    },
    CreateWorld {
        local: u16,
        system: LivingCommandTargetV3,
        archetype: Slug,
        name: Name,
        owner: Option<LivingCommandTargetV3>,
        inventory: LivingInventoryV3,
    },
    CreateLane {
        local: u16,
        system_a: LivingCommandTargetV3,
        system_b: LivingCommandTargetV3,
        distance_units: u64,
    },
    CreateCivilization {
        local: u16,
        name: Name,
        policy: Slug,
        status: LivingCivilizationStatusV3,
    },
    CreateDeposit {
        local: u16,
        world: LivingCommandTargetV3,
        resource: LivingResourceV3,
        remaining_units: u64,
    },
    CreateFacility {
        local: u16,
        world: LivingCommandTargetV3,
        definition: Slug,
        status: LivingFacilityStatusV3,
        paid_alloy: u32,
        hit_points: u32,
        next_due: Option<Tick>,
    },
    CreateConstructionJob {
        local: u16,
        world: LivingCommandTargetV3,
        owner: LivingCommandTargetV3,
        definition: Slug,
        accepted_tick: Tick,
        completion_tick: Tick,
        paid_alloy: u32,
    },
    CreateHullJob {
        local: u16,
        world: LivingCommandTargetV3,
        owner: LivingCommandTargetV3,
        definition: Slug,
        accepted_tick: Tick,
        completion_tick: Tick,
        paid_alloy: u32,
        target_fleet: Option<LivingCommandTargetV3>,
    },
    CreateFleet {
        local: u16,
        owner: LivingCommandTargetV3,
        world: LivingCommandTargetV3,
    },
    CreateHull {
        local: u16,
        fleet: LivingCommandTargetV3,
        definition: Slug,
        hit_points: u32,
        return_credit: bool,
        passengers: PassengerCountInput,
    },
    CreateRoute {
        local: u16,
        kind: LivingRouteKindV3,
        source_world: LivingCommandTargetV3,
        destination_world: LivingCommandTargetV3,
        resource: LivingResourceV3,
        batch_size: u32,
        cadence_ticks: u32,
        source_reserve: u32,
        source_owner: LivingCommandTargetV3,
        receiver_owner: LivingCommandTargetV3,
        next_due: Tick,
    },
    CreateShipment {
        local: u16,
        dispatch_owner: LivingCommandTargetV3,
        intended_receiver: LivingCommandTargetV3,
        source_world: LivingCommandTargetV3,
        destination_world: LivingCommandTargetV3,
        resource: LivingResourceV3,
        units: u32,
        departure_tick: Tick,
        arrival_tick: Tick,
        original_travel_ticks: Tick,
        disposition: LivingShipmentDispositionV3,
        dispatch_kind: Option<LivingRouteKindV3>,
        returning_to_source: bool,
    },
    CreateHazard {
        local: u16,
        definition: Slug,
        lane: LivingCommandTargetV3,
        start_tick: Tick,
        end_tick: Tick,
    },
    SetWorldInventory {
        world: LivingCommandTargetV3,
        inventory: LivingInventoryV3,
    },
    SetWorldOwner {
        world: LivingCommandTargetV3,
        owner: Option<LivingCommandTargetV3>,
    },
    CreateColony {
        world: LivingCommandTargetV3,
        owner: LivingCommandTargetV3,
    },
    RemoveColony {
        world: LivingCommandTargetV3,
    },
    SetDepositRemaining {
        deposit: LivingCommandTargetV3,
        remaining_units: u64,
    },
    SetCivilizationPolicy {
        civilization: LivingCommandTargetV3,
        policy: Slug,
    },
    SetRelationBase {
        from: LivingCommandTargetV3,
        to: LivingCommandTargetV3,
        base_disposition: i32,
    },
    SetAgreement {
        kind: LivingAgreementKindV3,
        participant_a: LivingCommandTargetV3,
        participant_b: LivingCommandTargetV3,
        start_tick: Tick,
        end_tick: Tick,
    },
    RemoveAgreement {
        kind: LivingAgreementKindV3,
        participant_a: LivingCommandTargetV3,
        participant_b: LivingCommandTargetV3,
    },
    ForceWar {
        participant_a: LivingCommandTargetV3,
        participant_b: LivingCommandTargetV3,
        declarer: LivingCommandTargetV3,
        start_tick: Tick,
        end_tick: Tick,
    },
    ForcePeace {
        participant_a: LivingCommandTargetV3,
        participant_b: LivingCommandTargetV3,
    },
    SetFleetOrder {
        fleet: LivingCommandTargetV3,
        kind: Option<LivingFleetOrderKindV3>,
        target_world: Option<LivingCommandTargetV3>,
        not_before_tick: Tick,
        lane_path: Vec<LivingCommandTargetV3>,
        objective: Option<ObjectiveKey>,
        defense_request: Option<DefenseActionBinding>,
    },
    SetShipmentDisposition {
        shipment: LivingCommandTargetV3,
        disposition: LivingShipmentDispositionV3,
    },
    RemoveEntity {
        target: LivingCommandTargetV3,
        cascade: LivingCascadeDispositionV3,
    },
    EditSystem {
        target: LivingCommandTargetV3,
        name: Name,
    },
    EditStar {
        target: LivingCommandTargetV3,
        system: LivingCommandTargetV3,
        archetype: Slug,
        name: Name,
    },
    EditWorld {
        target: LivingCommandTargetV3,
        system: LivingCommandTargetV3,
        archetype: Slug,
        name: Name,
    },
    EditLane {
        target: LivingCommandTargetV3,
        system_a: LivingCommandTargetV3,
        system_b: LivingCommandTargetV3,
        distance_units: u64,
    },
    EditDeposit {
        target: LivingCommandTargetV3,
        resource: LivingResourceV3,
        remaining_units: u64,
    },
    EditCivilization {
        target: LivingCommandTargetV3,
        name: Name,
        policy: Slug,
    },
    SetFacilityEnabled {
        target: LivingCommandTargetV3,
        enabled: bool,
    },
    ScrapFacility {
        target: LivingCommandTargetV3,
        cascade: LivingCascadeDispositionV3,
    },
    CancelConstructionJob { target: LivingCommandTargetV3 },
    CancelHullJob { target: LivingCommandTargetV3 },
    EditFleet {
        target: LivingCommandTargetV3,
        owner: LivingCommandTargetV3,
        world: LivingCommandTargetV3,
    },
    EditHull {
        target: LivingCommandTargetV3,
        definition: Slug,
        hit_points: u32,
        return_credit: bool,
    },
    SplitFleet {
        local: u16,
        source: LivingCommandTargetV3,
        hulls: Vec<LivingCommandTargetV3>,
    },
    MergeFleets {
        target: LivingCommandTargetV3,
        source: LivingCommandTargetV3,
    },
    EditRoute {
        target: LivingCommandTargetV3,
        kind: LivingRouteKindV3,
        source_world: LivingCommandTargetV3,
        destination_world: LivingCommandTargetV3,
        resource: LivingResourceV3,
        batch_size: u32,
        cadence_ticks: u32,
        source_reserve: u32,
        source_owner: LivingCommandTargetV3,
        receiver_owner: LivingCommandTargetV3,
    },
    SetRouteSuspended {
        target: LivingCommandTargetV3,
        suspended: bool,
    },
    EditHazard {
        target: LivingCommandTargetV3,
        lane: LivingCommandTargetV3,
        definition: Slug,
        start_tick: Tick,
        end_tick: Tick,
    },
    CancelHazard { target: LivingCommandTargetV3 },
}

enum LivingCommandV3 {
    CreatorBatch {
        operations: Vec<LivingCreatorOperationV3>,
    },
}

enum LivingCommandModeV3 {
    Paused,
    Running,
}
```

## Frozen inherited field inventory: decision

The following is a copied declarative inventory, not an import or a Rust API.
V3-specific overrides below replace named records or append fields in the exact
stated location. `Living` prefixes are notation only. This frozen text does not
follow later V2 source edits. Source basis: `crates/nyon-workshop-core/src/living/decision.rs`, inspected 2026-10-02.

```text
enum LivingIntentActionV3 {
    Construction {
        world: ID,
        definition: Slug,
        scrap: Option<ID>,
    },
    Hull {
        world: ID,
        definition: Slug,
    },
    Route {
        source: ID,
        destination: ID,
        resource: LivingResourceV3,
        existing_route: Option<ID>,
    },
    Explore {
        fleet: ID,
        destination: ID,
    },
    Move {
        fleet: ID,
        destination: ID,
    },
    Settle {
        fleet: ID,
        destination: ID,
    },
    Defend {
        fleet: ID,
        world: ID,
    },
    Attack {
        fleet: ID,
        world: ID,
    },
    Return {
        fleet: ID,
        world: ID,
    },
    Agreement {
        peer: ID,
        agreement_kind: LivingAgreementKindV3,
    },
    War { peer: ID },
}

enum LivingDecisionTierV3 {
    PowerEmergency,
    OreEmergency,
    AlloyEmergency,
    Ordinary,
    Recovery,
}

enum LivingDecisionOutcomeV3 {
    Accepted,
    Rejected,
}

enum LivingDecisionReasonV3 {
    InsufficientResource {
        world: ID,
        resource: LivingResourceV3,
        required: u32,
        available: u32,
        reserve: u32,
    },
    OutputCapacity {
        world: ID,
        resource: LivingResourceV3,
        amount: u32,
        available_capacity: u32,
    },
    QueueCapacity {
        world: ID,
        queue: Slug,
        limit: u32,
        reserved: u32,
    },
    TargetReserved {
        target: ID,
        winner: ID,
    },
    LowerRank {
        winning_actor: ID,
        winning_ordinal: u16,
    },
    MissingIntelligence { world: ID },
    StaleObservation {
        world: ID,
        observed_tick: Tick,
        maximum_age: Tick,
    },
    InvalidTarget {
        target: ID,
        rule: Slug,
    },
    DormantRestriction { actor: ID },
    BilateralThreshold {
        peer: ID,
        forward: i32,
        reverse: i32,
        required: i32,
    },
    TruceActive {
        peer: ID,
        end_tick: Tick,
    },
    PolicyScore {
        policy: Slug,
        component: Slug,
        value: Score,
    },
    Forecast {
        world: ID,
        horizon: Tick,
        resource: LivingResourceV3,
        predicted: Score,
        demand: Score,
    },
    Accepted { rule: Slug },
}
```

## Mandatory V3 state overrides and new records

Notation `append` places fields at the end unless an insertion is named. These
are required fields, not backwards compatible V2 additions. Nested schemas:

```text
Environment { base_habitability:u32, stress:u32, fertility:u32,
              maximum_fertility:u32, next_environment_tick:Tick }
Needs { evaluated_tick:Tick, population_before:u32, food_need:u32,
        energy_need:u32, food_paid:u32, energy_paid:u32, population_after:u32,
        outcome:DemographicOutcome, causes:Sorted<DemographicCause> }
DemographicOutcome = growth | decline | steady
DemographicCause = food_shortage | energy_shortage | above_capacity |
                   growth_reserve_unavailable | zero_population | at_capacity |
                   fully_paid
Society { population:u32, specialization:Specialization, next_needs_tick:Tick,
          last_needs:Option<Needs>, workforce_assignments:Sorted<ID> }
Specialization = balanced | agrarian | industrial | research
Research { target:Option<Slug>, progress:Sorted<TechnologyProgress>,
           completed_technologies:Sorted<Slug> }
TechnologyProgress { technology:Slug, points:u32 }
RecipeFacts { evaluated_tick:Tick, staffed:bool, inputs:Sorted<ResourceAmount>,
              outputs:Sorted<ResourceAmount>, fertility_spent:u32,
              stress_added:u32, blocked:Sorted<LivingBlockedReasonV3> }
Passengers { count:u32, voyage_origin:Option<ID> }
PassengerCountInput { count:u32 }
Supply { last_success:Option<SupplyPayment>, last_attempt:Option<SupplyAttempt>,
         readiness:SupplyReadiness }
SupplyPayment { tick:Tick, world:ID, supplies_paid:u32, deadline:Tick }
SupplyAttempt { tick:Tick, world:ID, required:u32, paid:u32,
                outcome:SupplyAttemptOutcome }
SupplyAttemptOutcome = paid | unavailable
SupplyReadiness = supplied | supply_unavailable | never_supplied
Blockade { world:ID, war_a:ID, war_b:ID, accepted_tick:Tick }
ObjectiveKey { war_a:ID, war_b:ID, actor:ID }
DefenseActionBinding { request:ID, request_created_tick:Tick, commitment:ID,
                       commitment_created_tick:Tick, threat_observation_digest:Digest }
SecureEndpoints { world_a:ID, world_b:ID }
WarObjective { actor:ID, kind:ObjectiveKind, target:ID,
               secure_endpoints:Option<SecureEndpoints>,
               consecutive_witness_count:u32, status:ObjectiveStatus,
               last_evaluated_tick:Option<Tick>, witness:Option<Anchor> }
ObjectiveKind = defend | secure_access | occupy
ObjectiveStatus = pending | succeeded | invalid
Peace { proposer:ID, created_tick:Tick, considered_tick:Option<Tick>,
        status:PeaceStatus, reason:Option<PeaceReason> }
PeaceStatus = proposed | refused
PeaceReason = objective_complete | objective_invalid | force_unachievable |
              war_expired | objective_still_achievable
Terms = transit_access { participant_a:ID, participant_b:ID, duration_ticks:u32 }
      | resource_supply { source_world:ID, destination_world:ID,
          resource:LivingResourceV3, batch:u32, cadence_ticks:u32,
          source_reserve:u32, duration_ticks:u32 }
      | mutual_defense { participant_a:ID, participant_b:ID, duration_ticks:u32 }
Offer { id:ID, proposer:ID, recipient:ID, kind:TreatyKind, terms:Terms,
        created_tick:Tick, expires_tick:Tick, demand:Option<DemandKey>,
        renewal_of:Option<ID>, last_consideration:Option<OfferConsideration> }
OfferConsideration { tick:Tick, disposition:OfferDisposition,
                     reason:Option<ObligationReason> }
OfferDisposition = waiting | accepted | refused
TreatyKind = transit_access | resource_supply | mutual_defense
DemandKey { actor:ID, world:ID, resource:LivingResourceV3 }
DemandRequest { actor:ID, world:ID, resource:LivingResourceV3, units:u32,
                tick:Tick, expires_tick:Tick, snapshot_digest:Digest }
Commitment { id:ID, parties:Pair, kind:TreatyKind, terms:Terms,
             created_tick:Tick, expires_tick:Tick, obligation_status:Obligation }
Pair { participant_a:ID, participant_b:ID }
Obligation { status:ObligationStatus, last_attempt_tick:Option<Tick>,
             route:Option<ID>, request:Option<ID>,
             last_fulfillment:Option<Anchor>, reasons:Sorted<ObligationReason> }
ObligationStatus = ready | fulfilled | waiting | suspended
ObligationReason = capacity_unavailable | own_needs_reserve | no_supply |
                   endpoint_changed | war_incompatible | no_eligible_fleet |
                   prior_obligation | request_expired | target_invalid | relations_below_threshold |
                   no_connection | no_known_demand | expired | recipient_refused |
                   resource_reserved | missing_intelligence | stale_intelligence |
                   declaration_required
DefenseRequest { id:ID, commitment:ID, requester:ID, defender:ID,
                 target_world:ID, created_tick:Tick, expires_tick:Tick,
                 disposition:RequestDisposition, originating_observer:ID,
                 observed_tick:Tick, observation:ObservationFacts, threat:ObservedThreat }
RequestDisposition = pending | accepted | refused
ObservationFacts { owner:Option<ID>, remaining_deposit:u64,
     facilities:Sorted<LivingObservedFacilityV3>,
     stationed_hulls:Sorted<LivingObservedHullV3>,
     operational_defense_batteries:u16, environment:EnvironmentObservation,
     threats:Sorted<ObservedThreat>, hazard_scopes:Sorted<HazardScopeObservation> }
EnvironmentObservation { base_habitability:u32, stress:u32, fertility:u32,
     maximum_fertility:u32, stellar_scope:StellarScopeObservation }
StellarScopeObservation { system:ID, originating_observer:ID, observed_tick:Tick,
                         events:Sorted<KnownEnvironmentEvent> }
HazardScopeObservation { lane:ID, originating_observer:ID, observed_tick:Tick,
                        events:Sorted<KnownLaneHazard> }
KnownEnvironmentEvent { id:ID, star:ID, system:ID, warning_tick:Tick,
                        start_tick:Tick, end_tick:Tick, habitability_impact:u32,
                        freight_factor:Fraction, originating_observer:ID, observed_tick:Tick }
KnownLaneHazard { id:ID, definition:Slug, lane:ID, warning_tick:Tick,
                  start_tick:Tick, end_tick:Tick, freight_factor:Fraction,
                  originating_observer:ID, observed_tick:Tick }
SharedIntelligence { recipient:ID, originating_actor:ID, world:ID,
     observed_tick:Tick, received_tick:Tick, observation:ObservationFacts }
StellarEvent { id:ID, star:ID, warning_tick:Tick, start_tick:Tick, end_tick:Tick,
     habitability_impact:u32, freight_factor:Fraction, provenance:RecordProvenance }
RecordProvenance = genesis { generator:Generator, kind:u16, ordinal:u64 }
                 | creator { capture:CreatorCapture }
                 | autonomous { snapshot:SnapshotReference, phase:u16,
                      actor:Option<ID>, subject_kind:u16, subject:ID,
                      allocation_ordinal:u32 }
CreatorCapture { accepted_sequence:u64, operation_index:u32,
                 application_tick:Tick, mode:CommandMode, snapshot:SnapshotReference }
ObservedThreat { enemy:ID, target_world:ID, war:Option<Pair>,
                 observed_tick:Tick, originating_observer:ID,
                 observation_snapshot_digest:Digest,
                 hostile_escort_count:u32, occupation:bool,
                 hostile_fleets:Sorted<ID> }
CommitmentBinding { id:ID, created_tick:Tick, terms_digest:Digest }
ManifestProvenance = paid { snapshot:SnapshotReference, route:ID,
                           commitment:Option<CommitmentBinding>,
                           withdrawn:ResourceAmount }
                   | creator { capture:CreatorCapture }
Fraction { numerator:u32, denominator:u32 }
Anchor { domain:u32, catalog_hash:Digest, branch:BranchID,
         revision:Option<RevisionID>, tick:Tick, state_digest:Digest,
         receipt_digest:Digest, event:Option<EventReference> }
EventReference { phase:u16, ordinal:u32, id:EventID, kind:EventKind }
```

Pair always has distinct ascending IDs. All basis points, population and local
resource quantities are0..10000. The society/ecology formulas determine Needs,
assignments and recipe facts; forged or inconsistent derived facts reject on
replay. TechnologyProgress sorts by technology; quantities cannot exceed its
cost and every completed technology has exact cost progress. Workforce sorts
by facility ID; reasons sort by (tag,canonical payload bytes), with no duplicates.

- Resource adds `food`; Inventory appends `food:u32` after alloy.
- Star inserts `next_activity_tick:Tick` after name.
- World appends `environment:Environment` after inventory.
- Civilization inserts `research:Research` after policy, before status.
- Colony appends `society:Society` after hub.
- Facility appends `recipe_facts:Option<RecipeFacts>` after disabled; workforce
  availability is represented once in RecipeFacts and Society, not a second
  independently editable assignment. Passive facilities have null recipe facts.
- Hull, Fleet, Route and Shipment declarations above include their complete V3
  passenger, Supply, Blockade, Commitment, immutable-duration and provenance fields.
  Non-ark Hulls require count0/voyage_origin null. Treaty Supply requires exact binding.
  Shipment.original_travel_ticks is immutable, positive and at most u64::MAX;
  actual paid dispatch sets it to the checked original arrival-minus-departure duration.
  Active return clocks may change; neither current graph distance nor those changed
  clocks replace this original duration. The required scalar survives bounded archive
  retirement without retaining a capacity-sized dispatch owner merely for duration. Immutable paid dispatch
  bindings survive route/commitment removal as authenticated historical references.
  No current matching route can replace them. The override records a creator
  disposition intervention and remains set thereafter; it is not dispatch credit.
- War appends `objectives:Sorted<WarObjective>, peace:Option<Peace>` after end_tick.
  Objectives sort by actor, with exactly one per participant. Secure-access target
  is a Lane with required nonnull secure_endpoints; other targets are Worlds
  with secure_endpoints=null. Deadline is end_tick. Witness counts are0..3;
  Occupy uses0/1. Formation and pending rows require live correctly typed targets.
  Secure world_a belongs to Lane.system_a and world_b to Lane.system_b, in Lane
  declaration direction, not sorted World-ID order. The Worlds are live/distinct.
  Terminal invalid AND succeeded rows retain the exact original target IDs;
  Secure terminal rows retain their nonnull original pair even after deletion or
  relocation. Only these terminal objective positions may resolve historically:
  stage8 uses original typed issuance, stage10 authenticates original formation
  topology, exact instance and true invalidation/success plus later edits. No
  pending/command/candidate/World/Fleet/Shipment/Request reference gains this waiver.
  Terminal historical IDs grant no current movement or ownership. Explicit
  replacement requires live new targets/pair and resets the original instance.
- Observation replaces optional operational_defense_batteries with required u16
  and appends `environment:EnvironmentObservation, threats:Sorted<ObservedThreat>,
  hazard_scopes:Sorted<HazardScopeObservation>`.
  Own/visible observations
  never include rival inventory, needs, workforce or research.
- Hazard inserts `warning_tick:Tick` before start_tick.
- BlockedReason adds `workforce_unavailable { required:u32, available:u32 }`,
  `environmental_limit { effective_habitability:u32, fertility:u32 }`,
  `missing_research_target` and `technology_ineligible { technology:Slug }`.
- FleetOrderKind and FleetOrder above are complete. ObjectiveKey names the exact
  sorted War pair and actor; defense_request follows objective in every order carrier.
  Both options are required explicit null when absent. A nonnull defense_request is
  legal only for Defend and names the original accepted Request/C/threat instance;
  null orders and all other order kinds require null. No target resemblance creates
  a binding and no omitted-field default or cross-version conversion is permitted.
- Routes, deposits, paid jobs, hull repair, relation counters, settlement and
  occupation retain the frozen fields. No growth/cargo/passenger quantity is
  created by an omitted field.

Supply validity is explicit. A new or creator-cleared supply record has
last_success=null, last_attempt=null, readiness=never_supplied. A first failed
attempt has last_success=null and a nonnull unavailable attempt with paid0;
readiness=supply_unavailable. A subsequent failed attempt retains the previous
last_success unchanged and records its new required/paid0 attempt. Paid attempts
have paid=required=the live hull count and create last_success with that exact
world/tick/amount and deadline=tick+100. Empty Fleets reject in Genesis, completed
State, selected HistoryViews and ordinary autonomous sources. The sole nonempty
roster exception is a genuine unfinished creator operation prefix or its exact
operation-indexed Change endpoint, authenticated by the creator construction
contract below; no supplied flag or standalone imported State selects it.
Between attempts readiness is supplied only while tick<last_success.deadline;
at or after that deadline it becomes supply_unavailable, even without a new
attempt. Thus expired success with a prior paid attempt is representable.
An unavailable paid0 attempt creates no new successful payment and neither clears
nor extends an existing last_success. First failure keeps last_success null;
failed renewal keeps its prior payment byte-identical. Readiness derives only from
last_success and the current tick: supplied while a prior success deadline remains
in the future, supply_unavailable after expiry or when an attempt exists without
a success, and never_supplied only when both records are null. The unavailable
attempt remains visible even if an earlier payment has not yet expired.
A failed renewal never creates or extends a deadline. Supply.last_success.world
and Supply.last_attempt.world are independently authenticated historical World
positions. This is a NEW V3 reference exception covering an original paid success,
a genuine ordinary unavailable paid0 attempt, and an exact creator-set component.
At ordinary formation the World must be live at the actual supply source; at a
creator replacement the changed component's World must be live at that actual
CreatorCapture. Byte-identical retained components preserve their own original
source. A new absent-World template cannot claim retention by resembling old
bytes. Stage5 checks the full Supply shape/domains; stage8 resolves a live typed
World or this exact retained-component exception; stage10 replays its actual
formation, component replacements and deletion ancestry. Wrong-kind or never-issued
IDs have no exception. Prior success W1 and failed renewal W2 retain separate
sources even when either or both Worlds are deleted. An unavailable attempt is
not authenticated at W1's payment tick. Deleted World alone changes neither
component nor readiness, deadline, credits or payment. Explicit SetFleetSupply
clearing submits both options=null/readiness=never_supplied and emits its real
full Fleet Change; it is never inserted automatically. Creator paid-shaped
components remain Creator evidence and grant no actor-paid fulfillment.
Save, reopen, import and fork preserve these exact components and eligible original
ancestry without read-time migration, reset or timestamp refresh. History cache
retirement may regenerate authentic sources; it cannot prune required proof.

StellarEvent.provenance names the most recent authored edit or automatic creation,
not a recursive RevisionID. For creation/edit by operation i at accepted sequence S,
record provenance uses CreatorCapture(S,i,application_tick,mode,pre-command
snapshot). Those inputs exist before the new command/revision digest. The matching
receipt's creator provenance contains the eventual exact RevisionID; replay verifies
it has ordinal S, operation i and the same capture. Edit replaces record provenance
with the current capture while the ID and creation history remain stable. Genesis
stellar records use genesis provenance and the allocation roster, without a root
branch or future revision reference. No zero-digest placeholder or recursive
post-hash command mutation is permitted.

## Closed scale domains: applicable paths only

For the listed scalar pairs this Proposed contract selects:

```text
denominator: 1..=u32::MAX
numerator:   0..=denominator
```

Both are still JSON u32 integers. Numerator0 means complete suppression, equal
numerator/denominator means identity, and numerator>denominator is not an allowed
amplifying scale. Preserve both submitted literals: event2/4 and event1/2 have
different canonical bytes/digests although their integer outputs may be equal.
No gcd reduction, decimal conversion, denominator default or omitted zero field.

| Exact field-family paths | Domain and additional binding |
| --- | --- |
| `CreatorOperation.create_stellar_event.freight_factor.{numerator,denominator}` and `edit_stellar_event.freight_factor.*`; `State.stellar_events[i].freight_factor.*` | General nonamplifying pair above, including0; this creator freedom remains Proposed for human approval. Exact warning<=start<end, target/provenance,128-family cap and checked clocks still apply. |
| `Catalog.hazard_definitions[i].effect` tagged `freight_batch_scale`, inline `.numerator/.denominator` | Same structural pair predicate, then exact finite-profile equality: reviewed ion_storm is literal1/2. Neither0/1,2/4 nor any other mechanically equal spelling becomes a valid fixed Catalog row. |
| `Catalog.technology_definitions[i].effect` tagged `carrying_capacity_penalty_scale` or `departure_energy_scale`, inline `.numerator/.denominator` | Same pair predicate, then literal fixed equality: Resilient habitation1/2 and Efficient logistics3/4. No general Catalog permission for free departures or amplified effects follows. |
| `KnownEnvironmentEvent.freight_factor.*` in selected events/scopes, Observation.environment.stellar_scope.events, ObservationFacts.environment.stellar_scope.events and copied shared/request records | Domain above plus byte-exact source event fraction at original observed_tick. Replaying a source event1/2 cannot authenticate copied2/4, even if outputs coincide. Stale genuine2/4 stays2/4; hidden edits do not normalize/refresh it. |
| `KnownLaneHazard.freight_factor.*` in selected hazards/scopes, Observation.hazard_scopes.events, ObservationFacts.hazard_scopes.events and copied shared/request records | Domain above plus exact resolved hazard-definition fraction at original observation. Current fixed Catalog gives1/2. CreateHazard/EditHazard have no arbitrary fraction field; do not invent one to admit0 or2/4 lane hazards. |
| `Facts.dispatch.modifiers[i].numerator/.denominator` | Domain above, with exact source event/definition and complete applicable event-ID order authenticated by replay. This is an explanatory copy, not editable terms. |

Actor `known_stellar_events/known_stellar_scopes` and
`known_hazards/known_hazard_scopes`, DefenseRequest.observation and
SharedIntelligence.observation reuse those exact child types. Facts.stellar.record
and creator reviewed before/after StellarEvent snapshots likewise reuse the
State predicate. No duplicate source ID or unknown/unobserved child can be added
merely because its ratio is in range.

Automatic flares still produce literal1/2 with impact2000 and the exact existing
warning/start/end clocks. Creator2/4 is a creator event with original provenance;
it cannot masquerade as that autonomous output. Fixed Catalog metadata permission
does not extend to ratio spelling or quantity changes. A future differently
balanced rules version would need its own reviewed profile.

## ResourceAmount domains and empty-list meanings

Do not impose one positive-quantity predicate on every u32 in the schema. For
each ResourceAmount below, keep strict unique resource-slug ordering, maximum4
entries and exact resolved four-resource Catalog. The quantity family predicate
is semantic stage7; fixed Catalog equality and historical source equality are
additional checks, not replaced by width/positivity.

| Exact field-family paths | Quantity/list predicate (Proposed) |
| --- | --- |
| `Catalog.industry_definitions[i].effect` tagged `recipe`, `.inputs[j].quantity` | Listed entries1..=u32::MAX; inputs may be empty for the fixed solar recipe. An entry with0 is not another spelling of an absent input. Exact fixed quantities still apply. |
| Same `recipe` effect `.outputs[j].quantity` | Listed entries1..=u32::MAX and nonempty outputs for the reviewed material recipes. Fixed solar Energy8, extractor Ore4, foundry Alloy2 and farm Food20 remain exact. No zero-output/free-resource Catalog variant is approved. |
| `Catalog.industry_definitions[i].effect` tagged `research`, `.inputs[j].quantity` | Positive entries; exact nonempty Energy2/Alloy1 inputs. Points1 is its separate scalar, not a ResourceAmount output or a new resource slug. |
| `RecipeFacts.inputs[j].quantity`, including `State.facilities[i].recipe_facts` and `Facts.operation.recipe` | **Actual applied local debits**, positive entries only. Empty means no resource debit. On blocked attempts inputs=[]; positive required-but-unpaid costs remain in the exact Catalog/blocked or NeedsReserved reason, never false actual payment. Solar success has inputs=[]. |
| Same `RecipeFacts.outputs[j].quantity` | Actual positive material output entries, with the one explicit zero-result exception below. Lab success has outputs=[] because research points are recorded by Research state/facts. Passive facility recipe_facts=null; a fresh active facility has no fabricated prior operation. |
| `ManifestProvenance.paid.withdrawn.quantity` |1..=u32::MAX; resource and quantity equal the original immutable Shipment.resource/units and authenticated actual source withdrawal. Paid route dispatch is bounded further by its positive route batch1..10, stock/reserve and modifiers. No paid manifest for0. |
| `AdmissionTerms.movement.departure_payments[j].quantity` | Positive entries and resource=energy under the fixed departure rule. Exact computed total is one energy entry, or an empty list iff computed cost is0. No explicit energy0 entry, absent cost invented as0, or extra Ore/Food/Alloy cost. |
| `ForceWitness.required_departure_payments[j].quantity` in declarer/defender audit carriers | Same positive-energy/empty-if-computed0 representation; bind whole live hull roster, departure world, path and exact source formula/technology/credits. It describes potential departure funding, not actual movement or debit. |
| `DeclarationHold.resources[j].quantity` in `Facts.declaration_hold.record` | Same entries/list as that candidate's own declarer ForceWitness, with exact actor/fleet/world/snapshot/identity. Empty can still hold the fleet asset if a legal computed funding requirement is0. released=false/true facts must match; no debit/refund inferred. |

Construction/Hull alloy_paid, passenger/needs quantities, deposits and other u32
families do not inherit this table. In particular catalog Recipe.deposit_units=0
for solar/foundry is valid; scout/ark damage0, zero policy bonus, needs/payment0,
zero population/HP where the existing cause permits it, zero fertility/stress and
ResourceFact.quantity0 are not globally forbidden. Quantity positivity does not
relax fixed costs or create a new empty-input research recipe.

### Proposed RecipeFacts zero-output exception and cause

Use this canonical representation for the ecology's actually evaluated
zero-Food result:

```text
RecipeFacts.inputs = []
RecipeFacts.outputs = [{ resource:"food", quantity:0 }]
RecipeFacts.fertility_spent = 0
RecipeFacts.stress_added = 0
RecipeFacts.blocked = [environmental_limit with its exact evaluated operands]
Facts.operation.resources = [{ role:"output", resource:food, quantity:0 }]
EventKind = facility_blocked
```

This is a zero result, not inventory credit or a produced-event success. It is
allowed only for the reviewed Farm after the evaluated prefix below passes and proves
`min(floor(baseFood * effectiveHabitability / 10000), fertility)==0`.
baseFood is20 or completed Agronomy25, never a submitted claimed quantity.
No energy/fertility is consumed, no stress or research point is added and no
consumer benefit is earned. A zero row without this cause rejects; do not turn
OutputFull, absent target or unvisited formula into an Environmental limit witness.
An unrelated blocked attempt has inputs=[] and outputs=[] with its authentic
blockers and no invented zero material record. This is a Proposed reporting convention, not evidence of implemented runtime behavior.

The Proposed Farm evaluated prefix is literal, not an all-blockers scan:

1. First establish an ordinary applicable operation: an existing completed Farm
   with resolved active recipe/period, a canonical next_due<=the phase5 boundary,
   and the required habitable owned colony. Habitable here is the ordinary world/
   colony applicability predicate; effective environmental habitability0 does not
   turn an otherwise applicable Farm into an absent facility. Passive, unbuilt,
   removed, unresolved, not-yet-due or otherwise structurally inapplicable rows
   do not acquire a new attempt, clock update, RecipeFacts or event from this rule.
   Retain their earlier facts/clocks under their ordinary lifecycle.
2. For an ordinary due attempt, compute checked old_next_due+period once, before
   effects. Overflow faults the unpublished boundary; no wrapped/saturating clock
   or partial facts. Disabled behavior remains the existing declared Disabled
   branch ONLY where the ordinary recipe contract already attempts a disabled
   due row: no environmental evaluation or Food0; keep that branch's existing
   blocked/clock/event behavior. Do not invent a disabled attempt/reason/event for
   a row excluded at step1. The new environmental branch requires enabled=true
   (the retained facility's disabled=false).
3. Test its committed assignment at this operation time. Without the one required
   worker, blocked=[workforce_unavailable{required:1,available:0}], staffed=false,
   inputs=[], outputs=[], fertility_spent=stress_added=0. Emit the ordinary
   facility_blocked, evaluated_tick=current boundary and the checked clock. Do
   not visit environment, input/storage or NeedsReserved predicates.
4. With the worker, staffed=true. Resolve completed Agronomy and the exact current
   environment at this facility-ID position; compute the Farm formula above. If
   result0, blocked is EXACTLY the single environmental_limit cause, with Food0
   and its sole output ResourceFact as shown. Stop: input funding, output room and
   protected-needs/food-waiver checks are unvisited. Even Energy0, Food-full or
   held Energy cannot add MissingInput/OutputFull/needs_reserved to this result.
5. For positive computed Food q, keep ordinary atomic physical recipe validation:
   collect every failed required-input predicate in resource-slug order, then
   any applicable deposit predicate, then every failed output-room predicate in
   resource-slug order. A Farm has Energy2 input, no deposit debit, and Food q
   output. MissingInput reports actual physical stock (not stock minus needs
   holds); OutputFull means physical Food+q exceeds its declared storage cap.
   If any fail, blocked is the COMPLETE unique existing physical reason roster,
   sorted by the declared (tag,canonical payload bytes) key. For a Farm the only
   possible physical reasons are MissingInput{energy,2,physical_energy} and/or
   OutputFull{food}; no environmental_limit for a positive q. Stop before needs.
6. Only after physical validation passes, evaluate the existing due-needs guard.
   If Energy2 would spend held energy and the exact Food-producing waiver fails,
   blocked=[needs_reserved{world,resource:energy,physical_stock,held,required:2}],
   with its ordinary denied guard fact. Empty actual inputs/outputs, no resource
   debit or output ResourceFact. If the existing waiver passes, retain its
   food_waiver guard fact and execute normally; it is not an environmental bonus.
   Successful recipes have blocked=[], inputs=[Energy2], outputs=[Food q],
   fertility_spent=q and stress_added=0, plus ordinary actual debit/output facts.

At positive physical/needs denial, staffed=true, evaluated_tick=current boundary,
fertility_spent=stress_added=0, inputs=outputs=[] and no operation ResourceAmount/
ResourceFact claimed as paid/produced. The complete blocked reason array and
ordinary blocked/guard event still publish. All applicable due attempts above
advance the checked clock once, including staffing/physical/needs/environment
denial; excluded rows do not. For q>0 this retains the ordinary complete physical
blocker roster and canonical order, then the distinct protected-needs check. No
unvisited reason or evaluated quantity is synthesized. A shape-valid Food0 row
claiming an unvisited environment fails authenticated replay, not this local
domain check alone.

Actual farm output1..baseFood records positive quantities and exact fertility
spent; its successful input debit is Energy2. Output storage/funding/staff failure
consumes neither Energy nor fertility. Successful research records its actual
Energy2/Alloy1 inputs and1 point elsewhere; blocked/absent-target research records
no actual inputs or point. Due recipe attempts keep the ordinary checked
old_next_due+period once, including zero/environmentally blocked attempts.

ResourceFact keeps its u32 range because it also records actual zero outcomes,
failed paid0 supply and other cause-defined facts. This does not permit arbitrary
zero rows everywhere: the event's exact cause/roster and authenticated replay
decide their presence/value. It neither omits required Facts fields nor introduces
an optional receipt-omission feature. A list representing absent ResourceAmount
payments is[]; a zero payment entry is rejected even where a separate typed
SupplyAttempt.paid or ResourceFact legitimately records0.

## Exact ordinary Fleet dispatch

Freeze installed Fleet IDs at phase10 entry (max256), visit ascending once, then
Routes ascending. Travelling, absent order, not_before>T, target=null or target
current Docked World is NOT a departure attempt: emit0, change no credit/clock/
stock/location, retain undue supported self holds until publication. Same-World
standing action belongs its real combat/settlement/blockade phase. Dormant is no
implicit traversal filter. Newly admitted ordinary motion has not_before=T+1.
Only Docked due nonnull different-target order attempts actual departure.

For that attempt stop at the first failed guard, with no second explanation:

1. Actual typed owner/origin/target/full nonempty Hull definitions and exact
   objective/Request instance/order. Invalid installed shape/reference is an
   executor/decoder fault, not a blocked gameplay reason. Whole roster max16.
2. Return requires actual OwnedColony target; credited Dormant recovery requires
   Dormant→unowned target→Ark present→retained eligible target knowledge in that
   exact predicate order. Ordinary nonReturn launch requires actual owned local
   account unless this exact recovery. Then destination-purpose ownership precedes
   topology: Settle unowned, unbound Defend own, original Request-bound Defend
   requester-owned, Attack/Blockade foreign enemy. Preserve real owner Some even
   when Colony absent. Pure-scout Explore has no destination-owner requirement.
3. Compute current shortest public topology, no_route if absent; then current
   destination treaty/offensive/duty permission with the exact closed witness.
   Active Truce precedes missing active War; Transit is not offense. No longer
   graph path cures destination denial. Original old path is not guaranteed.
4. Attack/Blockade and travelling Defend require current supplied; Return/credited
   recovery/paid travel and ordinary nonoffensive Move/Explore/Settle keep their
   explicit exceptions. No departure Supply payment or deadline refresh.
5. Lowest actual paid HullJob targeting this Fleet blocks before live declaration
   structural lock. Either blocks even Return0/self funding. No invented winner,
   no private defender lock or fake standing order to exercise an unreachable case.
6. Return/recovery requires every Hull credit, lowest missing Hull ID first.
   Ordinary owned nonReturn pays base2+2 iff missing credit, Efficient Logistics
   ceil3/4 PER positive Hull (minimum1), checked Hull-ID sum. Actual physical Energy
   P<D gives physical_energy_unavailable; no held stock is labeled missing.
   Checked duration and T+duration arithmetic faults, never wraps or blocks falsely.
7. Shared resource availability D<=max(P-O-B,0) follows genuine physical funding;
   held-only failure uses solely dispatch_reservation_blocked/full witness.
8. Prepare all final source/cap/collision/event/byte/success operands before committing;
   these are complete unpublished faults, never gameplay capacity reasons.

Guards2–6 gameplay failure emits exactly fleet_dispatch_blocked with unchanged
complete order/location/Hulls/Supply/stocks and first finite reason; release own due
claim after complete attempt. No fresh intent_decided, CandidateAudit, paid motion,
AUTO ID, fake identical Change or ResourceFact. Guard7 has its sole separate carrier.
Silent classifier visits create no source/fact. Event-producing groups use actual
full pre-operation W/T/phase10, latest actually applied logical creator cursor,
actor-null original SnapshotReference (both digests full prefix), not H7 or future
Receipt. Replay reconstructs that unique Fleet-ID position under current ledger.

Success debits EnergyD once (paid ResourceFact only in motion, [] iffD0), grants
whole credits true on owned nonReturn or consumes false on Return/recovery, sets
Travelling original origin→FINAL target with checked T+wholePathDuration and actual
selected lane_path. All other order/binding/Supply/HP/repair/passenger counts persist.
First positive-passenger nonReturn Hull with voyage_origin null sets actual origin;
Return never creates origin, Some persists. Emit fleet_departed first, then full
containing-Fleet operation_changed iff ANY nested Hull credit/origin changes;
Change.resources[]/passenger_delta0. Location/order mutation appears in that full
Change but never duplicates payment. If no nested Hull changes, motion plus exact
source replay authenticates actual location/path transition with no fake Change.
No boarding/unload/workforce/contribution/fulfillment occurs at departure.

One paid final itinerary completes despite later graph/War/owner/technology edits;
no waypoint/account cursor or repeated payment exists. Future-order creator edits
cannot rewrite original departure history. Paid phase3 arrival emits fleet_arrived
THEN full Fleet operation_changed ALWAYS for its real location/order/binding result,
both sharing I3, resources[]/delta0. Unbound completion map and exact live bound
Defend exception are defined above and in scoring; genuine prior cleanup cannot
be resurrected. Actual endpoint deletion requires explicit traveller closure,
never a terminal-objective historical waiver for live paid location.

All nested classifier/guard/roster/path/source/hash/Hull/event/witness/cleanup work
shares remaining1–1024 visits. Public0/>1024 consumes0 invalid, internal0 yields.
No full State copy is one nominal visit. Fault/cancel/stale/premature finish retains
capacity owners and publishes no partial debit/credit/location/release; Ready0.

## Exact zero-freight transition

Retain route batch1..10/cadence10 and existing ownership, permission, reserve,
suspension and path guards. At an actual due route attempt in phase10:

1. Compute checked `new_next_due=old_next_due+cadence` once, before dispatch effects.
   Overflow faults the unpublished boundary, even if a factor would make cargo0.
   Do not set tick+cadence or perform catch-up. Undue routes do nothing.
2. After the clock, test old_suspended FIRST, then source participant, destination
   participant, path/route kind, public permission, then source reserve/funding.
   This is the coordinated forecast's literal order. An earlier failure stops
   later reads/holds; old_suspended=true stays true without any endpoint/modifier
   read. Unknown actor operands hold only the first visited unresolved predicate,
   without hidden-root repair; authority uses its actual operands.
   At the source-funding stage let P be actual local physical stock, R the
   retained route.source_reserve, N the known next Food/energy need for a needs
   resource (0 for a non-needs resource), and F=max(R,N). Read these exact operands
   from the authority state, or supported own actor accounts, after prior actual
   phase/ID mutations. Require P>F to reach restrictions; full nominal-batch
   funding is NOT required. If P<=R, stop with reason=no_supply, even if N>R.
   Otherwise P>R but P<=F stops with reason=own_needs_reserve: physical route-floor
   supply exists but the higher protected floor blocks it. These are the existing
   closed ObligationReason values, not NeedsReserved/DecisionReason substitutions.
   Either failure has eligible_batch=restricted_batch=actual_dispatched=0,
   blockers=[], modifiers=[], no capacity visit/debit/Shipment; the checked clock
   advances and old_suspended=false stays false. A later0 factor cannot replace
   this authentic earlier cause with reason=null. Unresolved N/funding in a private
   projection stays held and does not manufacture no_supply or eligible freight.
3. Set eligible_batch to the route's nominal positive batch after its eligibility
   guards. Apply the one eligible blockade reduction first: floor(eligible_batch/2),
   or unchanged without blockade. This is restricted_batch, not a shipment claim.
   Environmental factors operate on this nominal/restricted batch BEFORE the
   available-stock cap. Preserve the inherited dispatch equation below; scaling
   an already stock-capped quantity would silently change freight balance.
4. Apply every applicable active environmental factor to that current integer in
   increasing decoded event ID, preserving each literal numerator/denominator in
   Modifier order. Expired/unrelated events do not enter the list. No strongest-
   hazard V2 reduction, fraction-product shortcut or deduplicated equal-ratio rule.
5. Compute unchanged physical `Qp=min(modified_batch,P-F)`. If Qp=0,
   this is exclusively evaluated restriction-zero: reason=null, authentic complete
   modifiers/blockers, no Shipment-capacity visit/ID/debit. Release this due
   consumer's own cargo/floor claim after its complete attempt. Preserve suspension.
   Hold checks cannot legalize an earlier physical/floor/permission failure.
6. For Qp>0 apply the PRIVATE competing-hold guard BEFORE positive Shipment capacity.
   Let H be checked sum of all active cargo/payment quantities on this physical
   account, B their MAX linked source floor, S own consumer quantity, O=H-S.
   Require the retained invariant H+B<=P. Let G=max(current F,B) and
   `available=max(P-O-G,0)`, `Qa=min(Qp,available)`. This is an explicit NEW
   V3 competition equation; true physical P and numerical nominal/factors stay
   unchanged. Own cargo is not subtracted twice and own floor is not paid cost.
   Qa=0 has reason=resource_reserved and full DispatchReservation; no capacity visit,
   ID/debit. Smaller positive Qa carries that witness with reason=null. Equal Qa/Qp
   has reservation=null. Witness required=Qp, physical=P, floor=F, protected_floor=B,
   self_held=S/self_floor=max own floor; all actual other holders are retained in
   causal order. Blockade and route facts carry the same final Qa/witness.
7. For Qa>0 check actual Shipment capacity and remaining atomic operands. Failure
   releases own claim, debits/issues nothing and uses capacity_unavailable.
   Success withdraws exactly Qa once, issues the full genuine paid Shipment,
   consumes min(Qa,S) of self quantity in causal order and releases ALL remaining
   self entries/floor. Other entries survive. P'=P-Qa>=O+B guarantees remaining
   H'+B'<=P'. Original clocks advance once even on denial/zero, no retry.

Thus full4096 shipment capacity does not turn an otherwise valid0-result attempt
into a capacity failure: it needs no slot. A positive attempt still cannot bypass
that cap. Every due blocked/zero attempt advances the same checked clock once.
Cancel/end of the zero event does not reset route.next_due, resume an independently
suspended route, add catch-up shipments or reimburse anything. At event.end_tick,
phase2 removes the event before that boundary's phase10 dispatch.

Enumerate/record all applicable modifiers even if an earlier one produced0; this
preserves the exact causal list and counts each visit. A failing earlier route
guard does not fabricate that list. Source domains bound hazards128 and
stellar_events128 independently, so at most256 unique applicable event IDs need
inspection, before the existing receipt byte/event bounds. Copied equal IDs are
deduplicated by the existing authenticated source selection, not applied twice.

Zero dispatch creates no commitment fulfillment, demand decrement/removal,
defense-request resolution, aid/trade accumulator, consumer inventory/output,
arrival improvement or paid-delivery Anchor. Independently due request expiry or
ordinary local production still happens in its own phase. A valid negotiated
Commitment/Route can remain outstanding and wait without claiming a delivery;
the exact current consideration/lifecycle facts are retained. Future promised
cargo remains absent from scoring inventory. The positive public-request recovery
exception must prove a positive feasible funded matching dispatch; known0 freight
cannot claim recovery. Existing ordinary offer utility for its positive proposed
batch remains the declared formula, without a fabricated delivered benefit.

Accepted treaty capacity/Route ownership can persist through a temporary zero
event. For admission's transient first-batch hold, derive the known scheduled
first eligible attempt boundary d from the exact candidate/installed Route clock
and phase availability, not the acceptance tick or an assumed immediately due
batch. New Route next_due=checked(T+10); an adjusted Route retains old.next_due exactly; do not rewrite either to make suppression apply. A due attempt is
processed once at its first available phase10 boundary with next_due<=d; overdue
clock arithmetic remains old_due+cadence, not d+cadence. No guessed future resume,
permission, endpoint change or catch-up is an eligible scheduled dispatch.

Recheck the preceding eligibility/funding guards against the current exact ledger
and permitted supported inputs. For a supported eligible d, an environmental
source applies iff its target matches and start_tick<=d<end_tick after the known
phase1 interventions/phase2 expiry at d; warning_tick alone is not active freight
suppression. Its being active at acceptance is insufficient. A known cancellation
before dispatch excludes it; an unsupported future intervention is not predicted.
If d>=end_tick, the expired source contributes no factor, so a positive computed
next batch requires its ordinary exact positive transient cargo funding hold.
If d lies in the interval and all earlier guards pass, a computed0 requires no
cargo-stock hold/debit or Shipment token. This never waives a NEW Route's separate
Route capacity token, Agreement/Commitment capacity or other admission guards;
an existing-route adjustment uses its retained Route token. No positive feasible
recovery/paid delivery is inferred. Unknown first eligibility/time/permission or
required future source fact remains held under the coordinated forecast rules;
known0 is not a legalizer and cannot remove an earlier failed funding guard.

One shared private ledger survives the complete admission-to-dispatch handoff.
Positive Route cargo holds have linked F=max(reserve,supported next Need) floor;
other payment holds floor0. Distinct cargo sums H, floors MAX B, never summed paid
cost. Creation/replacement checks H_new+B_new<=P before committing; genuine other
accepted competition fails central stage9 target_reserved on affected World with
its earliest causal contributing holder actor, never stage8 physical stock=free.
Stage8 uses true physical P. Immediate payment converts its own provisional hold
once; paid job occupancy replaces its token, never doubles it. Structural declaration
Fleet locks independently forbid departure/split/merge until publication even
Return0; resource self entitlement cannot waive that guard.

A future undue next-batch hold (new RouteT+10) stays through this T dispatch and
releases at publication without persisted escrow or guaranteed future cargo.
A genuinely due earlier-guard failure/zero/capacity denial releases only its own
cargo AND floor after its completed attempt; MAX is recomputed incrementally.
Changed current F may reduce its actual attempt but never rewrites admitted floor
or steals another claim. Earlier blocked operations are not retried after release.
Actual Fleet Energy D physically funded by P must additionally satisfy
D<=max(P-O-B,0); a held-only failure emits ONLY dispatch_reservation_blocked,
full witness floor0/requiredD/availableP-O-B, no false MissingResource, fresh
Decision, fleet_departed or duplicate fleet_dispatch_blocked. D0 passes resources
but still obeys earlier structural lock. Fault/cancel/stale/premature finish retains
ledger graph for bounded cleanup and publishes no partial release/payment.

This calculation reserves at most the one exact next batch against today's
physical ledger, not forecast production or1200 ticks of inventory. The transient
hold ends at boundary publication and promises no future shipment slot/stock.
Every actual later attempt revalidates current stock/permissions/needs and only
then debits positive actual cargo. Existing in-flight manifests are untouched by
subsequent modifiers. First-batch source operands/reason use the same P/R/N/F
funding distinction; an earlier rejection keeps the exact existing central stage
reason contract, rather than replacing its DecisionReason with ObligationReason.

Creator CreateShipment cannot mint paid provenance. Require under this Proposed contract retained Shipment.units and CreatorOperation.create_shipment.units>0,
so a creator cannot insert an empty capacity-consuming shipment either. This is
one explicit Proposed family restriction, not a universal positive-u32 rule or
an invented creator batch10 limit. Existing greater positive creator quantities
obey their actual storage/reference/provenance constraints. Free creator shipment
or disposition override still earns no negotiated paid-delivery witness.

The new route_attempted/blockade explanatory facts consume their ordinary event
ordinals/EventIDs, and an accepted creator edit consumes its ordinary sequence.
Only zero dispatch's **Shipment** allocation/positive resource transfer is absent.
Receipt/event-cap failure remains an unpublished fault with preceding head intact;
it is not permission to drop a causal0 fact. Public identities, candidate terms,
action ordering and strict recovery priority do not change through this rule.

For example nominal batch10, available stock above reserve3 and factor1/2 gives
modified batch5 and actual transfer3. It must not give1 by first taking min(10,3)
and then halving. Source floors and protected-needs accounting remain actual;
only the modifier operand order is pinned, with no invented output or inventory.

## Arithmetic, funding and domain-check ownership

Each floor scale computes `checked_u64(quantity)*u64(numerator)/denominator`.
For u32 quantity/numerator the product fits u64, but still check it and its final
u32 conversion. Nonamplification bounds output by input. Do not multiply all
fractions together: aggregate products can overflow and flooring changes results.
Example batch5 with lower-ID3/5 then higher-ID2/3 gives3 then2; reverse IDs gives3
then1. The output depends on the specified ID order, not input array insertion.

Penalty1/2 uses its prescribed downward rounding. Efficient logistics uses exact
3/4 upward rounding, and clamps to at least1 only when its eligible base cost is
positive. A genuinely zero eligible base cost, such as prepaid return under its
exact existing rule, stays0 and has an empty payment list. Do not use a generic
freight floor for departure energy, clamp every quantity to1, or reprice a paid
leg after technology change. Compute local Energy requirement exactly from the
fixed per-hull rule/credits and aggregate in the order that rule prescribes;
these domain predicates do not choose a new per-fleet versus per-hull discount.

DeclarationHold resources are funding availability holds, not expenditure: War
admission pays0 regardless of whether its required potential departure list is
positive. Later movement revalidates/pays once. A matching released=true record
uses identical resources; no refund is inferred. Private defender holds/stock
are not copied into the declarer record or scored capture.

| Decoder stage | Exact ownership/error choice |
| --- | --- |
|0–5 structural | Existing raw-cap/UTF-8/syntax/header/key/schema precedence. Scalar wrong type/width is invalid_range; unknown fields/tags retain their closed errors. These proposed cross-field quantity/ratio predicates are not an earlier stage5 fixed-mechanics test. |
|6 canonical bytes | Compare unchanged exact spelling/order before semantic checks. Whitespace plus width-valid bad fraction/quantity selects noncanonical_bytes first. Submitted event2/4 is canonical when written in declared order; it is not normalized to1/2. |
|7 local semantic | After installed-identity checks, validate the table's denominator/ratio and quantity/list/cause-shape predicates and exact finite Catalog literals. Use invalid_range at the offending numerator, denominator, quantity/list or fixed-profile path; local declared-cap violations use record_capacity. Valid denominator is prerequisite to the cross-field numerator<=denominator predicate: denominator0 points to denominator, not an invented divide-by-zero code. Order remaining violations by existing schema-ordinal/numeric-index path rule. |
|8–9 linkage/integrity | Existing exact supplied-Catalog, typed/historical reference, graph/view/high-water and framed-integrity checks. Do not treat a genuine historical paid route reference as required current live route after an authorized removal. No new self-hash/certificate is added. |
|10 replay/evidence | Authenticate zero cause, evaluated operands, source fraction copies, paid source withdrawal, exact computed payment/hold lists and matching release. A domain-valid false output/payment/modifier returns replay_mismatch; forged authenticated witness retains forged_evidence at its declared check. Preserve the reviewed integrated issuance/replay encounter order and closed vocabulary. |

For example a canonical fixed Catalog ion_storm2/4 passes the general ratio
predicate but fails exact mechanics at stage7 invalid_range; its whitespace
variant fails stage6 first. Event0/7 passes this Proposed creator domain; denominator0 and numerator8/7 fail semantic stage7. ResourceAmount0
inside a listed payment fails stage7 even if physical stock is0. Quantity3 instead
of authentic computed4 passes positivity but fails authenticated derivation at
stage10. This is semantic decoder stage7, distinct from central attempt stage7
capacity and from the runtime's separate Arithmetic fault diagnostic.

Genesis/replayed State references, exact Catalog linkage and provenance remain
mandatory. Domain-valid0/1 does not bypass the optional minimal initial profile
or admit arbitrary stellar events/RecipeFacts/manifests at genesis. Creator previews
recompute exact evaluated facts, review event effects/cascades and validate the
projected queue/head before sequence allocation. Changing ratio/zero mode is an
attributed creator revision, not new paid delivery or autonomous flare provenance.
Missing/wrong references, unsupported initial families and false source snapshots
do not become valid through a positive quantity or mathematically equal fraction.
Every head-dependent mutation retains generation CAS and exact capture freshness.

## Catalog schema and definition registry

Catalog fields in order: `kind, format_version, rules_version, pack_id,
pack_version, star_archetypes, world_archetypes, resources, industry_definitions,
hull_definitions, hazard_definitions, policy_definitions, technology_definitions`.
There is no self-hash field. All catalogs are exact immutable documents; pack
version is positive u32, and every collection sorts by id (bonuses by intent).
Envelope types are kind:String, format_version:u32, rules_version:u32,
pack_id:Slug, pack_version:u32 (1..=u32::MAX).

### Finite Catalog mechanics and metadata (Proposed)

Rules_version3 uses exactly the snapshot's definition-ID/resource/role/effect/prerequisite/bonus/numeric roster. Valid Name metadata may vary only at existing `.name` fields in star_archetypes, world_archetypes, resources, industry_definitions, hull_definitions, hazard_definitions, policy_definitions and technology_definitions; pack_id and positive u32 pack_version may also vary. All other interpreted fields remain exact snapshot mechanics. Missing/extra definitions or any changed role, effect, prerequisite, policy bonus, storage or numerical value reject this profile; a different balance needs a separately reviewed rules version. Name absence where the declared record lacks a name is not a new optional metadata field. Metadata changes create a distinct exact PACK digest and must be linked by every consumer/root/identity frame; there is no Catalog self-integrity field. Historical leaf positives are not retroactively admitted full packs.

The frozen catalog records apply with these required overrides:

```text
TechnologyDefinition { id:Slug, name:Name, prerequisites:Sorted<Slug>,
                       point_cost:u32, effect:TechnologyEffect }
TechnologyEffect = farm_base_output { units:u32 }
                 | carrying_capacity_penalty_scale { numerator:u32, denominator:u32 }
                 | departure_energy_scale { numerator:u32, denominator:u32 }
IndustryEffect adds habitation { base_capacity:u32 }
                   research { period_ticks:u32, inputs:Sorted<ResourceAmount>,
                              points:u32 }
IndustryDefinition appends workforce_units:u32, stress_per_operation:u32
HullDefinition appends role:HullRole
HullRole = ark | escort | scout
IntentKind adds build_farm | build_habitation | build_research_lab |
               select_research | request_supply | negotiate_access |
               negotiate_supply | negotiate_defense | blockade | adaptation
```

Initial industry records preserve the existing costs/periods/recipe/HP/damage
values visible in `assets/living/core-pack-v2.json`, frozen in the snapshot below;
V3 slot cost remains1 and per-colony maxima remain their frozen values, not an
implicit expansion from6 to8. New farm/habitation/lab values come from society
§Added catalog definitions. Their proposed slot_cost is1 and max_per_colony8;
these fill previously undeclared catalog fields and require written review. All enabled active industrial kinds require one
worker; habitation/defense require zero. Extractor/foundry stress values are1/2,
others zero. Hull roles follow explicit definitions, never name guessing.
Initial technologies are exactly agronomy100, resilient_habitation150 requiring
agronomy, efficient_logistics100; effects exactly society §Research. The V3 required
override carrying_capacity_penalty_scale has numerator1 then denominator2 and
applies only to the integer capacity penalty D in ecology §World environment and
conservation Formula A: Cbase-floor(D/2). It never changes effective habitability
or Farm Food; base habitability still caps capacity. This renamed V3 tag requires
fresh Catalog bytes/digest derivation; the frozen V2 snapshot below is unchanged. Only these
reviewed effect kinds are allowed, costs/periods positive, graphs acyclic and
within64 nodes. A differently balanced catalog needs its own reviewed rules
version, not an arbitrary pack modifying fixed rules under rules_version3.

Industrialist is retained as a catalog policy; its V3 research preference uses
Neutral order because no distinct society order is specified. This explicit
compatibility proposal requires review; it does not change the stated Trader,
Guardian, Neutral or Expansionist preferences.

## Closed collection, record, key and Sorted-leaf declarations

CollectionKind is the closed unit enum whose strings and normative comparison
order are the table's first column in row order. Record is a tagged enum with
exactly one variant per row: `{type:COLLECTION_STRING,value:DECLARED_TYPE}`.
TypedKey is a tagged enum with exactly one variant per row:
`{type:COLLECTION_STRING, ...KEY_FIELDS_IN_LISTED_ORDER}`; `{}` in the table means
no fields after type, not a value object. Variant tags are literal lowercase
strings below, not singularization or implementation type names. Every Change
and comparison difference requires matching collection, Record and TypedKey
variants, and Record's key must equal TypedKey. Scalar before/after cannot be
null; collection creation/removal uses null as already specified.

| CollectionKind / Record / TypedKey tag | Record.value type | TypedKey fields after type |
| --- | --- | --- |
| tick | Tick | `{}` |
| branch_sequence | u64 | `{}` |
| next_sequence | u64 | `{}` |
| systems | System | `{id:ID}` |
| stars | Star | `{id:ID}` |
| worlds | World | `{id:ID}` |
| lanes | Lane | `{id:ID}` |
| civilizations | Civilization | `{id:ID}` |
| deposits | Deposit | `{id:ID}` |
| colonies | Colony | `{world:ID}` |
| facilities | Facility | `{id:ID}` |
| construction_jobs | ConstructionJob | `{id:ID}` |
| hull_jobs | HullJob | `{id:ID}` |
| fleets | Fleet | `{id:ID}` |
| routes | FreightRoute | `{id:ID}` |
| shipments | Shipment | `{id:ID}` |
| relations | Relation | `{from:ID,to:ID}` |
| agreements | Agreement | `{participant_a:ID,participant_b:ID,kind:AgreementKind}` |
| wars | War | `{participant_a:ID,participant_b:ID}` |
| observations | Observation | `{civilization:ID,world:ID}` |
| hazards | Hazard | `{id:ID}` |
| settlement_claims | SettlementClaim | `{world:ID,civilization:ID}` |
| occupations | Occupation | `{world:ID}` |
| offers | Offer | `{id:ID}` |
| demand_requests | DemandRequest | `{actor:ID,world:ID,resource:Resource}` |
| commitments | Commitment | `{id:ID}` |
| defense_requests | DefenseRequest | `{id:ID}` |
| shared_intelligence | SharedIntelligence | `{recipient:ID,originating_actor:ID,world:ID}` |
| stellar_events | StellarEvent | `{id:ID}` |
| counters | Counters | `{}` |

Aliases are exact: System/Star/World/Lane/Civilization/Deposit/Colony/Facility/
ConstructionJob/HullJob/Fleet/FreightRoute/Shipment/Relation/Agreement/War/
Observation/Hazard/SettlementClaim/Occupation/Counters denote their frozen
Living-prefixed V3 record with the mandatory overrides; AgreementKind and Resource
denote their frozen enum plus Food override. Other types use their new declarations.
Counters.value is `{}` and cannot differ under rules3. Scalar differences include
tick/branch_sequence/next_sequence separately in table order, even though two
compared states must have the same tick. Command preview lists scalar changes
only if the operation changes that value; accepted allocator consumption is
transport/revision metadata, not an implicit creator resource consequence.

Hull IDs exist only inside Fleet.hulls. There is no top-level hulls CollectionKind,
Record or TypedKey variant. A hull creation/edit/removal, passenger change or split/
merge emits full affected Fleet before/after snapshots under the fleet key;
comparison likewise reports Fleet snapshots and UI may read-only expand their
nested hull arrays. A hull target ID resolves uniquely through the fleet's sorted
hulls; duplicate hull IDs across fleets reject. This avoids two canonical deltas
encoding the same nested mutation. Temporary job-reserved target hull/fleet IDs
are validated through their separate reservations, never a fake top-level hull row.

Every Sorted type has these exact leaf keys. Composite keys compare tuple fields
in the listed order using scalar byte/numeric order; optional values null first.
Unit enums sort by their literal UTF-8 tag, except CollectionKind which uses the
explicit table order. Lists called Vec retain order and never acquire a sort key.

| Sorted leaf | Key |
| --- | --- |
| ID,Slug | decoded bytes / UTF-8 bytes |
| ResourceAmount | resource slug |
| PolicyBonus | intent tag |
| StarArchetype,WorldArchetype,ResourceDefinition,IndustryDefinition,HullDefinition,HazardDefinition,PolicyDefinition,TechnologyDefinition | id slug |
| BlockedReason,DemographicCause,ObligationReason | (tag,canonical payload bytes) |
| TechnologyProgress | technology |
| WarObjective | actor |
| ObservedFacility,ObservedHull | definition |
| KnownLaneHazard,KnownEnvironmentEvent | id |
| KnownWorldGeometry | id |
| HazardScopeObservation,StellarScopeObservation | lane / system |
| ObservedThreat | (enemy,target_world,observed_tick,originating_observer) |
| ResourceFact | (role tag,resource tag) |
| Damage | entity |
| GenesisAllocation | (kind numeric,ordinal numeric) |
| Revision,Branch | full revision bytes / branch bytes |
| Bookmark,Experiment,Evidence | ordinal numeric |
| Anchor | (branch,revision,tick,receipt_digest,event.phase,event.ordinal,event.id), null event first |
| EventReference | (phase numeric,ordinal numeric,id bytes) |
| Remaining top-level state record types | exact TypedKey tuple in the collection table |

An Anchor's event kind is validated against its authenticated receipt, not part
of its unique key. Modifier is an ordered Vec by event ID. MetricObservation is
an ordered Vec in the selected metric/reference order declared below. Snapshot
Sorted arrays use their underlying state record's exact key. No generic hidden
sorting fallback is allowed.

## Complete command additions, overrides and removal contract

Inherited CreatorOperation variants above encode snake_case tags with their
listed field order. New operations:

```text
SetPopulation { world:Target, population:u32 }
SetSpecialization { world:Target, specialization:Specialization }
SetResearchTarget { civilization:Target, target:Option<Slug> }
GrantTechnology { civilization:Target, technology:Slug }
RevokeTechnology { civilization:Target, technology:Slug, dependents:Sorted<Slug> }
SetEnvironment { world:Target, environment:Environment }
SetActivityClock { star:Target, next_activity_tick:Tick }
BoardPassengers { hull:Target, origin_world:Target, count:u32 }
SetPassengers { hull:Target, count:u32 }
UnloadPassengers { hull:Target, destination_world:Target, count:u32 }
CreateOffer { local:u16, proposer:Target, recipient:Target, kind:TreatyKind,
              terms:Terms, created_tick:Tick, expires_tick:Tick,
              demand:Option<DemandKey>, renewal_of:Option<ID>, last_consideration:Option<OfferConsideration> }
OfferConsideration { tick:Tick, disposition:OfferDisposition,
                     reason:Option<ObligationReason> }
OfferDisposition = waiting | accepted | refused
CancelOffer { target:Target }
CreateCommitment { local:u16, parties:Pair, kind:TreatyKind, terms:Terms,
                   created_tick:Tick, expires_tick:Tick }
EditCommitment { target:Target, terms:Terms, expires_tick:Tick }
RemoveCommitment { target:Target }
SetWarObjective { war_a:Target, war_b:Target, actor:Target,
                  kind:ObjectiveKind, target:Target,
                  secure_endpoints:Option<SecureEndpoints> }
SetFleetSupply { fleet:Target, supply:Supply }
SetBlockade { fleet:Target, blockade:Option<Blockade> }
CreateStellarEvent { local:u16, star:Target, warning_tick:Tick, start_tick:Tick,
                    end_tick:Tick, habitability_impact:u32, freight_factor:Fraction }
EditStellarEvent { target:Target, star:Target, warning_tick:Tick, start_tick:Tick,
                  end_tick:Tick, habitability_impact:u32, freight_factor:Fraction }
CancelStellarEvent { target:Target }
```

`Target` is the frozen LivingCommandTargetV3: existing typed entity or earlier
batch-local identifier. Local keys are unique u16 across the batch; forward,
self and wrong-kind references reject. New Terms/Pair/DemandKey references in a
command are existing typed IDs only, so complex creation uses an earlier batch
followed by a fresh exact preview. No untyped reference resolution is allowed.

CreateStar appends next_activity_tick; CreateWorld appends environment;
CreateCivilization appends research; CreateColony appends society;
CreateHull submits its declared count-only PassengerCountInput; creator creation
initializes voyage_origin=null, never submits paid origin. Non-Ark count must0; CreateHazard/EditHazard insert warning_tick before
start_tick. ForceWar appends `objectives:Sorted<WarObjective>`; the two objectives
start pending/count0/witness null and its end_tick obeys proposed diplomacy1200.
CreateFacility appends `disabled:bool, next_repair:Option<Tick>`; derived staffing
and RecipeFacts are recomputed rather than submitted. CreateFleet initializes
never_supplied and no blockade, explicitly and with a Creator intervention event.
CreateShipment's declaration above includes explicit original_travel_ticks,
dispatch_kind and returning_to_source. Its provenance is Creator with the actual
computed CreatorCapture and override=null; null override never makes authored
cargo paid. Require positive units/duration, arrival>departure and checked
arrival−departure=original_travel_ticks at this creation. Exact initial profiles
are Outbound/false, Returning/true and WaitingForStorage with either submitted leg
flag. Authored Returning/true and returned Waiting are legitimate Creator profiles,
not proof of a preceding paid outbound return. No codec supplies flags or clocks.

SetShipmentDisposition preserves every current clock, immutable duration, endpoint,
unit, party, kind and original dispatch-provenance byte. Let O=(Outbound,false),
R=(Returning,true), W_O=(WaitingForStorage,false), W_R=(WaitingForStorage,true).
These are the only valid prior combinations; malformed Outbound/true or
Returning/false rejects under local semantics and cannot be repaired by this tool.

| Actual prior | Request Outbound | Request Returning | Request WaitingForStorage |
| --- | --- | --- | --- |
| O | O + current capture | R + current capture | W_O + current capture |
| W_O | O + current capture | R + current capture | W_O + current capture |
| R | invalid_range return reset | R + current capture | W_R + current capture |
| W_R | invalid_range return reset | R + current capture | W_R + current capture |

For each of the ten successful cells set only disposition, returning_to_source
and creator_disposition_override to the genuine current pre-operation capture.
The two reset failures name disposition at the actual operation index and reject
the whole candidate. Repeated same-tag setters are real capture-stamping effects,
not value-only no-ops: accepted sequence/index/source distinguish each intervention.
Replay verifies the original accepted effect and never creates a new stamp.
One full direct Shipment Change records original/result with resources[] and
passenger_delta0; only aggregate creator events report it, with no autonomous
shipment_returned/waiting, inventory, Relation, C-attempt or witness companion.

O/W_O→R selects original source at the EXISTING active clock, never T+duration.
Future arrival waits to that same clock; due/overdue resolves on the next ordinary
phase3 visit. Waiting selects a storage retry at each advancing phase3 even before
its unchanged future clock, as an explicitly NEW creator intervention consequence.
Selecting O/R from Waiting reinstates the old traveling due predicate, no restart.
No cargo moves during paused application/save/fork; the next ordinary boundary
uses the recorded result. Running phase1 precedes the phase3 I3 capture and visits
each Shipment at most once. Missing endpoints require explicit reviewed traveller
resolution; the setter cannot relocate, repair references or reset one return.

At phase3 O/W_O chooses current destination ownership BEFORE War/storage: changed
owner including null resolves capture/unowned delivery; unchanged intended party
at War initiates its one genuine automatic return with current departure=T and
arrival=checked(T+original_travel_ticks), retaining the override. R/W_R chooses
original source and never bounces. Full endpoint storage produces Waiting with
retained leg flag and unchanged clocks; enough room transfers all units once and
removes the manifest with its exact normalized O/R outcome. No second same-phase
visit occurs after return initiation. Undue O/R emits nothing; Waiting always retries.

Paid relationship/Commitment credit requires genuine original paid provenance AND
override=null plus all original-party/outbound/binding guards. Every successful
setter permanently disqualifies this parcel from that credit, including O→O.
Actual transfer remains real; no fake zero fulfillment/attempt or prior Anchor
rewrite follows from nonqualification. Earlier genuinely sealed fulfillment remains
historical truth. Stage7 validates combinations/clocks, stage8 live endpoints and
original historical manifest linkage, stage9 integrity, stage10 the actual creator
matrix/capture/full effects plus original paid producer. Unaccepted failures consume
no published marks; accepted execution faults preserve queue/marks diagnostically
and abort the unpublished boundary. All replay/copy/hash/effects/cleanup share the
remaining1–1024 visits; no capacity owner drops or partial stamp publishes.
CreateRoute/EditRoute append commitment. SetFleetOrder declares
`objective:Option<ObjectiveKey>, defense_request:Option<DefenseActionBinding>`
after lane_path. Both options are mandatory and validated against their original
instances; creator association grants no payment, contribution or fulfillment. All other inherited fields remain
required. Hub clocks from CreateColony are specified by the accepted application
tick plus the fixed10/100/100 periods; initial last_needs null requires a future
needs boundary and zero assignments until immediate assignment recomputation.

### Creator Fleet construction context

In declaration-order execution of one CreatorBatch, derive a private set E of
Fleet IDs genuinely born empty by CreateFleet and not yet populated or removed.
An existing Fleet never enters E. The real creation inserts exactly
`{id,owner,location:Docked{submitted live world},order:null,hulls:[],
supply:{last_success:null,last_attempt:null,readiness:never_supplied},blockade:null}`.
It consumes the ordinary Fleet slot and genuine creator identity immediately;
there is no implicit Hull, credit, payment, passenger, objective or Request.

An E row must remain byte-identical to this initialized row, with live typed
owner/World, no paid job/declaration reservation or other live authored reference.
Only its first CreateHull or explicit Fleet removal may target it. Unrelated
operations may intervene only while every E predicate remains valid. SetFleetOrder,
EditFleet, SetFleetSupply, SetBlockade, target HullJob, split/merge, passenger tools
or any other new reference to E reject at that operation with invalid_range at
its operand/record path. Earlier Local resolution, unknown/forward/self/wrong-kind
checks, ordinary cap and issuer/collision checks remain unchanged. A same-batch
resolved ID has no greater permission than its Local spelling.

First CreateHull checks all submitted/catalog/HP/credit/passenger/cap predicates,
inserts the genuine sorted Hull into the complete containing Fleet, then removes
that Fleet from E. Its full Change is empty-before to singleton-after, never a
new top-level Hull Record. Removal has complete empty before/null after,
resources[]/passenger_delta0, deletes E membership and preserves issued identity
in actual history. Endpoint/owner removal must include this exact Fleet in its
normal direct/indirect closure. Later reference to the removed local rejects;
creation identity/local cannot be reused. Once populated a Fleet never reenters E:
subsequent last-Hull deletion uses containing-Fleet removal.

After the last operation require E empty and ordinary full State validity.
Surviving empty Fleet fails invalid_range at the lowest schema-array-index
Fleet.hulls path, with originating CreateFleet operation index in diagnostics
where available. An earlier actual operation/preflight failure keeps its existing
precedence; do not prescan planned later population to displace it. Every running
revision closes independently before the next accepted envelope, so separate
CreateFleet and CreateHull revisions cannot cooperate. Paused save/fork likewise
never persists a shell. Genuine create-plus-remove is real accepted history,
not a no-op, refunded allocator mark or recycled identity.

Direct effects and captures retain null→empty→singleton/removal chronological
full rows with their true operation_index and actual passenger/resource deltas.
Only aggregate creator_intervention and actual indirect creator_cascade report
these effects; no fleet_created or extra operation_changed event is added.
CreatorCapture hashes the exact original pre-operation prefix, including E where
present, retaining genuine sequence/index/tick/mode/phase1 source and parent.
Batch-entry capture has E empty. No Actor32/phase7 source claims this context.

Stage5 checks full field shape/scalars even for empty embedded Records. Stage7
checks nonempty in every completed State and ordinary Fleet Record, narrowly
deferring only cardinality for exact initialized creator Change endpoints or
registered creator prefix sources pending stage10 reconstruction. Stage8 retains
typed links/high-water and original enclosing command/capture/index linkage.
Stage9 integrity is unchanged. Stage10 starts from the valid original pre-batch
State, replays actual accepted operations and allocations through the named index,
derives E, compares exact prefix/capture/full effects, and proves final E empty.
Forged source/context or autonomous empty Change fails its owning invalid_range,
missing_reference or replay_mismatch stage; no context skips historical transient
identity scanning. The exception cannot be proved from an incomplete scan.

All context/source/copy/hash/effect/cap/final-validation/cleanup work shares the
remaining1–1024 visits. Failed/unaccepted preview publishes no State, Revision,
mark, event or partial identity. Accepted running execution faults preserve its
accepted queue/marks for diagnosis and discard the complete unpublished boundary,
without rewinding durable acceptance. Retain candidate/context/effect owners for
bounded retirement on cancellation, failure, stale publication or premature finish;
Ready exposes only complete valid output and repeated Ready consumes0.

CreatorBatch replaces its payload with `operations:Vec<CreatorOperation>,
reviewed_effects:Vec<Change>`. Each mutation's cascade, refund, discarded resource,
passenger loss, research revocation, ownership/route/request/intelligence/war
invalidation and clock/assignment recomputation appears in this exact change
list, in operation order then collection order then stable key. `Change` is:
`{operation_index:Option<u32>, collection:CollectionKind, key:TypedKey,
before:Option<Record>, after:Option<Record>, resources:Vec<ResourceFact>,
passenger_delta:Score}`. `Record` is a tagged union with one variant for every
state collection record in the frozen schema; CollectionKind uses those exact
state field names; TypedKey uses that collection's exact key schema. Before/after
records are complete snapshots. operation_index is null for autonomous facts and exact0-based for creator effects.
Null before means create, null after remove;
both null reject. All implicit consequences are listed once. The candidate is
computed without mutation, and byte-identical recomputation must match the
submitted preview. Missing/extra/changed consequences reject atomically.

The inherited cascade enum remains an explicit assertion of which dependencies
are to be removed. RemoveListed may be used only when no derived-key record is
involved; RemoveReviewed requires all derived records. DerivedRemoval adds
`commitment { id:ID }`, `offer { id:ID }`, `demand_request { key:DemandKey }`,
`defense_request { id:ID }`, `shared_intelligence { recipient:ID,
originating_actor:ID, world:ID }`, `stellar_event { id:ID }`,
`technology { civilization:ID, technology:Slug }`. SetWorldOwner/RemoveColony and
ForcePeace also require the CreatorBatch reviewed_effects even though their
inherited variant does not carry a second cascade list. No removal silently
teleports population, grants refunds, strands authoritative references or
fabricates prior payments. Historical Anchors retain original references.

CommandEnvelope fields: `domain:u32, catalog_hash:Digest, branch:BranchID,
expected_committed_revision:Option<RevisionID>, expected_tick:Tick,
expected_state_digest:Digest, expected_pending_sequence:u64, mode:CommandMode,
command:Command`. Command has only creator_batch. AcceptedCommand fields:
`accepted_sequence:u64, application_tick:Tick, envelope:CommandEnvelope`.
Revision fields: `id:RevisionID, branch:BranchID, parent:Option<RevisionID>,
tick:Tick, ordinal:u64, mode:CommandMode, capture:SnapshotReference,
command:Command`. Running applies before lifecycle at the next boundary;
paused applies after the selected completed tick and never advances clocks.

## Derived future Truce capacity and mutation contract (Proposed)

Retain both published caps. At successful War admission, reserve the future Truce
key for that canonical pair in the same256-account used by actual treaties. Derive
the reservation from retained War rows; store no extra State/War field, issued ID,
completion bit or claimed quantity. This reduces available ordinary treaty space
while a War is outstanding. It is an explicit Proposed admission change requiring
approval, even though it adds no reservation bytes.

Canonicalize a valid pair as `p=(a,b)` with distinct issued civilization IDs and
`a<b` by decoded bytes. Reject noncanonical/duplicate keys rather than normalize
an imported document. Define disjoint tagged treaty keys:

```text
AgreementKey = agreement { a:ID, b:ID, kind:AgreementKind }
CommitmentKey = commitment { id:ID }
FutureTruceKey(p) = AgreementKey { a:p.a, b:p.b, kind:truce }
A = set of keys of retained actual Agreement rows
C = set of keys of retained actual Commitment rows
R = { FutureTruceKey(pair(w)) : w in retained actual War rows }
N_state = |A union R| + |C| <= 256
N_wars = number of retained actual War rows <= 120
```

`agreement` and `commitment` remain separate token families even when ID bytes
coincide. An Agreement's pair/kind is not a fabricated entity ID. Every retained
War row reserves immediately, including a creator-supplied future start and a row
due for removal in the private next-boundary candidate. Never infer early release
from `tick>=end_tick` before its actual atomic conversion/removal. No unrelated
collection uses this account or acquires projected rows through it.

This is exact union accounting, not `agreements.len+commitments.len+wars.len`
without key checks. A reserved key and its actual same-key Truce occupy one unit.
Removing such an actual Truce alone does not release the War's reservation.
Replacing the War at the same pair does not allocate a second reservation.
Changing pair is removal plus independent admission, never identity reuse across
pair keys. Opposite-party declarations address the same canonical pair once.

The union formula also handles private conversion/override intermediate owners;
it does not make an otherwise incompatible simultaneous War/Truce legal. Normal
declaration still fails treaty/War compatibility before capacity. Canonical compatibility is no retained War plus retained same-pair Truce after a
completed mutation: creator ForceWar removes that incompatible Truce explicitly.
Thus existing Truce can transfer its unit to the future reservation in a reviewed
override without consuming a257th unit. This compatibility clarification must be
approved with the capacity rule; it is not claimed as already explicit for every
future-start creator interval in the frozen package.

## Ledger and accepted creator queue

Before central publication, add successful provisional War pairs and exact treaty
allocation holds to the same typed union. A second owner of the same key does not
add another unit; incompatible ownership still rejects through the existing
guard/order rules. A failed attempt removes only its own provisional ownership,
never another candidate's hold or the retained War's reservation.

```text
R_live = R union successful_unpublished_War_keys
N_live = |A union R_live union successful_Agreement_allocation_keys|
       + |C union successful_Commitment_allocation_keys| <= 256
```

Set union is within its tagged family; Commitment holds bind exact identities or
the existing authenticated issuance reservation, not a fabricated zero ID. The
resident queue already contains authenticated CreatorBatch operations and exact
reviewed effects. Reconstruct its projected states in accepted order, deriveR from
the War rows of each such state, and validate each operation/batch prefix before
accepting the next command. A final valid tail cannot excuse an invalid earlier
War admission. Conversely, do not sum mutually exclusive current and projected
state layouts: an explicitly removed Agreement can transfer its unit to a new
War in the same reviewed prefix. A War created and explicitly cancelled inside
one atomic batch passes its operation admission checks but leaves no surviving
reservation in that completed projected state.

The existing timing guarantee matters: running accepted commands apply at the
next boundary's phase1, in accepted order, before expiry and before ANY autonomous
material admission. Phase7 captures and the phase8/9 ledger therefore see the
actual result of those queued mutations, including its derivedR. No intervening
autonomous treaty can spend a promised projected slot while leaving that queue
unapplied. Paused edits/queue changes must reconcile the exact pending parent/
sequence chain, not mutate underneath an accepted preview. Rebuild projections
on reopen/replay/head validation. Existing64-batch/1024-operation/16MiB queue
bounds remain. This contract introduces no union lease across mutually
exclusive future heads and no extra capacity restriction for a reviewed swap.
If a later design permits accepted commands to skip phase1 or apply after material
admissions, explicit time-indexed queue leases become a separate prerequisite.

Compare the exact actual or projected union before and after each mutation. Only
removals already performed in that private operation/prefix offset a reservation;
do not borrow an unrelated later expiry or a later unapplied queued deletion.
Normal negotiation, creator insertion, renewal, cancellation and final validation
use the same account. Failed supply acceptance retains its pending Offer and exact
ID with `capacity_unavailable`; it cannot install half a Commitment/Route or debit
stock. Same-terms renewal occupies its retained Commitment ID once. A new supply
Commitment separately needs its Route account and actual dispatch funding; treaty
reservation grants neither.

## Termination, same-key behavior and creator disposition

For accepted peace or expiry at actual ending tick `E`, prepare an atomic transition
to remove the War, emit its end fact, cancel the pair-bound future orders/Blockades
and install `Truce(p,start_tick=E,end_tick=checked(E+300))`. This converts a leased
key to an actual key, so `N_after=N_before` unless other independently reviewed
rows are removed. Remove the War before the Truce in authenticated fact order, but
publish neither intermediate state. Release the War120 token exactly once. For creator ForcePeace/override/cascade,
operation mutations still remove the War before installing any Truce, but evidence
is exactly the creator_intervention/optional creator_cascade Change partition.
Do not add an autonomous war_ended/peace/refund/fulfillment event for that same
creator effect or replay aggregate events as mutations. Natural expiry/negotiated
peace retain their actual autonomous lifecycle facts.

At War admission require checked `start+1200=end` under its existing clock rules
and checked `end+300`. Every newly installed/replaced ForceWar also requires
`end_tick > application_tick`; a prior start or future start is permitted only
while this future-end predicate and the other exact guards hold. Completed root/
selected imported State retaining a War requires `end_tick > State.tick`. Reject
an overdue imported War without catch-up or time repair. These are new Proposed
creator/root temporal predicates, not inferred existing decoder behavior.

An inherited War whose deadline is running boundary E may remain in the private
phase1 working candidate copied from completed E-1. It reserves its future key
until phase2E converts once at exactly E, or ForcePeace at phase1E converts once
and phase2 sees no War. The completed-root inequality is not applied prematurely
to this inherited phase1 due row. A new/replaced ForceWar at E must have end>E,
and a paused publication at completed E leaves no overdue War. No backdated
conversion, invented past fact or extra300 interval is permitted. A retained War whose mandatory expiry cannot represent its
Truce deadline is invalid at admission/root/import validation. Early peace at
`E<=end` also checks `E+300`; do not saturate clocks. A generic arithmetic/structural
fault remains distinct from an ordinary central capacity rejection. No partial
end fact, missing Truce, permission change or receipt publishes on failure.

| Operation | Exact Proposed disposition |
| --- | --- |
| Autonomous declaration | New pair reserves one future Truce unit atomically with both objectives. Existing compatible treaty capacity must be available after existing guards; no unilateral removal of obligations. |
| Second concurrent declaration for installed pair | Reject actual permission compatibility at central stage3 with `invalid_target {target:peer,rule:permission_changed}`; preserve scored candidate. No second War/end clock/reservation/defender selection. A pair already incompatible in the capture never formed a candidate. |
| Expiry or accepted Peace | Convert at actualE as above; natural expiry does not wait for a100-tick negotiation boundary. Objective success/proposal/refusal alone does not remove the War or release its slot. |
| Creator ForcePeace with a War | Convert at its application tick, including reviewed order/Blockade cancellation and existing occupation reset semantics. Creator attribution, no negotiated success or objective credit. |
| Creator ForcePeace without a War | No nonexistent reservation is claimed. Existing same-key retained Truce follows the three interval cases below and upserts once; absent Truce needs a real free treaty unit. Full account without that key rejects the preview/batch. |
| Existing same-key Truce on a ForcePeace upsert | If intervals overlap or touch, coalesce to `start=min(old.start,E)`, `end=max(old.end,checked(E+300))`. Never shorten a longer existing NoOffense interval. If `old.end<E`, replace the ended interval with exactly `[E,checked(E+300))` at the SAME typed key; reviewed before/after terms record that replacement and no continuous permission across the past gap. `old.end==E` is touching and coalesces. If `old.start>checked(E+300)`, reject the disjoint future interval rather than fill its gap or discard that separately authored interval; equality is touching. This three-case rule covers a backdated SetAgreement created earlier in phase1 without inventing lifecycle work or another treaty unit. Explicit reviewed removal/replacement can be a different creator operation. |
| Creator ForceWar at existing same pair | Explicit override replaces the full War/objectives/clock/peace state; holds the same future key once. Old objective-bound orders/Blockades that become invalid are reviewed/cancelled. Require1200 duration and representable eventual300 Truce. No declaration energy debit. |
| Creator ForceWar replacing an active Truce | Explicitly remove the incompatible Truce in reviewed effects and install War+derived reservation atomically; existing key transfers its one unit. It is an attributed creator override of NoOffense, never ordinary permission. |
| Creator SetAgreement/CreateCommitment/EditCommitment affecting War compatibility | Preserve exact eligible/override semantics, but any War cancellation must be an explicit reviewed removal; recompute the account including replacement row. No silent use of a War's future slot while that War survives. Truce-producing SetAgreement explicitly cancels the incompatible War, then applies its submitted interval as creator terms. |
| Explicit creator removal of War (the derived `War{a,b}` cascade target) | **Cancel the reservation; do not install an implicit Truce.** This is a creator deletion, distinct from ForcePeace/negotiated peace/expiry. Its exact preview shows War and objective-order/Blockade removals and that no Truce row is added. No peace/fulfillment/victory event is fabricated. A creator wanting300 Truce uses ForcePeace instead. |
| Participant/civilization deletion | Cancel every affected War/reservation through its reviewed cascade; never create a dangling Truce. Remove affected actual agreements/commitments/requests/routes under existing explicit cascade rules. |
| Delete only objective target, SetWarObjective or lost achievability | Existing objective invalidation/order cancellation/proposal rules apply; if War survives, reservation survives. No automatic Truce/capacity release until actual accepted peace, expiry or explicit creator War removal. |
| Queue cancellation, undo or branch change | Rebuild leases for the exact remaining queue/history view; release cancelled provisional ownership only. Previously accepted allocator marks remain consumed, as frozen. |

There is no independently named `DeleteWar` CreatorOperation in the copied command
registry: the existing typed derived War removal/cascade is the surface this row
addresses. A new public command must not be invented by the implementer. The
cancel-versus-convert choice for generic creator War removal is deliberately made
above for review, rather than inferred to be accepted. It prevents a dangling
automatic treaty on party deletion and keeps free deletion distinct from peace.
RemoveEntity targets an issued entity ID, not a War pair. Its RemoveReviewed
cascade can remove the dependent typed War. A standalone arbitrary-pair DeleteWar
form would need a separately approved additive command; this specification adds no
such command and does not claim that existing cascade grammar can target a War
as a top-level issued entity.

Coalescing is not a treaty renewal eligibility bypass for actors. An ordinary actor
cannot deliberately coexist with Truce and War to avoid the capacity check or
reset a Truce deadline. Incompatible duplicate Agreements reject their strict
Sorted key; duplicate Commitment terms follow the existing rejection/renewal
rules. The derived reservation has no separate duration, record lifetime or ID:
it lasts until its owning War terminates/cancels. The actual Truce then exists on
`[E,E+300)` and expires in phase2 atE+300. It permits no new offensive declaration
or hostile action during that interval and grants no Trade, TransitAccess,
MutualDefense, supply route or settlement entitlement. NoOffense here names the
permission predicate, not a new wire field or persistent flag. Creator overrides
remain explicit exceptions with reviewed provenance.


## Phase2 clocks, lifecycle facts and stellar issuance

Use the twelve phase2 subpasses specified by ecology in that exact order. Each
subpass freezes current entry roster; ended rows cannot warn/start later, and
new automatic stellar warns only within its Star operation. Each independent
operation saves one actor-null phase2 full current pre-operation State source,
latest actual applied creator cursor, authentic historical counters and tickT;
its complete group shares that source. Agreement expiry emits1 agreement_expired.
Natural War deadline converts its already reserved typed Truce key to exact
[T,T+300), sharing source for war_ended then agreement_formed; it allocates no
entity or extra treaty unit. Existing exact timed War-start producer has no fake
primary event and applies real Request/Fleet/C effects only at genuine crossing.

Hazard end_tick<=T removes latest full row and emits1 hazard_ended/Facts.hazard.
Retained start_tick==T emits1 hazard_started with unchanged full row; warning-only
emits0, no hazard_warning. Stellar ended row emits1 stellar_ended; retained
warning_tick==T/start_tick==T emit respective stellar_warning/started, warning
first when equal, same source/full record. No retroactive marker for endpoint<T,
no persisted announcement bit or identical mutation Change. Phase1 edited latest
schedule controls; canceled row emits no later marker. Backdated scope bytes are
not knowledge refresh. Authority uses one latest row; actor environmental winner
selects newest retained child per event identity BEFORE applying expiry, preserving
negative coverage and no resurrection of older moved record.

Due World environment advances oldclock+100 once, fertility+=min(50,max-current),
stress-=min(10,stress), base/max unchanged; emit1 environment_updated including
clock-only full-fertility/stress0, no catch-up/deposit/population/needs event.
Due Star oldclock<=T advances oldclock+1000 once, actual selection E=T. Stream
SHA256(non-NUL ASCII NYON_LIVING_V3_STELLAR_ACTIVITY || authenticated Genesis
seed32 || StarID16 || E:u64LE); first little-endian u16<6554 selects. Seed remains
authority Genesis evidence, never Actor32 clock foresight or new Snapshot field.
Every due attempt emits1 operation_changed containing complete Star clock Change.
Unselected performs no flare clock/cap/allocation checks. Selected computes checked
E+100/E+200 then literal impact2000/freight1/2 before current stellar occupancy.
At128 after prior endings/creations, emit environment_capacity_omission
{star,due_tick:E,proposed_kind:solar_flare,capacity:128}; no ID/success ordinal.
Hazard cap128 is separate. Successful selected creates one kind18 StellarEvent
using AUTO root/branch/lifecycleflag0/zeroactor/E/phase2/kind18/successordinal/
subjectkind2/StarID/original pre-Star full digest. Successful ordinals contiguous
per exact namespace in StarID order; unselected/full consumes none. Full digest
collision faults without salt/retry. Record is id,star,warningE,startE+100,endE+200,
impact2000,Fraction{numerator:1,denominator:2},autonomous provenance with same
source/phase2/nullactor/subjectkind2/StarID/allocationordinal. Emit exactly1
stellar_warning after clock event, no duplicate creation/start event. Both
selected-full and selected-success groups have2 events. All checked arithmetic,
source/event/byte cap and identity faults abort unpublished boundary; ordinary
stellar occupancy omission is its explicit separate result. Common remaining
visits include full sources, hashes/rosters/collision/history/cleanup; no partial
expiry/clock/Truce/flare publishes or large-owner drop occurs in poll.

## Central accepted-record literals and current reservations

Central traverses immutable original candidates in the exact Q total/local-head
order. Original identity/score/capture never changes after a preceding acceptance.
Revalidate ten stages reference/definition/order, intelligence, permission/path,
supply, workforce, protected needs, structural capacity, true physical funding,
incompatible reservations, private defender selection; stop first actual cause.
Stage8 sees true physical stock/refund, never hold-subtracted synthetic P; stage9
uses complete causal shared holds/floors/structural locks, with no extra physical
debit. Failure releases only this candidate's private prospective owners and
consumes no entity/AUTO/material success ordinal/lane. Accepted private promise
is ephemeral boundary ledger ownership, not persistent escrow or entitlement.

ConstructionJob full after={id:allocated kind8,world:terms.world,owner:actor,
definition:terms.definition,accepted_tick:T,completion_tick:T+catalog.build_ticks,
paid_alloy:catalog.alloy_cost}. HullJob kind9 has the same full fields then exact
target_fleet; null iff reserve_new_fleet. Construction owns industrial slot,
per-definition and queue reservations; Hull owns actual shipyard concurrency,
global Hull/Fleet-if-new and target<=16 reservations, not an industrial slot.
Acceptance creates no completed Facility/Hull/Fleet or initialization/credit.
Optional actual Facility scrap refunds floor(old.paid_alloy/2), usable only up to
actual storage room, discarded remainder; gross new cost remains catalog Alloy
and exact terms paid. Transaction checks every guard/collision before committing
scrap/refund/debit/reservations; failure preserves all originals.

New Route full after copies exact terms id/kind/source/destination/resource/batch
1..10/cadence10/reserve/source_owner=actor/receiver_owner, next_due=T+10,
suspended=false,commitment exactoption. Adjustment preserves old ID/next_due,
uses submitted exact validated terms and false only after current validation.
No Route.owner exists. Reserve formula is20 for Ore/Alloy and max(20,supported
next own due need) for Food/Energy. No acceptance-time cargo/debit/Shipment.
New undue positive cargo/floor hold remains until finalization, then releases;
no T+1 persistent guarantee. Adjusted overdue Route attempts once phase10,
old_due+10; no catch-up or normalization. Zero modifier consumes no cargo/floor/
Shipment slot but actual Route/treaty structural units remain. Old paid Shipments
never adopt adjusted terms or resembling replacement C.

Accepted existing Fleet motion copies complete id/owner/location/Hulls/Supply;
installs exact captured kind/target/path/objective/DefenseActionBinding and
not_before=T+1. Terms departure_not_before must equal that checked value.
Admission does not depart, pay Energy, consume credits, set voyage origin or
reset readiness. Captured objective/Request formation explicitly chooses binding;
current target resemblance cannot infer it. Non-Blockade motion clears Blockade.
Blockade installs exact {world:target,war_a:min(actor,currentEnemy),war_b:max(...),
accepted_tick:T} and kindBlockade/path/T+1/bothbindingsnull; actual standing effect
requires due clock and real Docked endpoint/War predicates. Travelling/nonidle
or incompatible whole-roster reservation rejects before replacement.

Board100 requires actual Ark in actor-owned Docked Colony, population>=200 and
ordinary ordered guards. Selected NEW physical per-Ark maximum is u32::MAX via
checked count addition only, no invented passenger capacity account/rejection.
Reached overflow faults whole boundary; earlier guards retain precedence. Copy
all Hull fields except count+=100; original voyage_origin is unchanged until its
first genuine paid nonReturn departure. Colony population-=100, other Society
clock/needs/specialization preserved, immediate exact workforce recomputation.
Complete group intent_decided→passengers_boarded→operation_changed Fleet/Colony
(fullChanges resources=[],passenger_delta +100/-100)→workforce_changed iff
actual assignment changed. Facts/Changes witness one conserved transfer, never
apply it twice. Creator Board uses actual intervention/cascade, not this group.

Prospective AUTO uses exact successful actor phase9 kind namespace and permitted
original H7 digest/subject, not later current-State recapture; actual C/Route/
Offer/Job full allocation order and successful counters must match their owning
production operators. Actor null lifecycle completion separately uses phase5.
Source registration uses full actual pre-group working State where required,
original actor H7 where declared, latest real logical creator cursor and no future
publication digest/Event/Receipt. Mixed final partition does not reorder causal
ledger/mutations/first fault. One shared remaining1..1024 charges every enumeration,
reservation/refund/copy/hash/source/fact/journal/cleanup child; invalidpublic0/>1024
consumes0, exhaustedinternal0 yields, Ready repeats0, fault/cancel/premature finish
retains capacity owners and no partial publication. Freshness swap installs only
one complete validated/sealed/journal-prepared result.

## Arrivals phase3 immutable entry and exact disposition

Capture one actor-null phase3 immutable full working State I3 after phases1/2,
with latest actual creator cursor and authentic original historical counters.
Every phase3 event uses this entry source; actual before/after rows are replayed
in Fleet-ID then nested Hull-ID arrival/unload order, then Shipment-ID order.
Original paid producer sources remain separate historical bindings, not I3 or
currentReceipt. Fleet final arrival applies its original paid whole-itinerary
clock and endpoint independent of later topology/technology. Emit fleet_arrived
then one full Fleet operation_changed for actual Docked/order result, resources=[]
and passenger_delta0. Move/Return clear; Explore and ordinary unbound Defend clear iff their target
is absent or equals the actual arrival World, otherwise retain; genuine
same original accepted Request/C/threat bound Defend retains only at matching
requester-owned target while current duty valid. Attack/Settle clear iff their
target differs from arrived endpoint, otherwise retain; Blockade retains its
separately bound standing action. No target resemblance rebinds an instance.
Arrival allocates no entity/credit/payment/contribution. Conditional forecast may
hold unsupported order consequence while completing known paid location.

Return passenger unloading only follows genuine eligible arrival and authentic
positive original voyage_origin matching current own Colony World. Transfer
actual supported amount from Hull to Colony, retaining remainder/origin, checked
population/physical count conservation; clear origin at count0. No stationary
retry, stale/unknown origin substitution or population from creator relocation.
Emit exact passengers_unloaded (which owns the actual Colony population change),
then exactly one operation_changed with the full containing Fleet Change only,
resources=[], passenger_delta=-actual unloaded count; actual workforce companion
follows iff changed. No duplicate Colony Change represents that same transfer. Every Hull transfer occurs once; old paid/source proof
survives bounded history retirement and fork ancestry.

Shipment original_travel_ticks is immutable positive original paid duration,
not reconstructed from changed active schedule. Genuine War return resets active
departure=T/arrival=T+original duration once and never another return/payment.
Outbound=false flag; Returning=true; Waiting retains whether outbound or returned
leg. Active travelling arrival<=T is due; Waiting retries every phase3 including
early creator Waiting, clocks unchanged. Choose endpoint/current ownership before
storage: outbound changed owner (includingnull) chooses captured/unowned recipient
first; unchanged intended owner plus active War starts one return without storage
read, otherwise ordinary delivery. Returning uses original source current owner
and never bounces. No missing-endpoint workaround; explicit creator closure must
already resolve it. Storage requires entire units or none. Full storage preserves
cargo/binding, sets Waiting retained flag, emits shipment_waiting each genuine
retry even equal manifest bytes. Return initiation emits shipment_returned,
recipient=null,resources=[],full AFTER Returning/new clocks; no same-phase return
arrival. No storage or resource debit on that initiation.

Atomic unload transfers all units once and removes manifest. Outbound intended
owner→shipment_delivered; differentnonnull→shipment_captured; unowned→delivered
recipientnull. Returning dispatch_owner→shipment_returned; differentnonnull→
captured; unowned→delivered recipientnull with returned leg explicit. Terminal
Facts.shipment contains full row normalized Waiting→actual active leg immediately
before removal; one transferred ResourceFact(resource,positive units), actual
recipient. Waiting/return-initiation contain full post-transition row, no resources.
No input/output/refund/paid role double-counts historical withdrawal. Override,
original duration, endpoint/parties/paid provenance remain unchanged.

Only genuine paid outbound with override=null, actual recipient original intended
foreign party and original admissible Trade/Aid binding grants relationship:
Aid+10 per100 real delivered units capped20; Trade+1 per100 capped20; preserve
remainders, absent relation starts base0/other counters0. Returning/captured/
unowned/creator override grants0. C delivery belongs exact original same surviving
C/Route source instance and real actual attempt; no C2 matching terms substitutes.
Set last_attempt=T on real attempt, preserve prior Anchor; genuine fulfillment
source is installed only at next ordinary phase3 through prior sealed history,
never this Receipt's future Anchor. Creator-only, paused/save/read/fork performs
no handoff or payment. Original paid delivery remains historical after later
same-T capture; current rights may immediately suspend under their own adapter.
Source/resource/arithmetic/capacity/fault aborts full unpublished boundary; common
remaining visits include nested sources, copies/hashes, events and retirement.

## Creator semantic instances, constructors and value no-ops

Every valid SetWarObjective is a genuine replacement, including equal submitted
kind/target/endpoints. Resolve ascending current War parties and actor exactly
one party through typed existing/earlier-local Target rules. Construct full named
objective with submitted kind/target/pair,count0,pending,last_evaluated_tick=null,
witness=null. Defend formation requires live actor-owned Colony World and pair
null; Occupy live OTHER-party Colony World and pair null; Secure live Lane and
distinct plain World IDs in its exact declared System direction. No threat is
required for creator Defend, and creator selection grants no current permission
or success. New pending formation cannot reuse terminal historical target waiver.
Preserve other actor objective/War parties/declarer/clocks; clear entire Peace on
every replacement. Cancel exact previous-instance future Fleet orders, preserving
already-paid motion/credits/passengers/Supply, with exact Blockade dependencies.
Original instance locator is authenticated RevisionID/operation_index/War/actor,
not tuple equality or new nonce. Paused replacement reruns no completed sample;
queued phase1 replacement may genuinely evaluate at that boundary's later100
phase. Newly phase9-admitted War cannot retroactively evaluate phase8.

ForceWar required objectives contain exactly both parties sorted, each constructor
above with explicitly submitted count0/pending/evaluationnull/witnessnull; other
values reject rather than normalize. Full resulting War.peace=null. Require
end=start+1200,end>applicationT and checked end+300. Valid past/current/future
start follows actual capacity/deadline contract. Every ForceWar replaces War and
both instances even byte-equal, removes incompatible actual legacy keys and
cancels exact old-instance actions. start<=T executes actual Request→Fleet→C
active effects once; future start does not suspend early and receives only its
later genuine crossing. Phase1 replacement supersedes old inherited crossing.
No autonomous declaration/negotiation, Energy cost/refund, offense contribution,
force escrow or success marker is fabricated.

SetFleetOrder is a revalidated value setter, with both required bindings explicit.
Equal bytes and unchanged valid dependencies emit no full Change/new paid action;
changed bytes/dependencies produce real containing Fleet effects. Earlier genuine
instance replacement must already cancel the old binding; resemblance cannot
reattach it. A later creator attachment uses its actual operation source and
still grants no presence-only negotiated credit. SetRouteSuspended(false)
revalidates actual endpoint/participant/permission before equality. Only genuine
true→false original bound Route transition with C suspended/[endpoint_changed]
can clear C to ready/[] under complete current first causes, preserving other
fields; that actual C Change is indirect. Repeated already-false is no latch-clear
cause. Other operations retain their own exact semantics, not a blanket equal
setter replacement or bypass. Missing-target removal is not a no-op.

Compute semantic effects first; omit full Record Change only when bytes equal
AND no actual resource/passenger disposition. Row-equal replacement stays in
actual indexed immutable Revision; legal empty reviewed_effects still advances
accepted sequence once and emits one creator_intervention with empty direct
Changes, cascade iff genuine indirect effects. No equal War marker or fake header
Change is added. Repeated replacement operations have separate original locators;
last survives, earlier cancellation occurs once. Effects remain operation-indexed,
not batch-net differences. Authentic history/ancestry, not State digest equality,
authenticates instances through fork/save/import. No implicit version default.

Same-instance ordinary pending progress preserves Peace byte-identically.
Ordinary terminal success/invalid and creator geometry invalidation preserve
existing Peace and original objective instance; creator invalidation does not
negotiate while paused. Pointer-only next ordinary handoff changes no semantic
Peace/proposal. SetWarObjective and ForceWar clear it as above. Actual expiry,
accepted Peace and ForcePeace remove War/Peace under explicit Truce conversion;
generic participant/War removal creates no implicit Truce. Read/save/fork performs
no proposal, response or handoff. At ordinary100 logical K freeze actual War-pair
roster and entry Peace: none=fresh, prior proposed(created<T)=one response,
created==T=no response, refused=retry under exact semantic-change rule. Newly
refused is not retried in same K; only active same-instance intervals negotiate.
Known supported counter-objective achievable=true refuses; false may accept;
unknown defers rather than assert false. Strict recovery outranks discretionary
objective expenditure. Full proposed/refused Peace is recorded as the complete Peace leaf in
Facts.peace.record; Facts.peace.war_a and war_b identify the exact canonical War
pair. Authenticated execution/replay updates and checks that exact War's embedded
Peace; the sealed State authenticates the resulting full War. Emit exactly one
existing peace_proposed/peace_refused event with this pair and complete Peace leaf;
no additional War Change carrier or companion event is required or permitted for
this producer. Peace.reason is objective_complete/objective_invalid
on the corresponding proposals; no additional independent Facts.reason field is added. Refused retry uses authenticated declined baseline
and semantic objective/eligible-force/knowledge changes, ignoring pointer/
transport/time-only changes; no Receipt-being-built Anchor or hidden force shortcut.

### Peace retry finite semantic comparison

Fresh K proposal requires authentic terminal same-instance objective; pending
force_unachievable does not originate one. Choose lowest eligible actor ID,
install proposer/createdT/considerednull/proposed/reason objective_complete or
objective_invalid. Retained pending proposal is never overwritten by another
terminal actor. Refused retry locates the exact latest peace_refused matching
original War/objective instances, pair/proposer/created/considered and authentic
ancestry. Reconstruct BOTH original H7 inputs and same-boundary evaluated own
objectives, never current root or sibling/parent future. A current terminal actor
is retry-eligible only when its own finite semantic comparison differs from that
baseline; choose lowest eligible actor. New proposed replaces refused with
createdT/considerednull, exactly one peace_proposed and no War ID/cancel event.
Conflicting purported baseline faults replay; cache eviction regenerates ancestry.

Stream private PeaceRetryInput in order: actor,kind,target,secure_endpoints,status,
consecutive_witness_count,reference_state,observations,force_choices. Reference
entries visit exact target then fixed world_a/world_b: typed ID, live, permitted
System/Lane geometry option, own-or-known owner tagged known/unknown with value.
Observation entries visit required goal Worlds in same endpoint order: absent/
stale/fresh knowledge, known/unknown owner, permitted hostile Escort count,
operational batteries, Occupation, hostile Fleet IDs and enemy threats. Unknown
quantities remain unknown. Normalize threat semantic enemy/target/count/occupation/
hostile IDs; exclude authenticated observer/timestamp/digest transport identity.
Freshness class uses age<=300; only class crossing changes semantics, not raw
observed/received tick. Force choices visit own Fleet ID then exact endpoint:
complete Hull IDs, live Escort count, location, readiness class, known reference
compatibility/path support, selected Lane path option/travel ticks, known required
payment, physical fundability/needs compatibility/objective strength. Known empty
path/cost0 differ from unknown. Exclude unrelated HP unless live roster/strength/
eligibility changes, foreign HP/stock, later private R/O/M holders, last_evaluation,
witness, Peace fields, Event/Receipt IDs, caller clock and unrelated resource/
environment/name metadata. Lists stream in stable order under shared visits;
no retained Cartesian list, persistent fingerprint field/hash or new ID/domain.
Replacement clears Peace and never compares across obsolete instances. Counter
change alone on pending actor cannot qualify a proposal. This complete comparison
rather than time/pointer/rehashed identical threat drives retry.

## Production phase5 actual ownership and reports

Process due retained paid jobs by ID once (completion_tick<=T); no catch-up or
second Alloy/worker payment. Resolve actual owner/World/catalog/target before
allocation. Missing reference faults; proven ownership, retained shipyard or
Docked target incompatibility cancels with floor(paid_alloy/2) actual World
refund capped by storage, removed-job discarded excess fact and reservation
release. Construction cancellation emits one construction_cancelled; Hull job
cancellation one operation_changed, no duplicate refund_discarded. Missing World
cannot teleport a refund. Successful completion converts exact paid reservation
to occupancy. AUTO lifecycle flag0/zero actor/T/phase5 uses original pre-job full
State digest, World-kind3 subject, independent successful kind counters: Facility7,
Hull11 then Fleet10 iff new Fleet. Rejection/cancellation consumes no ordinal;
collision faults, never salts/retries. Facility initializes full catalog HP
(defense max, otherwise0), operational/enabled, paid_alloy preserved, next_due
T+period for periodic effect else null, next_repair=null,reasons=[],facts=null.
Hull initializes catalog-max HP, return_credit=false,next_repair=null,passengers
{count:0,voyage_origin:null}. New Fleet is owned/Docked/singleton/order=null,
Supply both null/never_supplied,blockade=null. Existing Fleet preserves fields and
adds one Hull. Primary completion emits one construction_completed/hull_completed
with full job/asset Changes, resources=[]; immediate workforce recomputation emits
one workforce_changed iff changed under its own pre-assignment source, before
next job. Completion itself grants no automatic departure or supply payment.
The later Supply subpass freezes its then-current Fleet roster including newly
completed Fleets, so an eligible new singleton at T%100=0 receives its genuine
later Supply attempt under that subpass, not a completion grant.

Due-needs Food/Energy hold equals min(actual prefix stock,due need based on
phase5-entry population/needs); recompute before every guarded debit and after
local stock mutation including outputs/refunds. Nonlife consumers use unheld
stock; only exact positive Farm Food waiver applies. Hubs process due clocks once
as oldclock+period. Energy2/Ore1 outputs are silent; fallback below Alloy120
requires physical Energy4/Ore4/output room first, then Energy hold, and outputs
Alloy1. Threshold skip advances100 with no guard. Energy attempt replaces shared
Hub reasons; Ore only when shared empty; fallback only with its own nonempty
reasons. Ordinary Hub attempt emits0; held fallback emits exactly one denied
needs_reserved hub_fallback{world}, no RecipeFacts or generic duplicate.

Current completed periodic Solar/Extractor/Foundry/Farm/Lab attempts require
owned habitable Colony and next_due<=T; passive/inapplicable/non-due emits0 and
preserves fields. Due disabled advances oldclock+period and blocks disabled,
no staff/funding/environment read. Enabled missing one worker blocks exactly
workforce_unavailable{required:1,available:0}. Otherwise physical inputs in resource
order, matching Deposit, output room are collected/sorted before Needs hold.
Extractor selects lowest-ID Ore Deposit with>=4, not residue aggregation; none
reports max matching residue or0. Success SolarEnergy8; ExtractorEnergy2/Deposit4
→Ore4/stress contribution1; FoundryEnergy4/Ore4→Alloy2/stress2. Real stress_added
is min(10000,checked(old+contribution))-old; contribution remains fixed even at
cap. Farm retains its exact q/environment/full-physical/waiver operator. All due
attempts advance once; actual RecipeFacts inputs/outputs/blocked/staffed are
complete, Facility reasons equal facts, unrelated repair fields unchanged.
Ordinary block/success emits one facility_blocked/produced; held denial or real
Farm waiver emits its needs_reserved immediately before that event, exactly2.
Recipe ResourceFacts describe actual applied inputs/outputs once; Deposit is not
invented Inventory input. Only genuine Farm q0 carries Food0; other blocks empty.

Lab after common staff checks target then prerequisite/incomplete/progress<cost,
then physical Energy2/Alloy1 then Energy hold. Each due clock advances20. Failed
Lab emits only facility_blocked, with preceding Needs guard iff held. Successful
Lab adds exactly1 point; terminal point installs completion and clears target
before next Lab. Emit facility_produced with resources=[], then exactly one
research_progress OR terminal technology_completed carrying complete Research
before/after, lab=Some(id), actual Energy2/Alloy1 facts. No duplicate target-change,
point/payment or generic Civilization event. Free F selection never pays a point.

Repair traverses actual entity IDs globally. Eligible due living Hull requires
own Docked World; battery requires owned World/Defense effect. Exclude actual
phase4 CombatWorlds and current hostile live Hull/battery co-location including
disabled battery. Unknown private forecast guard remains held, never root-read.
Noneligible emits0. Full-HP due clears clock and emits one full operation_changed
without debit. Damaged due computes oldclock+10 before funding: physicalEnergy1,
then hold, then HP+1 capped and clear clock iff full. Denial still advancesclock.
One primary operation_changed contains exact selected Hull's containing Fleet or
battery Facility Change and World inventory Change iff paid. No other Hull/Fleet
or production facts/status change. Held guard repair{fleet}/battery_repair{facility}
precedes primary event with same source. The exact changed Hull and traversal
bind repair{fleet}; no per-Fleet aggregate guard. No combat/refund/worker cost.

Supply visits every Fleet ID under its actual pre-operation source. Derived
readiness supplied iff T<original deadline, unavailable when any component exists
otherwise never_supplied. Off100 or travelling/foreign Dock only refreshes
readiness via one operation_changed iff changed; no fake attempt. Owned Dock at
T%100=0 requires exact live Hull count. Physical shortage installs unavailable
paid0 attempt, preserves original success and emits one fleet_supply_unavailable;
held shortage prepends one needs_reserved supply{fleet}. Paid success debits
Energy=count once, records matching paid attempt/success deadline=T+100/readiness
supplied and emits one fleet_supplied with full Supply before/after and actual
input ResourceFact. No separate readiness event or invented supply clock.
All operation events share actual actor-null phase5 pre-group source/latest
creator cursor; assignment uses its own causal source. Nested copy/hash/debit,
identity, emission, retention and cleanup share remaining visits and atomic
publication. No owner destruction or partial payment on cancel/fault/finish.

## Conflict phase4 execution and complete causal sources

At T%10=0 process genuine automatic noncombatant retreats by Fleet ID, then one
globally simultaneous combat round. Travelling Fleet has no combat presence.
Noncombatant means every current live Hull in this Fleet has damage0; another
friendly Fleet is no implicit escort/merge. Attempt iff host World owner is a
current declared enemy or a hostile positive-damage piece threatens it there.
First retreat failure: lowest target HullJob reservation, lowest missing-credit
Hull, no owned Colony, no permitted route. Actual reachable owned Colonies tie
whole-itinerary ticks then World ID then lexicographic Lane path. A failure spends
nothing and changes no Fleet field; emit one retreat_blocked/Facts.retreat with
before=after, destination=null,outcome=blocked and exact first RetreatReason.
Success consumes existing whole-roster return credits once, costs0 Energy,
installs Return with chosen path/target/not_before=T/both bindings=null, full
final Travelling T/T+duration, clears Blockade and preserves Supply/HP/passengers.
Emit exactly one fleet_departed/Facts.retreat with complete before/after,
destination nonnull,outcome=departed,reason=null. No generic duplicate Fleet
Change or combat contribution. Each attempt uses its own actor-null phase4
immutable actual pre-group full State source/latest applied creator cursor.

Combat captures ONE immutable pre-round source after retreats. Resolve active
War/no-Truce enemies and target class then decoded target ID from all current
pieces: class0 live Escort, class1 owned live battery, class2 Scout/Ark Hull,
then lowest hostile entity ID. Fleet ID is no target tie. Unowned battery has no
combat faction/target. Enabled operational battery damage4, Escort damage2,
Scout/Ark0.
Disabled battery remains targetable but cannot fire. All pre-round shots fire,
even when attacker dies in this round. Nominal target selection is simultaneous;
allocate actual realized HP loss by target ID then attacker ID with
min(nominal,remaining pre-round HP). Full CombatAction rows sort attacker ID,
include actual target/containing Fleet/owner/War and exact bound Request option;
never infer target owner, War or contribution from presence/order resemblance.
Surviving actually damaged piece with null repair clock gets T+10; existing
nonnull clock stays, destroyed piece gets none. World with no hostile engagement
emits0; each engaged World emits1 combat_round with full CollectionKind/key-sorted
Fleet/owned-battery before and logical damaged after, attacked-ID sorted Damage
rows and complete actions including overkill0. World combat Facts precede
Request contribution groups in C ID order, then
global destroyed entity ID removals. All round facts share original pre-round
source and report this one atomic mutation, not sequential damage commands.
Only current supplied matching original bound named-enemy Escort realized damage
qualifies for Defense credit; battery, overkill, creator damage and docking do
not. Later phase5 renewal is no retroactive qualification. Emit exactly one
defense_action_recorded per actual considered bound contribution group and one
obligation_changed iff its complete C bytes change; never a fulfillment Anchor
into the Receipt being built. Full private action carriers are authority evidence.

A named Defense action group requires positive-fire Escort with full original
bound Defend at target, original owner=R.defender; authentic live accepted R and
mutual_defense C with exact IDs/creation ticks/parties/threat digest/C.request;
actual target owner=R.requester; original named threat identity preserved; selected
shot enemy=R.threat.enemy with actual attacker/enemy active War/no-Truce and no
requester/defender forbidding War. Stale unknown never proves absence. A later
real lawful ordinary shot may qualify from an earlier explicit creator binding,
but creator intervention itself cannot fulfill. Attack/Move/Blockade/unbound order,
other-enemy fire and presence do not form a named group. CombatAction association
can retain its real binding even for nonqualifying fire; DefenseShot list contains
ONLY actual matching named-enemy shots from full round actions. One action binds
one R, no double credit; multiple genuine bound Fleets aggregate one C/R group.

Defense total_realized sums all matching shots; credit_realized sums only shots
whose original containing Fleet supplied at Sround (T<authentic deadline).
Deadline equality is unsupplied; prior still-live success qualifies despite latest
failed renewal. No reallocation from unsupplied/battery consumed HP to supplied
attacker, and later phase5 payment has no effect. Outcome contributed iff credit>0;
else no_realized_damage iff any matching supplied shot, otherwise readiness_denied.
No matching named shot emits0/no attempt even when Fleet present or unsupplied.
For nonempty group emit one defense_action_recorded then obligation_changed iff
full C bytes change, C-ID/R-ID order. Both outcomes write last_attempt=Some(T)
once; contributed sets fulfilled/reasons=[], noncredit preserves prior status/
reasons. Both preserve full id/parties/kind/terms/created/expiry/route/request and
prior last_fulfillment. Request stays accepted/outstanding. Only genuine positive
outcome may supersede pending positive fulfillment for next ordinary phase2 prior
history handoff; noncredit never clears or replaces earlier fulfillment. No shot
resource cost, AUTO/action nonce, target-HP borrowing or currentReceipt Anchor.

Actual Hull destruction removes it from full containing Fleet, retaining other
Hull fields; last Hull removes Fleet. Exact destroyed before records may have
logical post-round HP0 only at this registered replay/removal position, not a
completed root or arbitrary import. Destroyed Hull emits1 hull_destroyed full containing Fleet/target-job cascade;
if positive passenger count emit1 passengers_lost immediately afterward, containing
Hull/world/owner/beforeCount/after0 with population fieldsnull. Zero count emits0;
Fleet Change delta=-count represents this same loss once. Battery emits1
operation_changed full Facility removal. Genuine combat target-job cancellation
uses ordinary half-Alloy refund and
actual storage discard; creator no-refund deletion remains a separate operator.
Destroyed battery has its actual Facility removal and no cancellation refund.

Occupation World order is decoded ID. Unique enemy Escort owner can progress
only without original owner's Escort/enabled operational battery defenders or
ANY third-party Docked Fleet hostile to claimant, including noncombatants. Multiple
claimants is contested. Invalid predicate removes retained Occupation with1
operation_changed iff existed, else0; same valid claimant increments1, different
starts1, result<100 emits1 exact full Occupation Change. No catch-up/saturation
or progress100 retained row. At100 capture
preserves population/food/inventory/environment, changes World/Colony owner,
clears workforce, cancels old paid jobs with exact half-refund/overflow, suspends
mismatched endpoint Routes and removes Occupation/claims. Former-owner→captor
relation decreases30 floor-60, absent relation uses base0/other counters0.
Primary colony_captured owns these full sorted Changes once. Then affected
Civilization status transitions by ID, hosting workforce recomputation, then ALL
retained C/Request/Fleet eligibility adapters under the complete continuing-rights
contract, each its actual prefix source, before next World. This supersedes the
historical narrow Supply-only adapter; no richer retained family waits until100.
Status is Active iff any current owned Colony, otherwise Dormant; retain identity,
policy/research/assets. Byte-identical status/workforce/eligibility emits0.

Settlement removes claims at owned or actually armed defended Worlds. A genuine
paid Settle arrival opens a half-open50 window, later join iff arrival<close;
original claimant linkage survives future Move/Return editing while physically
Docked. At close no eligible claimant clears the window before any new one.
Rank eligible civilizations by full SHA256 bytes of ASCII NYON_LIVING_V3_CLAIM_RANK then exactly one NUL,
RulesV3 u32LE, seed32, World ID16, close Tick u64LE, civilization ID16; tie decoded
civilization ID. Consume winner's globally lowest eligible Ark Hull ID, with its
actual entire passenger count conserved into new Colony, not Fleet-first choice.
World becomes Energy20/Ore20/Alloy0 with Food/environment/deposits preserved;
explain discarded old stocks once. Colony initially balanced, Hub dueT+10,
population/needs dueT+100, last needs=null, workforce=[] then real assignment.
Remove claims/Occupation, clear only associated Settle orders and apply neighbor
same-System/direct-Lane→winner relation decrease10 floor-20 once per civilization.
Every primary/status/workforce/continuing adapter source is its own registered
phase4 prefix, without currentReceipt witness, victory/peace or guessed IDs.
All roster/copy/hash/action/conservation/capacity/cleanup work shares remaining
visits; fault cannot publish a partial round, capture or founding.

## Free research target F

At T%50=0 logical F visits original H7 Active actor IDs after M and before audits.
Only own original Research/policy/linked Catalog chooses a target; current truth
validates it but never rescans for another preference. Eligible exact technologies
are incomplete with all prerequisites completed and own progress (absent=0)<cost:
agronomy100/[], efficient_logistics100/[], resilient_habitation150/[agronomy].
Trader order Logistics,Agronomy,Resilience; Guardian Resilience,Agronomy,Logistics;
neutral/expansionist/industrialist Agronomy,Logistics,Resilience. Retain original
zero-based ranks after filtering; score=checked(3000-1000*rank). No eligible or
original desired==original target emits0 and does not clear a target. For a
changed desired validate current typed actor/Catalog/definition, Active status,
current prerequisites/incomplete/progress, then current desired equality, in that
order. Current equality emits0; no current fallback target. The fixed ordinary
schedule has no genuine post-H7 research eligibility-loss producer: an alleged
loss under that schedule is unpublished source/replay fault, not a fabricated
rejected history. Conditional later connected-producer loss requires separately
reviewed genuine timing and uses one first reason: invalid_target{target:actor,
rule:research_reference_changed}, dormant_restriction{actor}, or
invalid_target{target:actor,rule:research_target_ineligible}, respectively.

Real switch emits intent_decided then research_target_changed in B9, both actor
phase9 provenance with original phase7 H7 snapshot. Decision has next actual
actor phase9 ordinal, select_research{technology:desired}, candidate=null,
target=null, score above, tier=ordinary, observed_tick=null, Accepted, reasons
ordered policy_score{policy:original policy,component:research_target_priority,
value:score}, accepted{rule:research_target_selected}. These component/rule Slugs
are explicitly permitted NEW free-selection labels, not material components.
Facts.research is {actor,technology:desired,before:full current Research,
after:{target:Some(desired),progress:unchanged,completed_technologies:unchanged},
lab:null,resources:[]}. It owns that actual mutation with no duplicate generic
Civilization Change, points, technology completion or workforce. No AUTO/entity
ID, material lane, Trace, enumeration hash or AdmissionAttempt is consumed.
Original H7 stays phase7; no self-Receipt or target Slug-as-ID is allowed. Scan,
validation, copy/hash and event/ordinal checks share remaining visits; zero-record
outcomes are not free unbounded scans. Cancel/fault retains owners and publishes
no target without its complete group. Fresh independent vectors must cover ranks,
filtering/no-ops and authentic paid Lab histories before human package approval.

## Inherited free bilateral Agreement production

At ordinary T%100=0, logical H after older Offers O and before K enumerates
canonical decoded party pair then Trade/Nonaggression kind. Require both original
Active phase7 captures: both directed totals>=0 and captured verified own-Colony
connection for Trade; both>=20 with no connection condition for Nonaggression.
Any retained same-pair War including future-start, active Truce, or retained
same-kind Agreement excludes capture eligibility and creates no attempt. Material
AdmissionTerms.agreement remains a declared shape but is forbidden as an
autonomous material candidate: no lane, Trace, PolicyScore or material attempt.

For original eligible pair revalidate actual current participants Active, absent
same-kind key, no retained War/active Truce, bilateral thresholds, current Trade
connection, distinct treaty capacity, then checked T+1200, in that order. Actual
Trade connection may use another current own-Colony pair over the SAME parties;
removed original witness alone does not cause reference_missing when one exists.
No current gain creates an originally ineligible pair. First failure alone emits
two Rejected free Decisions; path_unavailable represents actual no connection,
BilateralThreshold carries direction-specific actual totals, admission_capacity
uses each Decision actor subject and true treaty occupancy. No IDs/resources/holds
or pending Agreement are installed. Success installs exactly
Agreement{kind,participant_a:a,participant_b:b,start_tick:T,end_tick:T+1200} at
absent typed key without Relation credit or entity/AUTO allocation.

Emit intent_decided a→b then b→a; success only adds agreement_formed with one
Facts.change: operation_index=null, before=null, after=full Agreement,
resources=[],passenger_delta=0. Each Decision has phase8,candidate=null,score0,
tier=ordinary,target=Some(peer),observed_tick=null, original own H7 snapshot and
exact agreement action. Accepted success reasons in order are
BilateralThreshold{peer,forward:actual own→peer,reverse:actual peer→own,required:0|20}
and Accepted{rule:mutual_trade_with_connection|mutual_nonaggression}; rejection
has exactly one actual first free cause. Actor phase8 ordinals continue from
prior P/R groups, never reset per pair. Agreement Change uses one actor-null
phase8 full current pre-group source/latest real creator cursor; Decision events
use each original H7 capture. Local group order survives final phase buffering.
Retained active/future key is never autonomously extended. After real phase2
expiry a later fresh original capture may reform the same typed key with new
T/T+1200, without backdating a gap. New H right grants no same-capture material
candidate or Transit permission. At most240 groups/480 Decisions/720 events;
source/path/copy/hash/retirement share the remaining visits and global receipt
caps. Cancellation cannot publish a half-pair. Actor projection never shares the
peer's private capture/audit; canonical evidence remains creator/replay authority.

## Creator traveller resolution and current-reference retirement

V3 EditFleet keeps its existing target/owner/world command shape but accepts a
Travelling source as an explicit creator override. Resolve actual submitted typed
Fleet, owner and surviving World at this operation prefix; preserve the actual
HullJob reservation guard. A reserved target Fleet requires genuine prior
CancelHullJob before relocation. Successful EditFleet sets owner to submitted
owner, location=Docked{submittedWorld}, order=null and blockade=null, preserving
full Hull IDs, HP, definition, credits, repair state, passenger counts/origins and
authentic Supply components except independently required ownership cascades.
Its direct full Fleet Change proves Travel→Docked with resources=[] and
passenger_delta=0. It terminates that paid itinerary; no future automatic arrival,
Energy/refund, unloading, new credit/origin, resupply or fulfillment is produced.
V1/V2 retain their original versioned interpretation; this V3 Travelling
creator override does not amend either prior contract.

For RemoveEntity World/System compute actual issued closure from the complete
pre-operation State after earlier batch operations, plus exact derived keys,
using stable collection/key order. System closure includes its actual children
and incident Lanes. A Fleet with Docked World or Travelling origin/final World in
the closure must have been relocated earlier to surviving submitted World or be
explicitly included for removal. A Shipment with either source/destination World
in the closure must be removed once, regardless of disposition; existing commands
cannot rewrite its endpoints. SetShipmentDisposition is not endpoint repair.
Otherwise reject through existing referenced-target/cascade checks. There is no
nearest-World placement, teleport, implicit endpoint rewrite or replacement ID.
RemoveReviewed assertions must equal actual issued/derived closure; RemoveListed
is legal only when no derived records are involved. A traveller resolved earlier
is absent from the later current closure and cannot also be asserted for removal.

Only a removed future Order target/path cancels that future order to null while
preserving current paid motion. Removed passenger voyage_origin clears that
origin to null in the containing Fleet Change, preserving count/HP/credit.
Supply references follow their independent historical exception above. Paid
Route/C removal does not rewrite a surviving Shipment's original provenance.
Intermediate paid Lane/System deletion cannot alter whole-itinerary motion or
Shipment original duration when both final endpoints survive. EditWorld System
relocation preserves World identity and existing paid endpoints/clocks; future
orders/routes/fixed Secure endpoints are revalidated against actual new topology
and receive explicit cancellation, suspension, removal or pending invalidation
before aggregate validation. Distance-only Lane edits retain valid future order
identity and affect only a later departure's current route cost. No automatic
rerouting substitutes another path/target. Suspended Route is no missing-endpoint
waiver. Terminal objective references retain only their authenticated historical
exception. Current Fleet location and Shipment endpoint remain strict live refs.

For nested Hull RemoveEntity resolve its unique current containing Fleet. If any
Hull remains emit one direct full Fleet before/after, passenger_delta=-removed
Hull.count, resources=[]; preserve compatible target HullJobs and other Hulls.
Whole-roster Request-bound Defend is canceled with actual derived Order assertion;
otherwise a still-valid unbound/objective order remains for later actual guards.
If no Hull remains emit that same direct Fleet before→null, never an empty Fleet
or separate Hull row. Exact issued dependents include containing Fleet ID and
all actual target HullJob IDs, excluding primary Hull; derived closure includes
Order{fleet} iff nonnull and every actual required key. No separate Blockade
removal tag exists: it disappears inside the Fleet. RejectIfReferenced rejects
this real parent dependency; missing/extra/duplicate assertion rejects atomically.
Last-Hull target HullJobs are indirect creator removals with NO refund; release
paid reservation/occupancy once. Genuine earlier CancelHullJob instead performs
its ordinary half-Alloy refund/overflow before removal, so the later closure sees
no canceled job. Do not borrow combat refund semantics. Sequential Hull removals
use actual preceding results; after last Hull removal any later missing Fleet or
Hull target rejects the entire batch at that actual operation index.

Destroyed Fleet passenger_delta is minus the checked sum of its actual physical
Hull counts once; resources=[] since fuel was already spent. Destroyed Shipment
has one discarded ResourceFact for its actual positive units and delta0; no source
refund or receiver/C/relation credit. Destroyed World explains its positive current
inventories as discarded in resource order. Deleted Colony has passenger_delta=0;
its full before→null proves population loss without conflating Hull passengers.
Primary World/System removal is direct; dependent travellers/orders/origin clears
and other rows are indirect, even when asserted. Nested Hull target's containing
Fleet is direct; target Job removals are indirect. Every operation emits exact
full Changes at its actual index. Creator Revision partitions direct Changes in
creator_intervention and indirect Changes in creator_cascade iff nonempty, without
changing actual operation mutation order. No autonomous arrival/departure/combat,
paid fulfillment or currentReceipt witness is manufactured. Original paid sources
remain historical in full before records; creator operations use their actual
pre-command CreatorCapture and accepted allocator marks.

Preview computes the complete closure/conservation first and compares submitted
reviewed_effects byte-identically by operation_index/collection/key. Existing
typed reference/definition/range/reservation and cascade mismatch precedence
remains index-first; no new traveller-unresolved diagnostic is added. Checked
arithmetic, source, event/byte capacity, cancellation or stale publication abort
the whole unpublished candidate. Shared 1–1024 visits include roster, nested
copy/hash, source, journal and retirement; retain capacity-sized owners on fault,
cancel or premature finish. No partial relocation/destruction or allocator
publication is legal. Removed current occupancy does not recycle issuance IDs
or prune original ancestry; no implicit version migration is permitted.

## Failure order, resources and identity

Keep the frozen stage order:1 reference/definition/order,2 intelligence,
3 actual permission/path,4 supply,5 workforce,6 protected needs,7 capacity,
8 physical funding,9 incompatible asset reservations,10 private defender selection.
Category0 ordering and funded-needs/cross-colony guards do not change. A War cannot
take a slot already needed by an earlier protected category0 admission. This rule
adds no urgency, utility, favorable forecast victory or settlement/loot credit.

For a captured-eligible new War whose next future Truce would exceed256, stage7
uses existing `admission_capacity {capacity:agreements_commitments,subject:actor,
limit:256,reserved:N_live_before_attempt}`. At a full account reserved is256, not
257 and not a rival's stock quantity. It is an exact central denial fact in its
authorized audit, not a new public global count in the phase7 source snapshot.
No force/funding hold survives that failed attempt. Do not run the private defender
selector merely to append a second reason. A stage6 needs failure beats this
capacity failure; an otherwise independent stage8 funding failure does not.

**Closed registry and War120 reachability:** AdmissionCapacity has no `wars` tag.
With at most16 civilizations and strict unique canonical War pairs, there are
only `16*15/2=120` possible War rows. At120 rows a new legal pair does not exist:
an attempted existing-pair declaration fails actual compatibility at stage3,
before stage7. Therefore no new tag is needed for a reachable autonomous121st
War denial under these exact limits. Root/import validation still enforces the
120 collection limit and rejects duplicate/invalid pairs through its existing
structural validation; it does not invent a CandidateAttempt capacity reason.
Queued future pairs involving subsequently deleted/new civilizations are treaty
leases, not extra current War rows or a fictitious live War capacity count.
Do not encode a supposed121st legal War as agreements_commitments, an arbitrary
slug or QueueCapacity outside the frozen central contract. If a later proposal
raises civilization capacity or creates a new distinct War lifetime token, an
explicit `wars` tag/order amendment needs separate approval/vectors then. Existing
inherited TruceActive/other reason variants are not substituted for the integrated
central first-failure table. This contract leaves that vocabulary unchanged.

Successful declaration pays0 and starts no movement. Hold its exact declarer
force/account only after earlier guards pass and before private defender selection;
failure there releases its own new War/future-Truce/force holds and records the
unchanged candidate. Valid null-force defender objectives remain legal. Successful
publication releases transient force/payment holds while the derived future Truce
reservation persists. Later actual departure revalidates and pays once. Existing
travel, manifests, HP, passengers and prepaid credits survive War cancellation
under the existing cascades; no free return or refund is introduced.

War/Truce use pair-derived keys and consume no entity ID. Failed attempts consume
no successful autonomous allocation ordinal. Considering/denying a proposal never
rebuilds its identity or changes permitted trace ordering/hash. Successful creator
submission consumes accepted_sequence only after full prospective capacity,
preview, identity and digest checks; stale/rejected/cancelled private submissions
consume none. Accepted commands later abandoned/undone retain their durable
high-water consumption. Existing event/receipt identities follow actual fact order,
without a new reservation event or fake issuance frame.

## Validation, views, mutation coverage and ownership

Pure root validation reconstructs the tagged key union and checks256 and120,
strict unique keys, exact references, both objectives, clock ranges/checked future
Truce end and incompatible same-pair state. It does not silently repair an old
full256-plus-War V3 document, evict another treaty or import it with a missing lease.
This is a change to Proposed V3 validity, so pre-amendment draft vectors need an
explicit revision/compatibility decision before any codec exists. V1/V2 routing
must not use these rules. No new V2 rule number or conversion shim is proposed.

| Head-dependent surface | Required recomputation/fence |
| --- | --- |
| Genesis/new document and imported archive | Every reconstructed state/history view obeys the derived rule; validate exact catalog/domain, queue chain, references and replay before new-slot publication. No reservation certificate accepted from artifact input. |
| Running creator preview/accept | Recompute against projected accepted queue and exact reviewed effects; every ordered prospective prefix's derived union must fit. Envelope expected revision/tick/digest/pending sequence and accepted parent chain remain exact; phase1 applies that chain before material admissions. |
| Paused creator application | Recompute complete private candidate at selected tick, including reviewed conversion/cancellation; no clock advancement or silent lease release. |
| Autonomous negotiation, War admission and actual expiry | One central ledger/private boundary owns all provisional sets; enforce successful prefix capacity and atomic conversion. Failures preserve committed head/accepted queue. |
| Undo/redo/fork/checkout/history pruning | Derive from that exact replayed branch/view and coherent pending queue. Historical immutable states/receipts are never rewritten; retained allocator high-water marks do not shrink. |
| Save/export/native or browser resident installation/coordinator selection | Use the exact coherent validated state/history/queue; generation CAS for every head-dependent mutation and exact selected-view/catalog/slot binding. Publication validates capture/generation/cursor/allocator before moving owners. |
| Restart/reopen/readback/replay | Rebuild leases from authenticated War/queue inputs; no default-zero recovery, old selection fallback or persistent host certificate. |

Creator inspection may derive full actual/leased occupancy from the authorized
state and projected queue. Actor capture continues to contain only public Wars
and Agreements to which that actor is party and its authorized Commitments/offers.
Derive its own future Truce keys from those own-party War rows, preserving public
pair/end data; add no total War count, foreign pair roster, exact foreign treaty
occupancy or queued hidden creator input. Current private forecasting captures
own known capacity and conditional allocation under unknown competition. Unknown
foreign slots remain unknown. Under this Proposed contract, a valid
captured own-party War itself proves its reserved future Truce key: model its
natural expiry conversion without asking for a hidden global count. Checked
public end/timing and continued War ownership are still required. This is a
protocol capacity guarantee, not knowledge of foreign future actions; an unseen
early peace or creator cancellation is not predicted. A proven surviving own-party War converts in private forecasting at phase2 using its reserved agreement key, without hidden treaty occupancy. No pending-capacity fallback is permitted for that conversion.
Unproven early conversion/new treaty allocation retains bounded conditional
dependencies and never authenticates authority success. Actor trace
must remain equal for equal permitted inputs despite hidden capacity changes;
central admission/outer receipt may differ. Successful Truce is a real permitted
public record at the next capture; no hidden-root read during scoring authenticates
an earlier expectation.

Indexing actual treaty keys, at most120 War pairs and bounded queued effects,
prospective validation, fact emission, hashing and disposal all consume common
record visits. Valid public budgets1/2/17/1024 must yield identical final state/facts/reasons. Public0/>1024 rejects invalid_range without work; internal remaining0 continues.
Do not scan the whole queue/state in constructor/finish or release capacity owners
just because a poll returns Pending. Conversion moves existing owners into prepared
state/retirement without dropping unvisited objectives/peace/history buffers.
Cancellation retires provisional keys/force owners and partial indexes in bounded
work; publication moves a complete prepared result. This specifies required
ownership behavior, not a claim that the separate boundary implementation has
already implemented or proved V3 retirement.
## Diplomacy request, acceptance and peace record lifecycle

The diplomacy region uses ecology's exact P/R/B/O/H/K/M/F/A logical order,
immutable phase7 actor captures and SAME actual W/ledger. P-entry freezes old
pending Request IDs created<T in C-ID/R-ID order. P performs maintenance/objective
samples/Demand/fresh Request/sharing only; B considers surviving frozen old Requests
AFTER real category0R and before old OffersO. New Request/Offer atT never receives
same-T response. Lifecycle expiry never fabricates consideration of removed rows.
All actual actor-owned Decisions/facts retain ORIGINAL full phase7 reference while
execution/issuance phase remains8. Lifecycle immediate groups use registered actual
actor-null original prefix; no rewrap/current Receipt self-source.

### Free own Demand producer and exact decisions

Demand key=(actor,World,resource), cap1024; existing key refresh consumes no extra
slot. Only own positive known shortage creates units1..10000, tick=T,
expires=checked(T+200), snapshot_digest=original actor H7 permitted digest.
Order actor/World/resource tag. Food/Energy use captured population's next ordinary
ceil(pop/100)/ceil(pop/200) Need minus captured physical stock. Ore uses ONE earliest
(due tick,FacilityID) enabled staffed applicable own Foundry due<=T+100, with all
other actual guards satisfied except Ore deficit. Alloy analogously uses ONE Lab
with eligible selected target/Energy/staff and other guards except Alloy deficit.
No speculative construction, summed future production or hidden receiver stock
forms shortage. Creator-authored other genuine purpose rows remain legal and are
not silently deleted because this autonomous roster differs.

For every positive posting/refresh/omission emit free intent_decided:
phase8, actor producer, next actual actor/phase ordinal, action request_supply with
COMPLETE proposed Demand, candidate=null, targetSome(World), score0,tierordinary,
snapshot original H7 phase7, observed_tickSome(T), outcomeaccepted/rejected.
Accepted reasons[]; rejected exactly protocol{failure}. No material utility/lane/
CandidateAudit. Acceptance follows with demand_posted full old/new reasonnull;
new null→new and existing refresh old→new give2 events even if units unchanged.
Full1024 NEW key gives rejected protocol.capacity{demand_requests,1024,1024} then
adjacent request_capacity demand{before:null,after:null,capacity_unavailable}.
Adjacency/same producer/capture authenticates exact omitted key; lone null/null is
invalid. Current ownership/reference loss has ONLY rejected protocol.target_invalid
Decision and preserves existing row; no false capacity/end/no-op companion. Present
P has no owning after-H7 World mutation, so do not fabricate unreachable failure
histories. Genuine earlier loss uses its real removal before capture. Known resolved
own autonomous purpose removes via demand_expired/reasonnull, no zero active record
or posting Decision; target loss=target_invalid, TTL=expired. Arithmetic/source/cap
fault aborts complete unpublished boundary, never truncates omitted attempts.

### Older Offer response, formation and renewal

O-entry freezes current eligible old Offers created<T<expires, recipient-ID then
Offer-ID. Validate original authentic Terms/participants/H7 and actual current
W/ledger guards without rescoring or new target. Waiting cap/own_needs_reserve/
physical no_supply retains exact Offer and last_consideration{T,waiting,reason};
no_supply is the explicit exception to refusal. Other genuine eligibility failure
refuses/removes. Waiting emits offer_considered1; refusal considered then
 offer_refused with exact updated consideration2. New proposals have consideration
null and are excluded from this walk; score/identity and outgoing capture occupancy
remain immutable. Checked clocks/source/collision/event faults are not refusals.

Accepted new C has actual ID, sorted parties, exact original kind/Terms, createdT,
expireschecked(T+1200), statusready/attemptnull/fulfillmentnull/reasons[]/requestnull.
Access/Defense route=null; Supply route=actual new bound Trade Route, exact original
sender/receiver/endpoints/resource/batch/reserve/cadence10, suspendedfalse,
next_duechecked(T+10), commitmentC. No accepted cargo/payment/fulfilled witness.
This Trade-kind right authorizes only this original Supply route, not global Trade.
Transient positive batch/floor protects only supported next dispatch and ends at
publication, no1200-tick escrow. Original Access/Defense acceptance emits considered,
commitment_created, full accepted Offer removal3; Supply adds full Route insertion
before Offer removal4. Recipient H7 owns consideration/C/removal; original proposer
H7 owns Route insertion. Actual allocation uses flag1 phase8: Offer15 subject
Civilization5 recipient; C16 subject Offer15 original; Route12 subject World3 source;
Request17 subject World3 target, contiguous successful counters per actor/phase/kind.
No denied/renewed case consumes a success counter; prospective ID collision faults,
never increments until free.

Renewal preserves exact C/Route IDs/created/Terms/obligation, updates expiryT+1200
only, consumes no AUTO counter, dispatch/attempt, suspension reset or old witness.
Emit considered, commitment_renewed fullold/new, accepted Offer removal3. Different
Terms requires explicit old removal/new formation; paid old C cargo never fulfills
C2 by resemblance. Missing expired renewal source refusesexpired under genuine
historical reference, not implicit new treaty. Due C removal is actual phase2 or
immediate owning cause, never deferred to next100.

ObservedThreat names one exact known hostile actor at the requested owned world.
Its owner/escort/occupation facts come from the requester's visible observation,
not live rival state. DefenseRequest.threat must match a threat from that exact
requester/target observation and retains its originating observer, original tick
and snapshot digest. Optional war names an actually observed current pair with
requester and enemy; an enemy without such a pair can require normal declaration
admission before relief. Two enemies yield two distinct observation threat
records; each defense request chooses one enemy explicitly, never aggregates
ownerless hull counts as an actor identity. Counts are at most2048 escorts and
at most256 unique hostile fleet IDs globally, limited by observed state; threats
cap15 per world and sort by (enemy,target_world,observed_tick,originating_observer).
Removing/capturing the target or loss of its observed named threat terminates the
request with a fact; stale observations retain their original age. Shared
ObservationFacts include only these copied observed threats, never rival supply,
stockpile, fleet HP or newly changed enemies.

ObservedThreat.observation_snapshot_digest is a separate acyclic observation-fact
hash, not the actor's permitted snapshot digest or current State digest. Its exact
formula is SHA256(domain bytes `NYON_LIVING_V3_THREAT_OBSERVATION` followed by one
NUL || canonical payload). The payload's required field order is:
`{format_version:3,rules_version:3,catalog_hash:Digest,
originating_observer:ID,target_world:ID,observed_tick:Tick,enemy:ID,
war:Option<Pair>,hostile_escort_count:u32,occupation:bool,
hostile_fleets:Sorted<ID>}`. Values are copied from that ObservedThreat plus the
exact immutable catalog hash. Exclude observation_snapshot_digest itself, all
actor snapshot/State/receipt hashes, all surrounding observations/requests/shared
intelligence and future metadata. Derive the observed fields from the observer's
permitted visible facts at phase7 before inserting the new threat into observation
state; serialize/hash this ID-free payload once, then insert the computed digest.
The label snapshot denotes this immutable fact capture only. Do not substitute
any digest of a structure containing the field being derived.

During replay, independently reproduce the original observer/target/tick facts
from the authorized observation phase and recompute the exact payload digest.
Request creation and sharing copy this original record and digest byte-for-byte;
received_tick and recipient are surrounding transport fields and cannot refresh
it. A fork retains inherited records/digests, including their original observer
and tick; a genuinely new post-fork observation derives its own fact payload.
Identical observed facts at the same original observer/world/tick/catalog may
have the same digest across forks; branch identity is bound by the enclosing
history/Anchor, not forged into this fact hash. Wrong catalog, altered enemy/
counts/fleet IDs, changed timestamp or using an enclosing permitted snapshot hash
rejects replay validation.


### Defense creation, sharing and old-request response

Fresh Request requires live usable original mutual_defense C, original party
requester/direct-visible own target and exact original requester H7 Observation/
Threat at observedT. Threat enemy/World/observer/tick/digest/optional requester-enemy
War/counts/Fleet IDs are copied byte-exact; no shared stale input refreshes it.
Choose C-ID/requester-ID/World-ID/full threat key, one first eligible target/threat
per requester/C. Reciprocal lower requester wins one outstanding C pointer.

Prepare ID-free DefenseRequestInput before actual issued R. First causes: current
C/party/target/ownership/interval; actual same-pair unusable War rights; authentic
original target/threat; outstanding C Request; Request256 cap; prospective new
share4096 key cap. Integrity/TTL/collision/output fault aborts, never denial.
Target/reference failure refuses target_invalid; active pair War refuseswar_incompatible;
expired C refusesexpired only where its genuine input still exists; outstanding
R waitsprior_obligation (actual reciprocal loser), caps wait exact ProtocolCollection.
Denied input has installed_requestnull and one defense_request_attempted only;
no ID/success ordinal/C attempt/partial share is allocated.

Accepted R is genuine phase8 flag1 Request17/targetWorld3/requester original H7
AUTO success; full fields exact input plus id/dispositionpending. C.requestSome(R),
statuswaiting/reasons[], preserve last_attempt and prior fulfillment. Atomic group:
attemptedaccepted/nullreason/installedSome; defense_requested full R; changed C
obligation_changed; intelligence_shared iff actual transport mutation (3 or4).
Share ONLY exact target facts to key(defender,original observer,target), receivedT;
original observation tick stays unchanged. New key consumes slot; newer original
payload replaces; identical equal-tick or older retains stored received_tick and
emits0. Equal observer/World/tick inconsistent payload is source-integrity fault,
not last-write-wins. No transitive sharing or whole galaxy/rival private facts.

At B consider ONLY surviving P-entry old pending IDs C-ID/R-ID against actual
post-R W/ledger and defender original H7. True lifecycle invalidity ends Request
through owning cause, no nonexistent-row consideration. Then first response cause:
missing/stale original action intelligence waits its explicit tag; strict own
category0 recovery or genuine admitted contrary structural/account contributor
waitsprior_obligation. Build finite ORIGINAL whole own Fleet alternatives, current
physical/reference/Supply/path/needs/structural guards, separate LawfulNow from
DeclarationOnly lacking ONLY War against exact named enemy. Lawful local nonoffensive
Defend requires no invented offense. Unknown nonpermission inputs wait, never empty
Fleet proof. LawfulNow feasible accepts. Only DeclarationOnly searches original
actually eligible still-unattempted reachable War CandidateTrace against exact enemy
with its original force and current queue/guards. Rejected R identity or filled Fleet
lane cannot retry. Genuine candidate dependency waitsdeclaration_required{enemy,
war_candidate_identity_bytes}; otherwise known no feasible action refusesno_eligible_fleet.
No post-capture new plan, reserve, order, payment, declaration or fulfillment follows.

Contrary-holder comparison uses same original alternatives/current physical inputs:
F_actual versus removal ONLY of earlier genuinely contrary resource/floor/structural
entries. Never remove own strict recovery, real prior debit, current canonical order
or permission failure. If a different lawful Fleet remains, accept; one blocked
candidate is no veto. Exact private witness selector is lowest(holder targetID,
holder actorID,holder identitybytes) among causal participants, reproducible from
P/R/B, no forged pre-R winners. Persistent installed-action conflict has its own
exact predicate, not erased by a counterfactual ledger. No accepted R owns a Fleet.

Every actual consideration emits defense_request_considered with exact old/new
RequestDisposition,T,outcome/reason, no Decision; then full R Change iff disposition
changes, then full C fact iff byteschange. C.last_attempt=T once; accepted→waiting/[];
wait→waiting/[exact mapped cause]; refused→waiting/[recipient_refused], Rrefused.
Missing/stale map exact new ObligationReason; declaration_required unit is backed
by full private candidate dependency fact. Prior fulfillment is unchanged. Waiting
R unchanged has no fake Change. Acceptance does not count contribution. Full
candidate-bearing consideration is defender/authority/replay only, not requester
canonical export; safe requester notice omits identity/force/account/source hash.

### Continuing C rights and immediate cause groups

Acceptance relation/Demand/connection thresholds are NOT automatic revocation.
C original ID/created/Terms/parties remain authentic through same-Terms expiry-only
renewal; EditCommitment may not change Terms. New Terms requires actual remove/new
instance. Live C has created<=T<expires and required actual references. Transit/
Defense usability excludes active same-pair War only; third-party War does not
revoke treaty. Existing otherwise usable C is compatible with Truce, which grants
no new rights and still blocks new Access/Defense acceptance/offense.
Supply additionally needs exact original Worlds/Route/C/parties, current matching
endpoint owners and Route.suspendedfalse. Original Demand/relations/current stock/
connection changes alone never revoke fixed rights; actual dispatch still guards.

Freeze retained C-ID roster at each real cause. First causes: missing participant
end; expiry end; genuine Supply World/Route removal end; source owner mismatch,
receiver mismatch, sticky Route suspension→suspended/[endpoint_changed]; active
same-pair War→suspended/[war_incompatible]; otherwise preserve nonsuspended bytes.
War-only suspended recoversready/[] if no stronger cause, preserving attempt/prior
pointer; never restores guessed old status. Endpoint latch clears ONLY legal actual
SetRouteSuspended true→false with exact original binding/current owners/no active
War: direct Route Change then dependent Cready/[], preserving all other fields.
Already-false repeat does not clear planted endpoint reason. EditRoute/owner equality/
War end/renewal is no substitute. Binding corruption is fault, not silent end repair.

Defense target capture/removal, actual C/party removal/expiry, same-pair War or later
authenticated absence of SAME named threat ends R once. Stale/unknown absence or
another enemy does not prove cessation. End primary, ascending exact originally
bound Fleet full order-null Changes in one operation_changed iffchanged, then one
coalesced C request/status fact. Preserve paid location/Hulls/Supply/credit/passengers,
last_attempt and prior fulfillment. Request-only waiting reasons resetready/[] only
if selected continuing rights valid; fulfilled persists absent real suspension.
C end emits commitment_ended, bound Route removal iffpresent, remaining R end and
bound Fleet cleanup iffpresent; paid Shipments retain historical dispatch linkage.
No dangling R/C/Fleet binding publishes. Full C-ID roster max256, Fleet<=256,
no unrelated treaty/endpoint resemblance or new replacement revisit.

Place actual cause adapters after primary World capture/status/workforce before
next World, after War end/Truce pair or current War installation/start before later
region work, and inside exact creator aggregate effects for owner/reference edits.
Autonomous group source is its full original per-C actor-null prefix/T/actual phase,
latest applied creator cursor. Only registered cause→adapter interval may temporarily
contain stale derived eligibility; never completed import. Changed before/after
emits1 C fact; byte-equal emits0/ordinal0. Bare scan never updates last_attempt or
fulfillment. Creator group emits aggregate direct/indirect Changes, never duplicate
autonomous obligation/end facts. Capture/War/removal are immediate at off100 too;
due R then C ends are phase2 every advancing boundary, not delayed protocol100.

Accepted bound Defend's explicit DefenseActionBinding is original R ID/created,
C ID/created, Threat digest; only Defend may nonnull. Current validates live accepted
R, exact parties/defenderFleet owner, targetrequester-owner, C.request exact, active
usable original duty and independent original intelligence/path/Supply. Multiple
original applicable Requests create separate ascending-ID binding alternatives,
no new score or post-B insertion; a pending H7 R can be preformed then genuinely
accepted atB. Refused H7 yieldsnone. Request accepted alone has no force escrow.
Installed genuine bound Defend is protected against autonomous contrary replacement/
Board/split/merge/roster destruction until owning end; other Fleet/unrelated War is
not alliance veto. Creator transformations explicitly cancel old bound action before
new roster, never clone/transfer contribution. Actual incompatible current bound
order fails earlier reference/order guard; transient actual holder fails stage9.
Terminal outcome/Request end preserves original action history, never C2 credit.

Every successful paid dispatch freezes its exact route, optional CommitmentBinding,
withdrawn resource/units and pre-dispatch SnapshotReference in ManifestProvenance.
terms_digest is SHA256(NYON_LIVING_V3_TREATY_TERMS followed by one NUL, then canonical
Terms bytes); the domain is distinct from identity domains. All immutable shipment
fields and paid provenance must match the authenticated dispatch replay.
Shipment delivery fulfills only the same original commitment ID/created_tick/terms
when still active and the actual recipient equals the original intended party.
Same-terms renewal retains that binding and may fulfill; expiry/removal/capture or
a replacement C2 records the original paid delivery but gives no C2 fulfillment.
Creator provenance or a nonnull creator_disposition_override gives no negotiated
fulfillment credit. A removed route/commitment reference resolves at dispatch
snapshot tick via bounded replay, never via current endpoint/resource matching.
Creator changes cannot rewrite manifests.

Supply fulfills only on actual delivery, never dispatch. Its latest authenticated
delivery Anchor survives shipment removal. Waiting/suspended statuses carry exact
attempt/reasons; Access is ready while valid; Defense has pending/accepted/refused
request provenance and fulfilled only after an actual recorded defense action.
Capture/war/removal reevaluates eligibility immediately and suspends or invalidates
records through explicit facts. Prior manifests and payments never change.
Renewal creates a new ordinary offer linked by renewal_of; it follows the same
delay, eligibility and admission rules. Acceptance renews the existing exact
Commitment ID, advances its expiry to acceptance_tick+1200 and replaces the offer;
terms must be identical, otherwise it is a new commitment. Old receipts remain
immutable. Overlapping same-party/same-kind commitments with identical terms
reject; incompatible terms require explicit expiry/removal before replacement.

### Exact Secure endpoint sampling and objective witnesses

Enumerate each public Lane's actual endpoint World Cartesian product in World-ID
order, then eligible whole own Fleets and each selected pair World as arrival.
With512 total Worlds and distinct endpoint Systems the product is at most65,536;
stream candidates/hash/best/head with shared-budget rescans, no retained product,
top-N cutoff or guessed System owner. Secure local tie inserts world_a/world_b
following objective target/peer and before arrival/Fleet/Hull bytes. Target stays
Lane. Each initial objective is pending/count0/last_evaluated null/witness null.

At a100-boundary evaluate pending objectives under original H7 capture in War-pair
then actor-ID order. Secure first checks its fixed live Lane/pair topology; missing
or relocated endpoint/broken Lane System membership invalidates/count0. Distance
alone does not invalidate. Then owned success requires BOTH chosen Worlds owned
by actor. Negotiated success requires EACH endpoint either own or foreign-owned
with its exact active usable actor/owner Transit C, with at least one actual
foreign Transit. Distinct foreign owners can require two contracts. Null owner
cannot satisfy negotiated success. Actual ordinary destination permissions must
pass for both endpoints; no unrelated C, Trade, pending Offer or new same-T Transit
creates a sample. Foreign ownership must be authorized observed at this T; own
full records are exact. Stale/unknown ownership or permission resets pending count0,
last_evaluated=T; no hidden-root absence or age300 offensive waiver applies here.

A complete positive sample increments iff previous sample was T−100 under the
SAME objective instance, else begins1; cap3 succeeds. Complete negative or unknown
resets0. Every visited pending evaluation records T and one objective_changed iff
bytes change. Terminal succeeded/invalid latches without later reevaluation/fact.
Creator deletion/World relocation/Lane-endpoint edit invalidates each affected
pending row before aggregate validation, retaining original IDs/pair and prior
pointer; terminal rows remain byte-identical. Repeated invalidation emits no phantom
change. Exact containing War before/after appears once in creator direct/indirect
effects; no substituted endpoint, autonomous success or fabricated observation.

Objective instance is original War installation plus its genuine last replacement
operation in retained ancestry, not equal bytes. Only actual terminal
objective_changed or creator-dependent pending→invalid aggregate War Change is
an outcome-witness producer. Pointer-only installation and pending counter samples
are not producers. Producing State retains prior Anchor P; next advancing boundary
phase2 handoff installs the original sealed terminal producer Anchor once for the
same surviving instance. Creator invalidation keeps its actual intervention/cascade
EventReference kind. Phase1 captures prior P before handoff; paused creator/save/
reopen/fork never consumes a handoff or installs its own receipt Anchor. Child uses
only genuine inherited ancestry; parent future is excluded. Removed/replaced
objective skips installation, never transfers to identical replacement.

Defense fulfilled-action pointers use the same next-ordinary phase2 cadence with
the genuine exact C/R/action producer. Same live C may consume after R legitimately
ends, preserving current status/reasons/request/attempt; removed/expired/replaced C
skips. A real action at T followed by later capture/War retains its historical
eligibility, not final-owner hindsight. Supply pointer handoff remains phase3
block0. No handoff repeats evaluation/payment/contribution or produces another
handoff-chain event. Pending source/copy/replay and cleanup share remaining visits;
mandatory pointer capacity fault aborts the complete unpublished boundary.

Every installed war records both objectives, attached atomically. An autonomous
declarer enumerates only its own objective/force and peer under its own immutable
post-phase7 capture; the defender objective is excluded from identity/score/order.
Central attempted admission first holds the declarer's force/account, then selects
the defender objective under its own same-tick phase7 capture using scoring §5.
No valid defender objective rejects the unchanged attempt with
no_valid_defender_objective and releases holds. A valid objective with null force
remains installable with force_unachievable audit; no unseen target is invented.
WarAdmissionAudit binds those captures/unchanged identity/selection. Only successful
installation discloses the public defender objective; its force/account audit remains
defender-private. There is no intermediate one-objective War or recaptured actor
queue. This replaces the earlier actor-declaration two-objective requirement.
Creator ForceWar explicitly supplies both exact pending constructors. Actual
objective completion/invalidation records its complete outcome while retaining
prior witness; its sealed producer Anchor is installed only at next advancing
ordinary phase2. K may propose from that authentic terminal same-instance result
without a self-Receipt pointer. The other participant considers only an older
proposal at the next actual100 diplomacy boundary using original permitted
observations and supported true/false/unknown achievability. Refusal retains its
full reason/considered_tick; retry requires the exact finite semantic-baseline
comparison above, never time/pointer-only changes.
Acceptance/expiry converts the exact reserved treaty key under Derived future
Truce capacity, removes War and emits its autonomous end fact before the300-tick
Truce. Expiry occurs at phase2 end-exclusive deadline without waiting for another
100-tick boundary. Simultaneous successful proposals select lower proposer ID for
one Peace record. All these transport/state choices are Proposed, not V2 changes.

## Events, decisions and authenticated receipt schemas

Phase registry (u16): creator1; lifecycle_ecology2; arrivals3; conflict4;
production_repair_research5; needs_demographics_workforce6; observations7;
diplomacy8; intents9; dispatch10; validation11; sealing12. Event ordinal is
boundary-global u32, contiguous0..65535 across phases, including paused creator
publication. Same-tick paused publications have distinct applied revision/state
payloads and therefore distinct receipt digests.

Provenance tagged union: creator `{revision:RevisionID}`; autonomous
`{phase:u16, actor:Option<ID>, snapshot:SnapshotReference}`. A lifecycle event
without an actor uses the captured boundary input snapshot, not omniscient facts
masquerading as actor observations. SnapshotReference fields:
`domain:u32, catalog_hash:Digest, branch:BranchID, revision:Option<RevisionID>,
tick:Tick, state_digest:Digest, phase:u16, permitted_snapshot_digest:Digest`.
For actor proposals permitted_snapshot_digest hashes `NYON_LIVING_V3_OBSERVATION\0`
plus canonical `{actor:ID, own_worlds:Sorted<World>, own_colonies:Sorted<Colony>,
own_facilities:Sorted<Facility>, own_deposits:Sorted<Deposit>, own_fleets:Sorted<Fleet>,
actor_policy:Slug, actor_status:LivingCivilizationStatusV3, research:Research,
own_jobs:Sorted<ConstructionJob>, own_hull_jobs:Sorted<HullJob>,
own_routes:Sorted<Route>, own_shipments:Sorted<Shipment>,
incoming_shipments:Sorted<Shipment>, agreements:Sorted<Agreement>, wars:Sorted<War>,
offers:Sorted<Offer>, known_hazards:Sorted<KnownLaneHazard>,
known_stellar_events:Sorted<KnownEnvironmentEvent>,
known_hazard_scopes:Sorted<HazardScopeObservation>,
known_stellar_scopes:Sorted<StellarScopeObservation>,
known_systems:Sorted<System>, known_world_geometry:Sorted<KnownWorldGeometry>,
known_lanes:Sorted<Lane>, own_settlement_claims:Sorted<SettlementClaim>,
own_occupations:Sorted<Occupation>,
relations:Sorted<Relation>, commitments:Sorted<Commitment>,
observations:Sorted<Observation>, demands:Sorted<DemandRequest>,
defense_requests:Sorted<DefenseRequest>, shared_intelligence:Sorted<SharedIntelligence>}`.
incoming_shipments contains only immutable actual manifests addressed to a world
owned by the actor at capture or whose recorded intended receiver at paid dispatch
was that actor; deduplicate by ID within each sorted array. The same own/incoming
manifest may appear once in each role but forecast delivery counts its ID once.
No unmanifested promise or rival unrelated shipment appears. Agreements/wars include
only public records to which the actor is party; offers include only its own sent
or addressed received offers. War includes public objectives but no private audits.
Selection is exactly the closed Actor32 source registry below; creator/lifecycle
uses state_digest as permitted_snapshot_digest. Observations include stale timestamps.

### Actor32 source registry (Proposed)

`KnownWorldGeometry {id:ID, system:ID}` has this exact field order, typed World/System references, strict id key and cap512. It is a projection leaf only: no State/Record/CollectionKind/entity tag is added. The OBSERVATION object above has exactly32 required fields. New array keys/caps are own_deposits id/1024, known_world_geometry id/512, own_settlement_claims (world,civilization)/8192 and own_occupations world/512. All are explicit empty arrays when empty; no default fills them.

All selection uses the immutable post-phase7 authority at tick T and actor A. Selected rows copy full canonical record bytes except KnownWorldGeometry. Selection is independent of later action eligibility. No expiry-based source eviction occurs while hashing; authority lifecycle removes rows. The visibility and source selection are Proposed choices, not implicit inheritance from generic public-topology wording.

| Fields | Exact source selection and cap/key |
| --- | --- |
| actor, actor_policy, actor_status, research | A's exact Civilization fields; one actor, not rival state |
| own_worlds | World.owner=A; id,512 |
| own_colonies | Colony.owner=A with normal World/owner consistency; world,512 |
| own_facilities / own_deposits | Current hosting World.owner=A; id,2048/1024 |
| own_fleets | Fleet.owner=A; id,256; full own hulls/paid movement/supply/passengers, underlying hull limits2048 total/16 per fleet |
| own_jobs / own_hull_jobs | Recorded owner=A; id,4096/512; do not replace with current world ownership |
| own_routes | Recorded source_owner=A; id,2048; destination ownership alone does not disclose a route |
| own_shipments | Recorded dispatch_owner=A; id,4096 |
| incoming_shipments | Recorded intended_receiver=A OR current destination World.owner=A; id,4096; exact immutable manifest selection |
| agreements / wars | participant_a=A OR participant_b=A; (participant_a,participant_b,kind)/(participant_a,participant_b); typed treaty union cap256, War120 |
| offers | proposer=A OR recipient=A; id,64; root uniqueness/one active proposer limit preserved |
| commitments | parties.participant_a=A OR parties.participant_b=A; id; typed treaty union cap256 |
| known_systems / known_lanes | All current root Systems/Lanes, full existing records; id,64/256 |
| known_world_geometry | All current Worlds projected id/system; id,512 |
| own_settlement_claims / own_occupations | civilization=A / claimant=A regardless direct visibility; full exact records, (world,civilization)/world,8192/512; no rival claim rank/progress |
| relations | from=A OR to=A; full directed rows; (from,to),240; both directions, no unrelated third-party rows |
| observations | All retained civilization=A rows, including stale; existing (civilization,world) key,8192; no newest-world merge before hashing |
| shared_intelligence | All retained recipient=A rows; (recipient,originating_actor,world),4096; originating_actor=A alone grants no extra transport row |
| defense_requests | All retained requester=A OR defender=A rows, including recorded dispositions; id,256/one per commitment; no transitive-alliance selection |
| demands | actor=A OR original world in A's current direct-visible roster OR a direct bilateral Commitment between A and request.actor with created_tick<=T<expires_tick; existing (actor,world,resource),1024; precise interval is a Proposed clarification |
| known_hazard_scopes / known_stellar_scopes | I1 selection from ALL permitted retained observations/shared/request facts, before world projection; lane/system,256/64 |
| known_hazards / known_stellar_events | I1 winning-scope children, event-ID newest winner including expired, then expiry filter then cap; id,128 each |

Snapshot keys use the declared underlying record/composite keys.
Per-role manifest dedup permits the same ID once in own and once in incoming;
forecast union counts it once. Addressed manifests are immutable paid dispatch
facts, not unmanifested contract promises. Ownership changes alter capture
selection without rewriting dispatch_owner/intended_receiver/source provenance.
Retained records are selected even when their own action eligibility has expired;
the authority lifecycle, not snapshot projection, removes requests. Demand refresh
and capacity facts follow Diplomacy request, acceptance and peace record lifecycle. All inherited reference/history checks
remain required before calling a source row admitted.

Define DirectVisible(A,w) from validated current base records as: w.owner=A OR an own Colony's world is in w.system OR any own Docked Fleet is at w OR an own Docked Fleet in w.system contains ANY hull whose catalog HullDefinition.role=scout. Mixed fleets qualify; travelling adds no sight. Submitted observations never authorize themselves. Public topology is all full Systems/Lanes plus World id/system only; System.name is explicitly public, World names/archetypes/dynamic data are not globally disclosed. Exact owned deposits include owned non-colonized worlds. Own claim/occupation bookkeeping is selected regardless sight, without rival rank/progress.

All permitted retained Observation/shared/request original scope facts feed I1 before world projection. Latest scope selection, negative coverage, latest child INCLUDING expired then chosen expiry then each-family128 fault follow I1 unchanged. Original source geometry/tick/observer authenticate at the source boundary; current World moves, received_tick or fleet traversal never refresh it. ActorInputContext/outer SnapshotReference shapes and domain bytes remain unchanged, but the32-field permitted hash and dependent traces/AUTO frames require fresh derivation.

### Closed actual dispatch, protocol and combat carriers

The following are required full V3 declarations. All options are explicit;
records use displayed field order and tagged unions type-first. No aliases,
free-text reason, fake paid ID or omitted field is admitted.

```text
MovementPermissionFailure = settlement_target_owned { world:ID, owner:ID }
 | defense_target_unowned { world:ID }
 | defense_request_required { world:ID, owner:ID }
 | binding_target_owner_changed { world:ID, expected_owner:ID, actual_owner:Option<ID> }
 | offensive_target_not_enemy { world:ID, actual_owner:Option<ID> }
 | truce_active { war_a:ID, war_b:ID, end_tick:Tick }
 | offensive_war_missing { world:ID, owner:ID }
 | no_transit_right { world:ID, owner:ID }
 | defense_duty_unavailable { request:ID, commitment:ID, cause:DefenseDutyCause }
DefenseDutyCause = request_not_accepted | request_interval_ended |
 commitment_interval_ended | commitment_suspended | request_pointer_changed |
 requester_defender_war_active
FleetDispatchBlock { fleet:ID, owner:ID, origin:ID, target:ID, order:FleetOrder,
 attempted_tick:Tick, reason:FleetDispatchBlockReason }
FleetDispatchBlockReason = return_target_not_owned { current_owner:Option<ID> }
 | recovery_not_eligible { predicate:RecoveryPredicate }
 | launch_account_unavailable { current_owner:Option<ID> }
 | no_route | permission_denied { predicate:MovementPermissionFailure }
 | supply_unavailable { readiness:SupplyReadiness }
 | hull_delivery_reserved { job:ID }
 | declaration_force_reserved { actor:ID, candidate_identity_bytes:String }
 | return_credit_missing { hull:ID }
 | physical_energy_unavailable { world:ID, required:u32, available:u32 }
RecoveryPredicate = actor_not_dormant | target_not_unowned | ark_absent | target_unknown
DispatchReservation { world:ID, resource:Resource, physical_stock:u32, floor:u32,
 protected_floor:u32, required:u32, available:u32, self_held:u32, self_floor:u32,
 holders:Vec<DispatchHolder> }
DispatchHolder { actor:ID, origin:HoldOrigin, consumer:HoldConsumer,
 quantity:u32, protected_source_floor:u32 }
HoldOrigin = candidate { phase:u16, lane:u32, identity_bytes:String }
 | older_offer { offer:ID }
HoldConsumer = route { id:ID } | fleet { id:ID }
ProtocolCollection = demand_requests | defense_requests | shared_intelligence
ProtocolFailure = target_invalid { world:ID }
 | capacity { collection:ProtocolCollection, limit:u32, occupied:u32 }
DefenseRequestInput { commitment:ID, requester:ID, defender:ID, target_world:ID,
 created_tick:Tick, expires_tick:Tick, originating_observer:ID, observed_tick:Tick,
 observation:ObservationFacts, threat:ObservedThreat }
DefenseRequestAttempt { input:DefenseRequestInput, outcome:DefenseProtocolOutcome,
 reason:Option<DefenseProtocolReason>, installed_request:Option<ID> }
DefenseProtocolOutcome = accepted | refused | waiting
DefenseProtocolReason = target_invalid | war_incompatible | expired |
 prior_obligation | no_eligible_fleet | recipient_refused | missing_intelligence |
 stale_intelligence |
 declaration_required { enemy:ID, war_candidate_identity_bytes:String } |
 capacity { collection:ProtocolCollection, limit:u32, occupied:u32 }
RetreatOutcome = departed | blocked
RetreatReason = hull_delivery_reserved { job:ID } |
 missing_return_credit { hull:ID } | no_owned_colony | no_route
CombatAction { attacker:ID, attacker_fleet:Option<ID>, attacker_owner:ID,
 target:ID, target_fleet:Option<ID>, target_owner:ID, war:Pair,
 nominal_damage:u32, realized_damage:u32,
 defense_request:Option<DefenseActionBinding> }
DefenseShot { attacker:ID, target:ID, nominal_damage:u32, realized_damage:u32,
 readiness:SupplyReadiness }
DefenseCombatAction { binding:DefenseActionBinding, requester:ID, defender:ID,
 world:ID, enemy:ID, shots:Sorted<DefenseShot>, total_realized_damage:u32,
 credit_realized_damage:u32, outcome:DefenseCombatOutcome }
DefenseCombatOutcome = contributed | no_realized_damage | readiness_denied
```

Movement purpose ownership precedes topology, then treaty/duty; active Truce
precedes missing active War. Binding instance/type mismatch is earlier reference/
order failure, not a substitute permission enum. Cause order follows the listed
DefenseDutyCause predicates; no ended/dangling binding survives completed State.
Retreat failure order is paid HullJob, missing Hull credit, no owned Colony, no route.
FleetDispatchBlock copies its actual complete order and owns no new Decision,
ResourceFact, travel clock or AUTO ordinal. Its physical failure requires D>0 and
available<D; every Hull/job/declaration witness resolves to this exact real Fleet.

Dispatch holders are EVERY other active positive account entry in original causal
admission order, not actor/ID sort or winner-only truncation. Quantity>0;
unique consumer/account. Global active-entry bound2240 (Route2048 + at most
16*3*4 accepted lane/account entries192); one Route resource claim includes its
older-Offer origin, not an extra64. Floors MAX-coalesce, cargo quantities add.
Exact witness recomputes H/B/self/other and H+B<=physical_stock, never redefines
physical P. Full bytes are authority/creator/replay only. Actor filtering removes
whole private carriers. A separate noncanonical own cause notice requires
authentic producer-prefix Fleet.owner or Route.source_owner equal to that selected
actor (receiver/current ownership grants nothing), and the
notice Event in the selected HistoryTip eligible ancestry (parent only through
fork); present ownership is insufficient. Missing/unsupported locator, wrong catalog/
branch, missing replay provenance or unauthenticated subject yields no notice,
never owner fallback. Original owner may retain notice after capture/removal;
later owner receives none for the earlier producer. Notice contains ONLY subject
ID and constant “Dispatch was constrained by reservations.” Coalesce paired
Route/blockade carriers by authentic private actual operation identity, not equal
tick; never expose that key. It exposes no holder count/identities, quantities,
account location, source hashes, rival stock or candidate identities and cannot
be copied into Actor32/scoring or portable canonical history.
No redacted canonical Event is exported. Holder traversal/byte copies count under remaining visits and receipt caps.

Defense attempt input is ID-free and denied installed_request=null consumes no
Request AUTO success ordinal. Accepted input/ID must match the actual full new row;
request expiry/observation/threat/source are authentic original values. Consideration
has no fresh Decision or material lane; its declaration_required identity is a
DEFENDER-only complete canonical carrier because it contains private ForceWitness.
Requester may see only a separately derived known-enemy willingness/cause notice.

Facts.combat with its complete CombatAction array and Facts.defense_action
with complete DefenseCombatAction/shots are whole-carrier creator/replay
authority-only. Actor panels receive neither a selectively redacted array nor
raw target allocation, rival HP, actual private Supply proof or source metadata.
A separate noncanonical own-duty semantic notice may derive from authenticated
original action ownership and permitted original C/R history at selected history
tip/eligible ancestry. It shows only original own Request ID, target/named enemy
already authorized by that Request, and contributed / attempted without
creditable damage / attempted without current Supply. The last explains only
original defender readiness. No shot count, target Hull IDs/HP, rival accounts,
source/action/Receipt digests, private force/audit or unauthorized history is
exposed. Original ownership governs after capture/removal; later owner gains no
earlier notice. Requester receives only its legitimate public C result, not
defender action ledger. Missing proof fails closed. This is derived presentation,
never a redacted canonical carrier or planning/Trace input.

Combat actions sort by attacker ID, bounded4096. Damage allocation visits target
ID then attacker ID and assigns min(nominal,remaining pre-round HP), checked; sum
realized equals actual HP loss. All pre-round shots fire even if attacker dies.
Battery damage contributes ordinary allocation but no Request credit. Defense
shots sort(attacker,target), max2048 Hull shots/group; groups<=256. Current supplied
positive matching named-enemy Escort damage alone contributes credit; no docking,
order, presence, creator damage or overkill nominal count qualifies. Association
may remain recorded on nonqualifying shots, never mislabels them contributed.
Readiness is genuine pre-round current T<deadline; later phase5 renewal cannot
retroactively grant credit. Outcomes: contributed iff credit>0; otherwise
no_realized_damage if any supplied matching shot, else readiness_denied.

EventKind contains every frozen LivingEventKindV3 variant plus:
needs_paid, needs_shortfall, population_changed, workforce_changed,
passengers_boarded, passengers_unloaded, passengers_lost, research_progress,
technology_completed, technology_revoked, research_target_changed,
environment_updated, stellar_warning, stellar_started, stellar_ended,
environment_capacity_omission, demand_posted, demand_expired, request_capacity,
offer_created, offer_considered, offer_refused, offer_expired, commitment_created,
commitment_renewed, commitment_ended, obligation_changed, defense_requested,
defense_request_ended, intelligence_shared, objective_changed, peace_proposed,
peace_refused, fleet_supplied, fleet_supply_unavailable, blockade_restriction,
intent_decided, operation_changed, route_attempted, creator_cascade, candidate_audit,
defender_objective_audit, war_admission_audit, needs_reserved, declaration_hold,
retreat_blocked, fleet_dispatch_blocked, dispatch_reservation_blocked,
defense_request_attempted, defense_request_considered, defense_action_recorded.
This list is closed. No arbitrary text reason replaces a typed fact.

EventPayload fields: `phase:u16, ordinal:u32, provenance:Provenance,
kind:EventKind, facts:Facts`. Facts is the tagged union below. Allowed kinds:

| Facts tag and exact fields | Allowed EventKinds |
| --- | --- |
| change { changes:Vec<Change> } | creator_intervention,creator_cascade,civilization_dormant,civilization_reactivated,colony_captured,construction_cancelled,construction_completed,hull_completed,hull_destroyed,refund_discarded,settlement_claim_opened,settlement_claim_resolved,agreement_expired,agreement_formed,war_declared,war_ended,operation_changed |
| combat { world:ID, before:Vec<Record>, after:Vec<Record>, damages:Sorted<Damage>, actions:Sorted<CombatAction> } | combat_round |
| operation { world:ID, facility:Option<ID>, recipe:RecipeFacts, resources:Sorted<ResourceFact> } | facility_blocked,facility_produced |
| fleet_motion { fleet:ID, origin:ID, destination:ID, departure_tick:Tick, arrival_tick:Tick, resources:Sorted<ResourceFact> } | fleet_arrived,fleet_departed |
| shipment { manifest:Shipment, recipient:Option<ID>, resources:Sorted<ResourceFact> } | shipment_captured,shipment_delivered,shipment_dispatched,shipment_returned,shipment_waiting |
| needs { world:ID, result:Needs } | needs_paid,needs_shortfall,population_changed |
| workforce { world:ID, before:Sorted<ID>, after:Sorted<ID> } | workforce_changed |
| passengers { hull:ID, world:Option<ID>, owner:ID, before:u32, after:u32, population_before:Option<u32>, population_after:Option<u32> } | passengers_boarded,passengers_unloaded,passengers_lost |
| research { actor:ID, technology:Slug, before:Research, after:Research, lab:Option<ID>, resources:Sorted<ResourceFact> } | research_progress,technology_completed,technology_revoked,research_target_changed |
| environment { world:ID, before:Environment, after:Environment } | environment_updated |
| hazard { record:Hazard } | hazard_started,hazard_ended |
| stellar { record:StellarEvent } | stellar_warning,stellar_started,stellar_ended |
| omission { star:ID, due_tick:Tick, proposed_kind:Slug, capacity:u32 } | environment_capacity_omission |
| demand { before:Option<DemandRequest>, after:Option<DemandRequest>, reason:Option<ObligationReason> } | demand_posted,demand_expired,request_capacity |
| offer { record:Offer, reason:Option<ObligationReason> } | offer_created,offer_refused,offer_expired |
| offer_consideration { offer:ID, before:Option<OfferConsideration>, after:OfferConsideration } | offer_considered |
| commitment { before:Option<Commitment>, after:Option<Commitment> } | commitment_created,commitment_renewed,commitment_ended,obligation_changed |
| defense { record:DefenseRequest, reason:Option<ObligationReason> } | defense_requested,defense_request_ended |
| intelligence { record:SharedIntelligence } | intelligence_shared |
| objective { war_a:ID, war_b:ID, before:WarObjective, after:WarObjective } | objective_changed |
| peace { war_a:ID, war_b:ID, record:Peace } | peace_proposed,peace_refused |
| supply { fleet:ID, before:Supply, after:Supply, resources:Sorted<ResourceFact> } | fleet_supplied,fleet_supply_unavailable |
| dispatch { route:ID, commitment:Option<ID>, eligible_batch:u32, restricted_batch:u32, actual_dispatched:u32, blockers:Sorted<ID>, modifiers:Vec<Modifier>, reason:Option<ObligationReason>, reservation:Option<DispatchReservation> } | blockade_restriction,route_attempted |
| decision { record:Decision } | intent_decided |
| candidate_audit { record:CandidateAudit } | candidate_audit |
| defender_objective_audit { record:DefenderObjectiveAudit } | defender_objective_audit |
| war_admission_audit { record:WarAdmissionAudit } | war_admission_audit |
| needs_guard { operation:NeedsGuardOperation, reason:NeedsReserved, outcome:NeedsGuardOutcome } | needs_reserved |
| declaration_hold { record:DeclarationHold } | declaration_hold |
| retreat { fleet:ID, world:ID, before:Fleet, after:Fleet, destination:Option<ID>, outcome:RetreatOutcome, reason:Option<RetreatReason> } | fleet_departed,retreat_blocked |
| fleet_dispatch_block { record:FleetDispatchBlock } | fleet_dispatch_blocked |
| dispatch_reservation { fleet:ID, record:DispatchReservation } | dispatch_reservation_blocked |
| defense_request_attempt { record:DefenseRequestAttempt } | defense_request_attempted |
| defense_request_consideration { request:ID, before:RequestDisposition, after:RequestDisposition, tick:Tick, outcome:DefenseProtocolOutcome, reason:Option<DefenseProtocolReason> } | defense_request_considered |
| defense_action { record:DefenseCombatAction } | defense_action_recorded |

ResourceFact `{role:ResourceRole, resource:Resource, quantity:u32}` sorts by
(role,resource); roles discarded/input/output/paid/refund/transferred.
Damage `{entity:ID, before:u32, after:u32}` sorts by entity.
Modifier `{event:ID, numerator:u32, denominator:u32}` is ordered by event ID.
Entity snapshots use the exact Record union. Generic change events still contain
complete typed before/after records and actual dispositions, never just a label.
A facts tag/kind mismatch rejects; nulls obey the event's cause and resulting
state. No facts-omission mechanism is permitted in V3 canonical receipts.
Overflow of the8 MiB receipt/65536-event cap faults before publication and retains
previous state; this explicit technical choice requires review.

Decision fields: `phase:u16, actor:ID, ordinal:u32, action:IntentAction,
candidate:Option<Candidate>, target:Option<ID>, score:Score, tier:DecisionTier, snapshot:SnapshotReference,
observed_tick:Option<Tick>, outcome:DecisionOutcome,
reasons:Vec<DecisionReason>`. Add IntentAction select_research `{technology:Slug}`,
request_supply `{demand:DemandRequest}`, offer `{offer:Offer}`,
blockade `{fleet:ID, world:ID}`, board `{hull:ID, world:ID, count:u32}`.
War remains its frozen `{peer:ID}` partial action. The exact own objective/force
is required in Candidate.admission_terms; defender selection is absent there.
DecisionTier replaces inherited tiers with imminent_needs_recovery | recovery |
obligation | ordinary, categories0..3. The user-selected strict recovery gate is
retained; numerical utilities/ties are Proposed scoring §§1–8. Material decisions
require nonnull Candidate and matching action/score/tier; protocol maintenance and
free research target selection use null; target choice and its exact policy
score/reasons are carried by the free Decision/research_target_changed fact. No ordinal silently substitutes for complete admission terms.
Add DecisionReason `workforce { world:ID, required:u32, available:u32 }`,
`needs_reserve { world:ID, food:u32, energy:u32 }`,
`obligation { commitment:ID, reason:ObligationReason }`,
`supply { fleet:ID, readiness:SupplyReadiness }`,
`needs_reserved { world:ID, resource:Resource, physical_stock:u32, held:u32, required:u32 }`
and unit no_valid_defender_objective plus `protocol { failure:ProtocolFailure }`
to the closed reason union. Protocol is only the defined candidate-null free
producer, never a made-up material capacity or C-specific reason. BlockedReason also
adds needs_reserved with those exact fields. No physically present held stock is
reported as MissingInput.

Receipt payload field order: `tick, applied_revisions, events, state_digest`.
Events in this payload are EventPayload with NO event ID; applied revisions
sort by full RevisionID. Events follow stable phase1..12 producer buffers;
within each phase preserve actual local group emission order. Logical mixed
P/R/B/O/H/K/M/F/A mutation/fault order is reconstructed independently and must
not be replaced by reporting partition order.
ReceiptDigest hashes the payload; EventID derives afterwards. Public Receipt:
`tick, applied_revisions, events, state_digest, digest`, where each public event
adds id BEFORE the EventPayload fields. No digest input contains its own hash,
event IDs or an Anchor to the same receipt. Any event witness for the current
boundary is installed only at the next advancing ordinary owning handoff: phase2
for objective/Defense outcome, phase3 for paid Supply delivery. Creator-only or
paused publication, read/save/reopen/fork consumes neither handoff. Objective/
obligation state stores its latest PRIOR authenticated Anchor; the current
outcome is fully evidenced in this boundary's fact. This delay avoids a state/receipt
hash cycle and must be reviewed for its UI and witness implications.

EventKind's frozen ordinary variants are exactly: agreement_expired,
agreement_formed, civilization_dormant, civilization_reactivated,
colony_captured, combat_round, construction_cancelled, construction_completed,
creator_intervention, facility_blocked, facility_produced, fleet_arrived,
fleet_departed, hazard_ended, hazard_started, hull_completed, hull_destroyed,
refund_discarded, settlement_claim_opened, settlement_claim_resolved,
shipment_captured, shipment_delivered, shipment_dispatched, shipment_returned,
shipment_waiting, war_declared, war_ended. Prefix-free references such as
`World`, `Research`, `Resource`, `IntentAction` denote their sole named schema
in this document; `ResourceAmount` means the frozen catalog resource/quantity
record. A declaration's `Living` prefix has no effect on its wire tag.

## Identity, kind tags and sequence consumption

Additional ASCII domains, each including exactly one final NUL:
`NYON_LIVING_V3_GENESIS_ENTITY`, `NYON_LIVING_V3_CREATOR_ENTITY`,
`NYON_LIVING_V3_AUTO_ENTITY`, `NYON_LIVING_V3_FORK_BRANCH`,
`NYON_LIVING_V3_OBSERVATION`, `NYON_LIVING_V3_TRANSFER`,
`NYON_LIVING_V3_COORDINATOR`, `NYON_LIVING_V3_CLAIM_RANK`. Parent PACK/STATE/GENESIS/ROOT_BRANCH/REVISION/
RECEIPT/EVENT/ARCHIVE/NOTEBOOK domains retain their literal bytes.

Stable entity kind u16le registry: system1, star2, world3, lane4, civilization5,
deposit6, facility7, construction_job8, hull_job9, fleet10, hull11, route12,
shipment13, hazard14, offer15, commitment16, defense_request17, stellar_event18.
Derived-key records have no allocation tag. Reserved values0 and19..65535 reject.
Actor namespaces are exact civilization IDs; creator namespace is document-global
accepted sequence. System lifecycle uses the all-zero actor bytes with an explicit
namespace flag0; actors flag1; creator flag2; genesis flag3. A valid civilization
ID equal to zero is prohibited, so lifecycle cannot alias an actor.

All framing below is concatenation in the stated order. `H` is SHA256; `D(name)`
is the named NUL-terminated ASCII domain; fixed widths are little endian.
`OptRevision` is byte0 for null or byte1 plus32 revision bytes. `L(bytes)` is
u32le byte length followed by literal bytes. `C(record)` is canonical JSON bytes.
`G` is catalog_hash32 || seed32 || L(generator_id UTF8) || generator_version:u32le.
Digest fields are full32 bytes. IDs below are first16 bytes except revisions,
which retain all32. These are proposed formulas; vectors must derive them outside
Rust before any codec implementation.

- Genesis entity digest: `H(D(GENESIS_ENTITY) || G || flag3:u8 || kind:u16le ||
  ordinal:u64le)`. Generator manifest declares the complete sorted allocation
  roster `{kind:u16, ordinal:u64, id:ID}`; each kind's ordinal is positive and
  unique. The roster sorts by (kind,ordinal). Generator code never runs during decode; validate this roster against
  the explicit tick-zero manifest. Generator ID/version label provenance only.
- Creator entity digest: `H(D(CREATOR_ENTITY) || root_branch_id16 || branch_id16 ||
  flag2:u8 || accepted_sequence:u64le || operation_index:u32le || kind:u16le ||
  local:u16le)`. This deliberately excludes revision_digest and command bytes,
  allowing exact cascade previews to contain their new IDs without circularity.
- Autonomous entity digest: `H(D(AUTO_ENTITY) || root_branch_id16 || branch_id16 ||
  namespace_flag:u8 || actor_id16 || tick:u64le || phase:u16le || kind:u16le ||
  allocation_ordinal:u32le || subject_kind:u16le || subject_id16 ||
  permitted_snapshot_digest32)`. allocation_ordinal
  is contiguous0-based per (namespace,actor,tick,phase,kind) for SUCCESSFUL
  allocations. Failed proposals consume none. Stellar events use lifecycle flag0,
  zero actor and ecology phase2, subject kind2 and exact star ID. Other allocations
  use the target/hosting entity's kind and ID; an allocation with no subject uses
  kind0 and zero subject ID. Candidate subject is recorded before allocation,
  never guessed from the resulting ID. E is the framed tick and activity ordinal
  uses stable star-ID order among successful allocations. Their selection hash remains
  the ecology document's different, non-NUL stellar-activity formula.
- Revision digest: `H(D(REVISION) || root_branch_id16 || branch_id16 ||
  OptRevision(parent) || tick:u64le || ordinal:u64le ||
  mode:u8 || L(C(capture)) || L(C(command)))`, mode paused0/running1. The Revision
  record's id is not hashed. Ordinal equals the accepted sequence.
- Fork digest: `H(D(FORK_BRANCH) || root_branch_id16 || parent_branch_id16 ||
  OptRevision(parent_revision) || fork_tick:u64le || fork_state_digest32 ||
  fork_ordinal:u64le)`. Exact fork state is independently replayed. Null revision
  is unambiguous and uses no all-zero fake parent digest.
- Root branch digest is the parent's ROOT_BRANCH formula; its genesis_digest
  hashes the explicit genesis manifest with no branch identity. Genesis entity
  allocation is therefore independent of root_branch_id, avoiding a cycle.
- State digest: `H(D(STATE) || C({format_version:3,rules_version:3,
  catalog_hash:Digest,state:State}))`. This wrapper binds interpreted catalog
  and explicit version. It supersedes a bare-state interpretation of the parent
  proposal and requires replacing proposed empty-state vectors, not V2 vectors.
- Genesis digest hashes GENESIS domain plus complete Genesis below; archive,
  Notebook, transfer and coordinator integrity hash their domain plus their exact
  ordered records excluding only their final integrity field. Pack hash hashes
  PACK domain plus the complete catalog. Root, event and receipt formulas remain
  as parent except the explicitly settled RevisionID width.

Document accepted_sequence_high_water and branch_sequence_high_water start0.
A successful admission reserves high_water+1 only after full candidate command,
ID, preview, capacity and digest validation. Atomically append accepted work and
advance the durable mark together. Rejected/stale/cancelled private submissions
consume none. An accepted submission subsequently abandoned or undone keeps its
reservation: never recycle the mark. Save must either finish or explicitly
abandon pending commands before a coherent capture; abandoned accepted marks
remain durable. Fork ordinals obey the same rule. Historical State.next_sequence
and branch_sequence follow Historical counters and acceptance timing below, not future
high-water marks, so later branching never changes an old digest. Ordinary actor
allocations use their tick/phase ordinal, not creator high_water.

## Historical counters and acceptance timing

Define historical State.next_sequence as **the accepted sequence ordinal of the last creator Revision applied into this State**,0 before any revision. Despite the field name, it is not the next available number. Define State.branch_sequence as this producing branch's fork ordinal,0 on root. Ordinary evolution preserves both. Durable accepted_sequence_high_water and branch_sequence_high_water are independent document fields, never inserted into a historical State merely because another branch/queue advanced them.

For a private prepared command let S=checked(accepted_high_water+1). Compute complete creator IDs, effects, candidate validity, collision scan, Revision bytes and capacity before acceptance. Paused preparation also seals its complete receipt before publication; paused acceptance publishes candidate State/Revision/Receipt/head and accepted_high_water=S atomically. Rejected, stale or cancelled private preparation publishes nothing and consumes no number. Accepted running submission atomically retains its AcceptedCommand/private Revision/capture and advances accepted_high_water=S; committed State/head/receipt remain unchanged until its boundary. No immutable running Receipt is sealed or committed at enqueue: any projected-queue receipt size preflight is discardable, and the complete receipt is sealed only after the full accepted queue and later boundary phases finish. Once accepted, queue abandonment/Undo/fault never recycles S. Retain marks on save even when the queue is explicitly abandoned; no queue field is invented in Archive.

Application sets candidate.next_sequence=S exactly once after applying that revision's operation mutations. branch_sequence remains the producing branch ordinal. These header updates are orchestration, **excluded from creator_batch.reviewed_effects** and creator Change facts; the exact sealed State digest authenticates them. tick updates are also orchestration, not an invented operation or resource fact. Command previews cover the operation mutations/cascades in their prescribed order, without adding a fictional last operation for headers. Imported replay verifies headers independently.

## Capture timing and working-boundary distinction

Envelope stale checks always use the selected published committed cursor: branch, committed revision, completed tick t, committed wrapped STATE digest and current pending tail. Running review uses the private prefix of earlier accepted envelopes, but the envelope does not falsely call that prefix a published committed view.

**Paused:** application_tick=t; private State starts as exact selected committed State; capture is `{domain:3,catalog_hash,branch,revision:committed_parent,tick:t,state_digest:pre_command_digest,phase:1,permitted_snapshot_digest:pre_command_digest}`. Compute capture before the operation/header change. Revision.tick=t, parent=committed_parent, ordinal=S, mode=paused. Its own ID is absent from capture/command; creator/lifecycle permitted digest equals State digest under Events, decisions and authenticated receipt schemas.

**Running (Proposed explicit private-prefix convention):** T=checked(t+1). Begin one private boundary candidate from committed State and set its working tick=T **before** creator phase1; no publication has occurred. Apply accepted envelopes in accepted-sequence order. Before each revision, capture this working State, including prior creator-prefix mutations and prior applied next_sequence, with phase1/tick=T/revision=previous queued Revision or committed parent. Revision.tick=T and parent names that same logical parent. state_digest and permitted_snapshot_digest both hash this exact prefix. C's candidate effects use application boundary T for clocks/provenance. After its operations set next_sequence=S; the next accepted revision captures that result.

This phase1 SnapshotReference authenticates an intermediate replay position, **not a completed HistoryView at T**. Replay reconstructs it by setting private tick=T then applying only earlier accepted revisions in that boundary; it never resolves it by reading the eventual phase12 State or a cached ordinary receipt. Actor phase7 captures remain the already-defined later source, and no future actor/root facts enter a creator prefix. The existing phase1 tag and ordered parent chain identify this position without adding a public prefix command/field. This is a substantive Proposed clarification and must be written consistently into snapshot validation, not silently assumed by a generator.

At queue submission, reconstruct exactly that prefix for T and stage the candidate capture/effects/Revision before advancing the durable mark. At execution reconstruct it again and byte-compare capture, effects, IDs and result. A mismatch faults the whole unpublished boundary and retains the accepted queue/marks for diagnosis; no prefix Revision/State/Receipt is committed. Committed revision in the envelope remains its submission cursor, while Revision.parent/capture.revision names the private logical parent. This distinction prevents earlier pending acceptance from pretending to be already committed.

Paused edits require empty queue. A pending tick job must be finished or abandoned under existing ownership/freshness rules before a new publication; a queued accepted command is never silently rewritten into a paused command. Pause alone changes pacing, not authoritative tick/headers/queue/marks.

## Exact creator event cardinality and ordered facts

Select **one creator_intervention per applied Revision, plus one creator_cascade iff that Revision has indirect consequences**, both creator provenance `{revision:R}`, phase1. No per-operation intervention, per-field operation_changed, duplicated generic semantic event or fabricated autonomous payment/fulfillment is emitted for the same creator effects. Paused and running use this same rule. Specialized actual autonomous phase facts remain unchanged in ordinary evolution.

Partition the validated full reviewed_effects without altering Change bytes:

This aggregate event partition is reporting only. Operation mutations and cascades execute in their original operation/effect order before receipt construction; the two event lists are not an instruction stream for reapplying direct changes before indirect changes. In particular, a later operation sees an earlier operation's indirect consequence immediately. Replay derives the ordered mutations first and then validates the emitted partition and its recombination.

- A Change is **direct** if its `(collection,key)` is in the operation's direct-write set below. The full row snapshot remains direct even when the same row contains recomputed fields (for example a direct Colony population edit includes its recomputed assignments). A Change to another row is indirect.
- Every creator Change has its exact operation_index; no null creator index or generic header Change is introduced. resources/passenger_delta remain on that same Change. Multiple operations affecting the same row retain separate ordered Changes with their actual before/after snapshots; only duplicate consequences within one operation are forbidden.
- creator_intervention Facts.change contains all direct Changes, preserving original `(operation_index,collection order,key)` order. Emit it even if that list is empty for a legal no-op batch; no-op batch admissibility must obey the existing Command constraints, not a new empty-command allowance.
- If indirect Changes exist, creator_cascade immediately follows with exactly those Changes in the same preserved order. Each Change appears in one event only. Combining both lists and sorting by the frozen reviewed-effect order reconstructs the submitted list byte-identically. No third event repeats a cascade/refund/passenger loss already in these Changes.

**Closed direct-write set selection:** use resolved actual keys, including earlier batch-local creation IDs. Create/Edit/Set/Cancel/Remove operations directly write their named primary row; dependent invalidations/refunds/other collection rows are indirect. Exact groups:

| Operation group | Direct rows |
| --- | --- |
|CreateSystem/EditSystem; CreateStar/EditStar/SetActivityClock; CreateWorld/EditWorld/SetWorldInventory/SetWorldOwner/SetEnvironment; CreateLane/EditLane; CreateCivilization/EditCivilization/SetCivilizationPolicy/SetResearchTarget/GrantTechnology/RevokeTechnology; CreateDeposit/EditDeposit/SetDepositRemaining|Respectively named systems/stars/worlds/lanes/civilizations/deposits row. Research dependent technology closure stays in the named Civilization full row; unrelated affected facilities/colonies are indirect.|
|CreateFacility/SetFacilityEnabled/ScrapFacility; CreateConstructionJob/CancelConstructionJob; CreateHullJob/CancelHullJob; CreateColony/RemoveColony/SetPopulation/SetSpecialization|Named facilities/construction_jobs/hull_jobs/colonies row. Storage refunds and other owners/assignments in other rows are indirect.|
|CreateFleet/EditFleet/SetFleetOrder/SetFleetSupply/SetBlockade; CreateHull/EditHull/SetPassengers|Named fleet row; hull operations use the actual containing/newly created Fleet because hulls have no separate top-level Record. Any distinct passenger origin/destination Colony adjustment is indirect for SetPassengers.|
|BoardPassengers/UnloadPassengers|Containing fleet row AND the explicitly named origin/destination Colony row, including its full resulting society. Other rows are indirect. No duplicated hull Record variant.|
|SplitFleet/MergeFleets|All fleets explicitly named as source/target/new result of that operation; removal of emptied named source is direct. Additional reservation/order dependencies in other rows are indirect.|
|CreateRoute/EditRoute/SetRouteSuspended; CreateShipment/SetShipmentDisposition; CreateHazard/EditHazard/CancelHazard; CreateStellarEvent/EditStellarEvent/CancelStellarEvent|Named routes/shipments/hazards/stellar_events row. Creator shipment provenance stays Creator; no paid fulfillment fact.|
|SetRelationBase; SetAgreement/RemoveAgreement; ForceWar/ForcePeace/SetWarObjective|Exact relation key, agreement key or War pair respectively. ForcePeace's installed Truce and cleared fleets/requests are indirect; no capacity exception is invented.|
|CreateOffer/CancelOffer; CreateCommitment/EditCommitment/RemoveCommitment|Named offers/commitments row; linked agreement/request/delivery changes in other rows are indirect.|
|RemoveEntity|Explicit target's actual primary row, or containing Fleet for a nested Hull target. Reviewed dependent removals/changes in other rows are indirect even when their IDs are in the cascade assertion.|

If an adopted operation has an additional direct target not covered here, this registry must be amended/reviewed before release; it is not an open string classification. No extra operation is invented to cover it. This partition is a Proposed event convention rather than a V2 transcription: V2 source emits additional semantic kinds for some effects but lacks this V3 full-Change partition.

For one paused Revision, ordinals start0: intervention0, optional cascade1. For running phase1 process each accepted Revision in accepted sequence order and append its one/two events consecutively, allocating the next boundary-global ordinal. Later phases append after all phase1 events. Receipt.applied_revisions is separately sorted by full RevisionID, not event/application order. All subsequent Event IDs derive only after Receipt payload sealing, and no Change/State includes this receipt's Event/Anchor hash. Receipt size/event cap failures abort publication under the frozen contract.

## Forks, restore and durable marks

At a retained selected parent HistoryView V with exact State P and digest DP, queue empty, propose fork ordinal F=checked(branch_high_water+1). Validate capacity, ancestry, identity/collision and any combined first command before accepting the fork. Derive existing FORK_BRANCH formula from original parent view/DP, not a child-adjusted State. New Branch fields are `{id:B,parent:V.branch,fork_revision:V.revision,fork_tick:V.tick,fork_state_digest:DP,ordinal:F,head:V.revision,tip_tick:V.tick}`. Inherited head null remains null; no invented fork Revision is appended.

Child initial State is P with **only branch_sequence=F**; tick and next_sequence retain P's historical values. Its child State digest may differ from DP. This deterministic local fork transition is authenticated by Branch.ordinal and replay; Branch.fork_state_digest continues to equal the parent's pre-fork digest. Parent State/receipts/tip/future are unchanged. Child initial capture/Bookmark uses the child-adjusted digest and inherited revision cursor. No fork Receipt/event/Anchor is invented. A new child receipt exists only after a real paused publication or ordinary boundary.

Accept fork publication and branch_high_water=F atomically. If first paused command is combined with the fork, reserve its S privately and publish fork/command/receipt and both marks together only after all checks; failure consumes neither F nor S and preserves the old selection/future. A separately accepted fork remains accepted if a later separately submitted edit rejects. Fork-only acceptance changes no accepted-sequence high-water. Current restore reads exact historical State/cursor plus producing branch ordinal; it does not copy newer sibling durable marks into that State. Document durable marks stay their maxima.

Automatic fork-before-edit after historical rewind uses this same transition, not another command tag. Replaying child history first authenticates exact parent fork view, copies it and applies branch_sequence=F, then processes child publications. Reencountered inherited issuance retains original issuer branch/origin, while new child creator/autonomous issuance uses child B. Ordinary child ticks preserve F until a different fork is created. Abandoned accepted sequence gaps do not vanish during restore or export.

## Notebook common ordinal and exact comparison construction

Select one Notebook-global positive u64 ordinal namespace for Bookmark, Experiment and Evidence. next_ordinal is the first never-issued value; new empty Notebook starts1. A prepared insertion proposes O=next_ordinal, checks O>0 and checked(O+1), caps/references/values/integrity and exact attachment capture, then atomically installs the record and next_ordinal=O+1. An atomic multi-record operation assigns consecutive values in explicit dependency order (new Bookmarks first in left/right order if needed, Experiment next, then Evidence input order) and commits all or none. Existing public portable shapes stay unchanged; these are Notebook construction operations, not new gameplay Command variants.

Rejected/stale/cancelled private Notebook preparation consumes none. Once a record was accepted, deleting/undoing it never decreases next_ordinal; holes are legal. Label edits and recomputations retain its ordinal. Import validates every live ordinal positive/<next_ordinal and globally unique across the three arrays; each array independently sorts ascending. A mark above the live maximum is legal and preserves removed-record reservations; never recompute next_ordinal=max(live)+1. Import has no authority to authenticate arbitrary claims about removed content beyond that retained mark. This is an explicit new V3 policy; V2's separate evidence mark is reference evidence only.

**Bookmark:** exact retained view/digest independently replayed, label metadata only. Bookmark.digest is wrapped STATE digest, not a Bookmark hash. A fork-only child Bookmark can be valid without a receipt Anchor; it must not invent one.

**Experiment:** left/right name retained Bookmarks whose view.tick equals comparison_tick exactly; no implicit forward evolution/nearest tick. Replay both full views under the same exact Catalog/root. Their labels/selection need not be different, but existing kind tags are descriptive observation categories, **not a proof of causal success/completion**. Validate every requested metric/reference/value and absent-versus-zero semantics from the frozen table; an empty observations/witnesses list is permitted structural metadata, not qualified evidence of the named activity.

Set authored_difference to the full-RevisionID-sorted symmetric difference of the two exact selected views' applied creator ancestry sets: include inherited/local Revisions actually applied by the selected cursor/tick, exclude accepted-but-abandoned/unapplied queue records, and exclude common prefix IDs once. Do not substitute command labels, Branch IDs, autonomous Event IDs or chronological order for those RevisionIDs. Divergent fork ordinary outcomes may differ even when this set is empty; the field does not assert an isolated causal treatment or State equality. Replay independently derives and compares it; no trusted submitted difference list. Same-tick root view R1 versus R2 yields `[R2]` when R2 is the sole additional applied Revision.

**Evidence:** bookmark must name one of the referenced Experiment's left/right Bookmarks; anchor must bind that Bookmark's exact branch/revision/tick/wrapped STATE digest and an independently regenerated receipt of that exact publication. Event references sort by frozen `(phase,ordinal,id)` and verify against that receipt; anchor.event, when nonnull, also verifies kind/phase/ordinal/ID. Entities resolve at that Anchor view, not current State after removal. Whole-receipt Anchor with empty entity/event lists is permitted as a receipt observation, not proof of a gameplay event. Require Evidence.anchor to be present once in Experiment.witnesses; adding Evidence plus a witness-list update is atomic and retains Experiment ordinal. Shared identical witnesses deduplicate by full frozen Anchor key, never kind label. A fork-only view lacking a receipt cannot supply this Evidence until a real publication exists.

All Archive anchors referenced by Notebook evidence/witnesses must be retained in its exact attached Archive descriptor set. Authenticated coherent reattachment to changed Archive bytes independently verifies the retained views/receipts again and updates archive_integrity_sha256 plus Notebook integrity atomically; it never rewrites an Anchor to the current head. No receipt bytes/cache/private verification flag become portable authority. Transfer then hashes exact catalog/archive/notebook/selected_view bytes. Native download/durable store proof remains separate from a portable_reproduction Experiment tag.
Before admission, reject a duplicate issuance origin/frame or a truncated ID
collision with a different full digest anywhere in candidate/live or retained
replay history, including transient removed entities. Do not retain a lifetime
issuance map. The exact bounded collision-validation algorithm is:

1. Freeze genesis, retained branch graph/tips, all authored inputs and allocator
   marks alongside the candidate's capture. Prepare at most1024 proposed issuance
   entries per batch, each at most256 bytes: namespace/kind, ID16, full digest32
   and exact issuance origin locator. Creator origin is (branch,accepted_sequence,
   operation_index,kind,local); genesis origin is (kind,ordinal); autonomous origin
   is (actual issuing branch,actor,tick,phase,kind,allocation_ordinal,subject).
   Check duplicate/collision pairs within the batch and live candidate first.
2. A cache at most65536 fixed256-byte entries/16 MiB may accelerate comparisons.
   It holds verified issuance entries and is purely discardable. Every cache miss
   requires a deterministic streaming history scan; absence from the cache never
   proves uniqueness. Scan retained branches in ascending ID and each exact
   ancestry from genesis to retained tip; shared ancestry may be re-encountered.
3. The scanner replays one branch at a time under the ordinary visit/segment budget
   and emits each actual allocation's origin, frame digest and ID. It does not
   accumulate old IDs. Compare each emitted entry against the bounded proposed
   batch. Same origin/full frame seen again during inherited-prefix or repeated
   validation replay is the same historical issuance, not a second admission.
   A candidate replay's own exact origin is excluded from its prior set; distinct
   origins producing equal full frame reject DuplicateIdentity, and equal ID16
   with different digest32 rejects Collision. Separate live structural validation
   still rejects two simultaneously installed records with one ID.
4. Scanner mode performs deterministic phases and structural/live collision checks
   but does not recursively invoke historical scans. Its transcript is private,
   unpublished and untrusted until the outer validation has checked its allocation
   batch against the retained scan. Import processes each emitted issuance batch
   with this same outer scan, so later allocations, removed transient records and
   nonselected sibling tips receive the same collision checks. An already-validated
   inherited origin need not be treated as a new allocation at a fork.
5. On cache eviction or branch/tip change restart/reconcile the exact scan cursor
   against a fresh capture; never use stale absence results. Cache eviction leaves
   authority unchanged. The collision validator owns at most64 MiB total workspace,
   including a candidate/scanner state capped8 MiB each, the16 MiB cache and bounded
   batch/cursor/encoding storage. Immutable captured archive/catalog owners are
   shared, not deep-copied; their existing64/1 MiB input caps remain independent.
   Additional scanner branches are processed sequentially, not retained in RAM.

These repeated scans deliberately exchange time for bounded memory and complete
retained-history collision coverage. Work segments retain exact inner and outer
cursors and return more_work_required at the published segment limit; no whole-tip
scan runs inside one poll or publication fence. Total cumulative work uses checked
u128 counters; schema u64 tick/ordinal limits and bounded per-boundary allocation
counts make the declared finite captured history enumerable. Any arithmetic or
workspace overflow rejects import/admission before publication, never silently
narrows collision scope. For continued ordinary ticks, a collision scan can remain
Pending across arbitrarily many segments; pacing waits for complete publication
rather than skipping the scan. Named-device timing remains an unverified gate:
this specification does not claim the expensive rescan meets the5 ms target.

No nonce retry, random salt or silent overwrite is allowed. Creator collisions
reject admission; deterministic actor collisions fault the unpublished boundary.
A fork's inherited origins retain the actual parent issuer branch, while new
post-fork autonomous allocations use the child branch. Discarded unreachable
history is outside the defined collision universe; every retained replayable
issuance, including one whose receipt was evicted, remains inside it.

## History, archive, Notebook and transfer schemas

```text
Genesis { kind:String, format_version:u32, rules_version:u32,
          catalog_hash:Digest, seed:Seed, generator:Generator,
          allocations:Sorted<GenesisAllocation>, state:State }
Generator { generator_id:Slug, generator_version:u32 }
GenesisAllocation { kind:u16, ordinal:u64, id:ID }
HistoryView { branch:BranchID, revision:Option<RevisionID>, tick:Tick }
Branch { id:BranchID, parent:Option<BranchID>, fork_revision:Option<RevisionID>,
         fork_tick:Tick, fork_state_digest:Digest, ordinal:u64,
         head:Option<RevisionID>, tip_tick:Tick }
Bookmark { ordinal:u64, label:Name, view:HistoryView, digest:Digest }
Archive { kind:String, format_version:u32, rules_version:u32, catalog_hash:Digest,
          genesis:Genesis, revisions:Sorted<Revision>, branches:Sorted<Branch>,
          active_view:HistoryView, final_tick:Tick, final_digest:Digest,
          accepted_sequence_high_water:u64, branch_sequence_high_water:u64,
          anchors:Sorted<Anchor>, integrity_sha256:Digest }
Notebook { kind:String, format_version:u32, rules_version:u32, catalog_hash:Digest,
           genesis_digest:Digest, archive_integrity_sha256:Digest,
           next_ordinal:u64, bookmarks:Sorted<Bookmark>,
           experiments:Sorted<Experiment>, evidence:Sorted<Evidence>,
           integrity_sha256:Digest }
Experiment { ordinal:u64, label:Name, kind:ExperimentKind, left:u64, right:u64,
             comparison_tick:Tick, authored_difference:Sorted<RevisionID>,
             observations:Vec<MetricObservation>, witnesses:Sorted<Anchor> }
ExperimentKind = supply_chain | autonomous_settlement | trade_delivery |
                 corridor_impact | war_alternative | dormant_recovery |
                 portable_reproduction
Evidence { ordinal:u64, bookmark:u64, experiment:u64, anchor:Anchor,
           entities:Sorted<ID>, event_references:Sorted<EventReference> }
MetricObservation { metric:Metric, reference:MetricReference,
                    left:MetricValue, right:MetricValue }
MetricReference = document | world { id:ID } | civilization { id:ID } |
                  fleet { id:ID } | research { civilization:ID,technology:Slug } |
                  objective { war_a:ID,war_b:ID,actor:ID }
Metric = inventory_energy | inventory_ore | inventory_alloy | inventory_food |
         owner | colony_count | facility_count | hull_count | tick | population |
         carrying_capacity | workforce | research_points | habitability | stress |
         fertility | commitments | objective_status | fleet_readiness
MetricValue = integer { value:Score } | unsigned { value:u64 } |
              entity { value:Option<ID> } | text { value:Slug } | absent
Transfer { kind:String, format_version:u32, rules_version:u32,
           catalog:Catalog, archive:Archive, notebook:Notebook,
           selected_view:HistoryView, integrity_sha256:Digest }
Coordinator { kind:String, format_version:u32, authority:String,
              rules_version:u32, slot:ID, catalog_hash:Digest,
              view:HistoryView, generation:u64, integrity_sha256:Digest }
```

Genesis kind is NYON_LIVING_GENESIS_V3. Other kinds are exactly the parent kinds;
Coordinator kind is NYON_SANDBOX_SELECTION_V3, format_version3,
authority living_v3 and rules_version3. This first proposal selects only V3;
old valid V1/V2 selections use their existing records when newer coordinator is
absent. Version-specific UI routing can name all three without reinterpreting
one record's payload as another. A present corrupt/newer unsupported record
preserves the resident and reports a typed error, never falls back automatically.

Archive branches sort by ID and form a rooted acyclic tree; root parent/revision
null, fork_tick0, ordinal0, fork_state_digest equals genesis state digest.
Each nonroot fork references an exact parent view at/before parent's retained tip
and uses positive unique global ordinal. Revision parent graph is acyclic;
branch head and active revision are reachable on that branch's inherited path;
mode and tick imply a legal chronology. head null is valid for genesis-only
branches. tip_tick is at least fork_tick; active final_tick equals view.tick.
Allocator marks dominate every retained accepted/fork ordinal, including high
marks advanced then abandoned. No out-of-order command is accepted by sorting it.
Historical rewind followed by a new edit creates an explicit sibling fork before
admission, preserving the original future; fork capacity failure leaves it intact.

Anchors sort by (branch,revision with null first,tick,receipt_digest,event.phase,
event.ordinal,event.id), null event first; duplicates reject. At most16384 anchors and4096 Evidence
records; each experiment at most64 witness anchors and1024 observations, each
Evidence at most1024 entities/events. Combined top-level byte caps still apply.
References to removed entities resolve at the exact Anchor view, never current
state. Bookmarks/experiments pin their branch/revision ancestry; removing pinned
history rejects until an explicit authored unlink/removal. Branch removal requires
all retained descendant branches and dependent evidence to be named in a reviewed
cascade, cannot remove root or active branch, and reclaims owners incrementally.
Ordinary evolution does not append authored revisions and continues at their cap.


Bookmark.digest is exactly the selected view's State digest from the explicit
version/catalog State wrapper, not a hash of Bookmark or its label/ordinal.
Replay must reproduce view and compare that State digest byte-for-byte. Notebook
bookmark ordinals are positive unique and below next_ordinal; experiment left/right
must reference retained bookmarks at comparison_tick. Label edits do not change
the selected State digest. All witnesses bind exact Archive/Catalog/receipt views.

MetricObservation is a closed metric/reference/value contract. Compute each side
from its exact selected State/catalog. If the referenced entity/research owner/war
or objective is absent on one side, use absent on that side; do not invent zero.
Unknown catalog technology rejects both the request and import. Existing entities
with zero count/progress return integer0. Objective references require sorted
war_a<war_b and actor one of those participants. The rows below allow no other
reference or value variants. Null owner is entity.value=null, not absent.

| Metric | Required reference | Value and exact meaning |
| --- | --- | --- |
| inventory_energy,inventory_ore,inventory_alloy,inventory_food | world | integer local corresponding units |
| owner | world | entity owning civilization or null |
| colony_count | civilization | integer count of that actor's colonies |
| facility_count | world | integer count of completed facilities at world |
| hull_count | fleet | integer hull-array length |
| tick | document | unsigned selected tick, preserving full u64 range |
| population | world | integer colony population; absent if no colony |
| carrying_capacity | world | integer derived effective society/ecology capacity; absent if no colony |
| workforce | world | integer assigned facility count; absent if no colony |
| research_points | research | integer exact technology progress,0 if eligible known technology has no stored progress |
| habitability | world | integer effective habitability from exact current ecology/technology rules |
| stress,fertility | world | integer corresponding environment value |
| commitments | civilization | integer active commitments involving actor |
| objective_status | objective | text pending/succeeded/invalid from exact participant objective |
| fleet_readiness | fleet | text supplied/supply_unavailable/never_supplied from exact Supply |

MetricObservation ordering is lexicographic (metric tag,reference tag,canonical
reference bytes); duplicate metric/reference pairs reject. Values are verified
against independently replayed states, not accepted because types parse.

No receipt bytes, decoded checkpoint or private index are portable authority.
Archive stores replay inputs and Anchor descriptors; replay reconstructs referenced
receipts and verifies state/receipt/event digests and typed facts before installation.
Notebook binds the exact archive bytes, not an archive's name or just active digest.
An export may fail64/96 MiB while the active document remains usable; do not silently
omit evidence or history. Import validates all retained branch tips and evidence,
not only the selected view, and creates a distinct new slot after successful replay.

## Optional minimal tick-zero bootstrap profile (Proposed)

This named profile is an independently decidable hub/deposit/docked-scout family, not a new portable profile tag or a global restriction on richer Genesis. General broader initialization remains open where its successful predicates are not declared; unsupported profile coverage is not invalid wire. Use explicit validated base records and complete initial projection, never execute Generator to recover missing fields. The concrete home/deposit/scout examples below illustrate the11-ID family and are not universal generator defaults.

An initial Shipment paid SnapshotReference or creator CreatorCapture naming this document's root is definitely cyclic: branch -> rootID -> Genesis digest -> initial Shipment -> branch. This profile has shipments=[] and no initial branch-bound Capture/Anchor. Authentic paused tick0 revisions can create creator cargo only after sealing root; real paid dispatch needs its actual withdrawal. No new genesis provenance form, zero branch or fixed-point search is permitted. Branch-free SupplyPayment/scalars/observed threat fact digests and genesis StellarEvent provenance are not intrinsically cyclic; richer initial truth/chronology require separate review.

## Proposed exact bootstrap phases and validation table

These are local initialization stages, **not** new authority Event phase tags. Do not emit phase0 events or manufacture a tick0 ordinary Receipt. Existing Event phase registry remains1..12. 

| Stage | Proposed exact work | Explicit input vs derived verification |
| --- | --- | --- |
| B0 inputs | Pin immutable approved Catalog; read complete ordered Genesis/State plus seed and provenance-only Generator; reject actual grammar/cap/reference/frame faults; separately classify a richer structurally valid family as outside_family/unsupported profile | No fabricated invalid_wire for profile mismatch. No procedural reconstruction or generated fallback for omitted fields. Validate base rows and allocation correspondence before B5; do not run visibility from malformed input or repair it afterward. |
| B1 identities/topology | Compute each Genesis allocation frame/ID; sorted positive(kind,ordinal), no requirement of contiguous ordinals; validate exact live correspondence, typed references and global ID uniqueness including nested hulls | In this profile the roster has only system/star/world/lane/civilization/deposit/fleet/hull rows. Colony has no allocation row. Lane endpoints resolve to distinct systems; arrays use their declared keys. Initial bytes cannot be assigned a fresh ID after hashing. |
| B2 clocks/primary asset limits | State.tick/branch_sequence/next_sequence="0"; Star.next_activity_tick="1000"; World.environment.next_environment_tick="100"; Colony.hub.energy_next_due="10", ore_next_due="100", fallback_next_due="100"; Colony.society.next_needs_tick="100" | Star genesis+1000 is already explicit in ecology §Stellar activity; hub genesis+10/+100 is inherited accepted rules:95. Initial environment/needs first-boundary convention is the Proposed extension. Exact fields are serialized Tick strings; no alias fields, catch-up or hidden prior operation. All arithmetic checked. |
| B3 status/research/workforce | Status Active iff actor owns at least one Colony, otherwise Dormant; initial research target null/progress[]/completed[]; recompute assignments from enabled completed facilities and floor(population/100), specialization/ID order | The minimal profile has no facilities, therefore workforce_assignments[] regardless of available workers. Population0 alone does not delete ownership or imply Dormant when a colony remains. Validate submitted status/assignments; do not silently rewrite them. |
| B4 no prior attempts | hub.blocked_reasons[]; Society.last_needs null; fleet Supply both records null/readiness never_supplied; hull full catalog HP, next_repair null, passengers0 for scout; fleet order null/blockade null | Explicit fields. Society.last_needs remains null even at population0. Empty explanations mean no attempt has occurred, not a serialized zero-needs/fully-paid result or artificial recipe execution. No initial energy withdrawal, return credit or supply payment; scout.return_credit=false. All future changes occur in ordinary authenticated phases. |
| B5 initial observations | Perform one pure authorized direct observation projection at tick0, from the submitted validated base State, before its final hashes | Compute V(A,w) solely from the validated base rows using the exact predicate below, never submitted observations. Serialize exactly one row per true V(A,w), sorted(civilization,world). A docked own fleet with any hull whose resolved HullDefinition.role=scout reveals its system, including mixed fleets; travelling fleets add no docked-world sight. Every owned world is directly visible even without Colony/fleet under I1. In the concrete profile only home/world1 is visible; public lane topology does not reveal remote dynamic data. |
| B6 complete scopes/facts | For each B5 row copy exact owner/environment; observed_tick0; correct deposit/facility/hull summaries; required defense count0; threats[] for this peaceful profile | Home's single ore deposit gives remaining_deposit20000; facilities[]; stationed_hulls=[{definition:scout,count:1}]. Stellar_scope is system1/observer=civ1/tick0/events[]; hazard_scopes contains the complete empty assertion for each incident known lane, here lane1. Initial known lanes means the complete public declared static Lane topology for this bootstrap projection, a Proposed exact source-selection choice. No remote hazard schedule is read absent an incident visible world. |
| B7 seal/reject | Recompute the whole expected derived initial projection and compare each submitted field byte-for-byte; reject absent or inconsistent source/fact/clock fields. Canonically freeze complete Genesis, compute STATE wrapper and GENESIS/root digest, then root Branch/view/high-water0 | No decode-and-fill defaults. No derived field is repaired after hashing. Bootstrap validation uses the explicit Genesis base as its original-boundary source, not a future ordinary replay/ActorInputContext. No Receipt Anchor is generated or required for B5. First normal boundary1 refresh can then derive observations with timestamp1. |

### Exact bootstrap visibility source (Proposed)

For actor A and world w in the validated explicit base State, define V(A,w) as the disjunction of:

1. `w.owner == A`, including owned worlds without a Colony or fleet (I1-owned-world extension).
2. An own Colony c has a referenced world whose system equals w.system.
3. An own Fleet f has location `{type:docked,world:v}` and v == w.id.
4. An own **docked** Fleet f at world v has **any** hull h whose catalog-resolved HullDefinition.role is scout, and v.system == w.system.

No term reads submitted Observation/SharedIntelligence/DefenseRequest, inferred visits, travelling endpoints or public lane endpoints. A mixed scout/escort fleet satisfies term4; a scout definition is identified by its role, not a slug/name guess. Only existing validated IDs/kinds/catalog definitions may be used, and B1 must succeed before evaluating V. In this bootstrap projection known_lanes is the complete declared public Lane topology; for each visible w, B6 includes only lanes incident to w.system. Empty complete scope for that incident lane does not make its remote world visible. All world geometry/ownership/docked-hull operands here belong to explicit initial authority, never a new permission for later actor forecast root reads. 

Tests of this predicate separately cover owned non-colonized worlds, colony-only system visibility, a mixed docked scout fleet, non-scout docked fleet world-only sight, travelling scout absence and a submitted remote Observation attempting to authorize itself. These predicate tests do not silently broaden the exact literal fixture's base assets or claim a richer family admitted.

The bootstrap source/evaluation contract and profile restrictions are Proposed. Accepted V2 is reference evidence; the V3 record/clock/source declarations here own this profile.

This profile requires source records **explicit and verified**, not reconstructed-only transport. Admitting a producer that omits observations/workforce/readiness and letting a decoder fill them would violate required-field/no-default/canonical-byte rules. A producer may calculate them before serialization; admission independently calculates them again from permitted bootstrap sources and rejects mismatch. The branch-free initial ObservationFacts/Scope data is part of Genesis bytes, so it is sealed before root derivation. Actor planning does not run during bootstrap; this uses the Actor32 source registry for later captures and scoring conditional forecast for later interactions. 

## Initial collection policy: minimal profile versus broader possibilities

The restriction below is selected as a **minimal fixture/bootstrap profile only**, not a global V3 admission rule. Richer well-formed families are outside_family/unsupported by this profile, not invalid_wire. Actual frozen grammar/reference/frame/cap faults still reject under their applicable predicates, under the decoder stages below where the applicable predicate is defined. No authority decoder may infer a blanket family prohibition from this narrow profile. A future global restriction would be a separate explicitly reviewed and human-approved content rule.

| Collections/fields | Proposed minimal profile | Broader contract or definite dependency |
| --- | --- | --- |
| systems/stars/worlds/lanes/civilizations/deposits/colonies/fleets/nested hulls | Explicit validated base rows, exact clocks/Research/Society/Supply limits as B0–B6 | General numerical/topology caps retain source values; no new balance or starter generator algorithm. |
| observations | Required B5/B6 exact direct rows | Initial stale, shared-origin, partial or omitted visible rows reject this profile. A richer profile needs an original source boundary, never a timestamp invented during load. |
| facilities/construction_jobs/hull_jobs | Empty | No inherent SnapshotReference cycle. Initial paid credits, repair/HP/blocked/RecipeFacts, accepted/completion chronology and reserved target fleet/hull correspondence require their own reviewed initialization table. Separate industrial vs hull reservations stay unchanged. |
| routes | Empty | Bare route commitment null and valid initial endpoints/clock is acyclic; initial eligibility/suspension/due/payment-history policy needs review. Actual later dispatch provenance still requires root. |
| shipments | Empty; initialize only after authenticated root exists | Definite paid/creator branch dependency above. Do not add a fake genesis ManifestProvenance or relabel cargo as paid. |
| hazards/stellar_events | Empty | Hazard bare rows and StellarEvent genesis provenance can be acyclic; initial warning/active schedule and event explanation/clock convention needs explicit richer-profile review. Not forbidden by the existing kind14/18 registry. |
| relations/agreements/wars/offers/commitments | Empty | Bare rows/null witnesses can be acyclic. Initial reason balances, eligibility, attempts, deadline/objective state and witnessed prior lifecycle require independent review. Any nonnull branch-bound Anchor must follow real authenticated history. |
| demand_requests/defense_requests/shared_intelligence | Empty | Requests require actual eligibility/capture and copied source evidence; threats have acyclic fact digests but need the source convention. No root-cycle claim solely from the word snapshot. Run no bootstrap negotiations or actor intentions. |
| settlement_claims/occupations | Empty | Initial participant/arrival/close/progress/eligibility and historical seed arbitration/combat facts unbound; no fabricated prior settlement/combat. Later runtime interactions use actual conflict rules; private scoring uses Conditional private B/F operator. Broader initial chronology still needs separate rules. |
| counters | Explicit{} | No extension bag or initialized hidden counters. |

No initial collection in this minimal profile contains actor-paid provenance. This is narrower than prohibiting all declaration-time balances/assets. After root sealing, a creator revision may freely author allowed initial assets with exact CreatorCapture/Receipt, and later authority phases may create actually paid cargo/jobs/supply. Do not retroactively change the Genesis manifest to flatten those publications into tick0 base content. 

## Checkpoints, retention and bounded work

Checkpoint is a local discardable cache: `{view:HistoryView, state_digest:Digest,
state:State, prior_receipt:Option<Receipt>}`. It is never trusted before the
associated replay chain is validated. Cache at most8 checkpoints,64 MiB combined,
one automatically every1000 completed boundaries if capacity allows. Evict oldest
unselected view first, then oldest tick/branch-ID; eviction never changes canonical
state/history/evidence. If the selected view alone exceeds cache cap, skip caching,
not authority validation. Imported checkpoints are not supported.

Keep at most128 recent full receipts and16 MiB total receipt cache. LRU eviction
uses (last access counter,branch ID,tick,receipt digest); reference Anchor records
remain retained and replay-authenticated. Current operation facts can be displayed
from the latest sealed receipt; an evicted receipt first shows Pending replay,
not fabricated reason text. Historical references remain navigable as long as
their genesis/catalog/revision/fork ancestry remains retained. No timing or cached
state affects canonical digests. Retired owners enter a bounded queue of at most16
owners/128 MiB; stop admitting further jobs and poll reclamation when full, rather
than dropping a capacity-sized graph synchronously.

Every poll receives1..1024 visits shared across decode, copy, dependency indexing,
phase/forecast traversal, hashing/encoding, replay, comparison and reclamation.
A visit processes one scalar/reference/key or at most4096 already-bounded bytes,
never one entire record with unbounded child arrays. Nested nodes spend additional
visits; hash/encode buffers cannot secretly copy8 MiB at once. Final publication
moves prepared owners only after exact resident/cursor/queue/allocator checks;
all fallible work and capacity-sized destruction precede or follow that fence.

A replay work segment advances at most1000 boundaries and16777216 visits, then
returns a typed resumable continuation with exact cursor and cumulative progress.
The supervisor may admit the next segment against the same capture, so histories
past100000 ticks do not acquire a fake global tick cap. Cancellation/staleness
between segments preserves the active resident and never publishes a prefix.
No single poll, request preparation, or continuation revalidation scans all history.
These segments are transport work limits, not simulation balance changes.

Comparison owns two exact retained views at one tick, replays both under the same
budget, then streams stable collection/key differences. Difference fields:
`collection:CollectionKind, key:TypedKey, before:Option<Record>,
after:Option<Record>`. Pages cap64 KiB and256 differences, excluding one indivisible
record of at most8 MiB which gets a dedicated page. Total comparison output cap16 MiB;
if exceeded, return OutputCapacity with a resumable filtered query, no complete
comparison claim and no omitted rows disguised as equality. Report total scanned
and matched counts. Export streams coherent capture bytes in4096-byte pieces,
checking freshness at each platform handoff; caps catalog1 MiB, genesis/state8 MiB,
archive64 MiB, Notebook8 MiB, receipt8 MiB, transfer96 MiB, Coordinator504 bytes.
Coordinator cap is checked before UTF8/parse/unsupported-kind checks as I6 specifies.
Memory output buffering
caps1 MiB besides retained authority; native staging is disposable until commit,
browser handoff receipt is distinct from durable file proof. Cancellation racing
an already-won durable store CAS reports the actual completed generation.

## V3 decoder stages and exact error selection (Proposed)

These stages apply to a document entering an explicitly selected V3 reader. A shared cross-version router needs its own bounded envelope routing and must preserve existing V1/V2 error interpretation. A parser error never silently tries an older slot/reader. All stages run unpublished; segment exhaustion/cancel/stale are job outcomes, not malformed input. Later stages are never run merely to select a more attractive error.

| Order | Work and proposed selection |
| --- | --- |
|0 raw bytes|Apply that entry point's byte cap before UTF-8/parsing/allocation. Return byte_capacity with root path, actual byte count and limit. Count whitespace/BOM/newline bytes. Do not parse to discover a smaller cap after exceeding the already selected entry point cap.|
|1 encoding|Strict UTF-8 validation with no replacement decoding or skipped BOM. Correctly encoded literal U+FFFD passes encoding and ordinary Name grammar; an alternate escaped spelling still fails stage6 canonical bytes. Invalid UTF-8 =>noncanonical_bytes, root path, first offending byte offset. A valid UTF-8 BOM is not canonical JSON and fails next stage.|
|2 safe syntax/depth|Use nonrecursive bounded tokenizer/parser. Root object/array counts depth1; each nested object/array adds1, including empty containers; strings/scalars/escaped bracket characters add0. Return the first syntax/depth fault reached by increasing input byte offset: invalid JSON/trailing tokens/unpaired escaped surrogate =>noncanonical_bytes; opening container65 =>depth_capacity before descending/allocating it. This explicit encounter rule resolves syntax versus depth without recursively processing an invalid depth witness.|
|3 duplicate object members|On structurally valid bounded JSON, detect duplicate decoded member names at every object before ordinary map materialization. Return duplicate_key at the earliest repeated member occurrence by byte offset, even if the spelling differs by escape. Never last-key-wins. This is JSON member duplication, not historical entity collision.|
|4 envelope routing|Use only the selected entry point's declared header roster. Catalog/Genesis/Archive/Notebook/Transfer: kind,format_version,rules_version. Coordinator: kind,format_version,authority,rules_version. At each listed field, check presence, type/scalar grammar/width, then its declared constant before moving to the next field. Missing field=>noncanonical_bytes; wrong scalar/container type or width=>invalid_range; noncanonical scalar spelling=>noncanonical_bytes; valid unknown kind=>unsupported_kind; valid unsupported format/rules=>the corresponding existing version error; wrong Coordinator authority=>invalid_enum. The State hashing wrapper has format_version,rules_version,catalog_hash,state and no kind: only the two version headers participate here, with catalog linkage later. This does not create a State transport entry point. Receipt and embedded State have no such headers and skip stage4, inheriting their explicitly selected enclosing V3 context. Coordinator rules_version is required; its504/442 caps remain unchanged. Known V1/V2 documents are routed under their existing contract, not reclassified by this V3 supplement.|
|5 schema traversal|Walk required fields in declaration order and arrays by input index. At an object, unknown member wins first (lowest UTF-8 member name), then required fields in declaration order; missing field=>noncanonical_bytes. Check container/type, enum tag/payload, canonical scalar spelling, width/domain in that fixed order. Wrong container/scalar type or scalar width/domain/Name/Slug predicate=>invalid_range; invalid unit/tag=>invalid_enum; noncanonical decimal/hex spelling=>noncanonical_bytes; unknown member=>unknown_field. Collection size precedes visiting its children=>record_capacity. After valid child keys, repeated non-issued key (Slug, ordinal, typed composite or reference-only key)=>duplicate_key; descending unequal key=>noncanonical_bytes. Equality of issued record IDs is deferred to the global installed-identity pass in stage7; preserve exact equal-key input order during canonical encoding. No sorting/repair occurs. Issued entity records are Systems, Stars, Worlds, Lanes, Civilizations, Deposits, Facilities, ConstructionJobs, Fleets, nested Hulls, HullJobs, Routes, Shipments, Commitments, Offers, Hazards, StellarEvents and DefenseRequests. Reference/composite-key rows such as Colony, ResearchProgress, DiplomacyAgreement, War, SettlementClaim and Occupation are not additional issued records. GenesisAllocation and retained issuance origins are declarations/history, not installed live duplicates.|
|6 canonical bytes|Compare complete canonical re-encoding to original bytes. Alternative whitespace, field order, escape spelling or other valid-JSON alternative=>noncanonical_bytes, first differing byte offset and containing schema path. No normalization occurs. A domain/range failure selected in stage5 is not disguised as byte mismatch.|
|7 local structural rules|First perform one global installed-identity pass over every issued record family above, including nested hulls and cross-kind duplicates: repeated ID=>duplicate_identity at the second occurrence in schema declaration/array-index traversal. Then validate Catalog finite-mechanics equality/positive periods/resource declarations, State cross-field/local capacity/reservations/key-owner relationships and the selected initial profile. Stage5 owns structural scalar domains only; finite-mechanics equality belongs exclusively here, after canonical-byte comparison. Exact definition roster comparison names its collection path for extra/missing rows, and names the submitted row field for unequal values; policy bonuses follow their explicit schema/table fields. Use invalid_range for violated semantic/profile predicates, record_capacity for declared local slot count; select schema-ordinal/numeric-index path order within this semantic substage. Fraction/ResourceAmount family domains are exactly the Closed scale domains and ResourceAmount domains sections above, including their semantic7/replay10 ownership; widths alone do not infer them. Bootstrap richer outside_family is a fixture coverage classification, not a new wire error; a production-supported family requires its own adopted contract.|
|8 linkage/identity/history|Check exact supplied Catalog hash=>catalog_mismatch; typed reference absence=>missing_reference, resolved foreign kind=>wrong_reference_kind; catalog/topology/history graph cycles=>invalid_graph only after required typed edges resolve; unreachable/illegal selected view=>invalid_history_view, insufficient high-water=>allocator_regression. Within this stage select this listed check family first, then canonical path. This stage checks preliminary typed graph/reference/view/high-water relationships only. Historical issuance duplicate/collision checks are performed by the integrated stage10 algorithm, with the exact existing scan universe; no current-root lookup substitutes for retained history.|
|9 framed integrity|Check each declared integrity/framed ID/hash against its exact domain/payload. Mismatch=>integrity_mismatch with named field/path. Catalog has no self-integrity field: changing a valid pack creates a different PACK digest; catalog_mismatch belongs to its expected-hash consumer. No extra Catalog self-hash is introduced.|
|10 authenticated replay/evidence|Reproduce retained histories and derived original-boundary facts under one integrated bounded replay/issuance-validation algorithm after stage9 framed integrity has passed. Apply the existing batch/live-first, ascending branch-ID/ancestry, origin exemption and nonrecursive scanner rules exactly. Each emitted proposed batch checks its internal/live comparisons first, then its outer retained-history scan. A prerequisite replay reference/Arithmetic/derivation fault stops that incomplete scan immediately: missing_reference/invalid_range/replay_mismatch respectively; no incomplete absence result proves uniqueness. A duplicate/collision encountered before that fault returns duplicate_identity/identity_collision; a later collision cannot displace an earlier prerequisite failure. Structural live checks run in scanner mode; historical scans never recurse. Same origin/full frame in inherited prefixes is exempt, while distinct origins/equal full frame and equal ID/different full digest retain their distinct errors. Cache misses/eviction/continuations retain or restart exact cursors as specified, with no publication until every required outer scan completes. In particular an earlier replay derivation fault in a nonselected retained branch wins over its later transient collision; an earlier batch/live collision wins before entering that scan. State/preview/receipt derivation mismatch=>replay_mismatch; forged Anchor/witness/metric proof=>forged_evidence; structural Anchor view failure remains stage8 invalid_history_view. Validate nonselected retained branches/transient issuances too. Select original deterministic replay branch/chronology/phase/ordinal order, then fact/field order. Never perform current-hidden-root repair or migrate a witness.|

The precise stage7 subpredicate roster and stage8 linkage/identity separation should be frozen with the full validator matrix; this specification does not claim that exhaustive semantic/admission matrices already exist. Width-correct arbitrary values are not automatically gameplay-valid. Imported arithmetic with no representable valid expected result should map invalid_range at the offending source/derived operation path; a running deterministic boundary's Arithmetic fault remains its separate authority diagnostic. Workspace/output failures must name the actual declared capacity owner: do not relabel a parser allocation failure as invalid_history_view or a runtime I/O failure as unsupported_version.

**Path rule (Proposed):** JSON Pointer from root `""`, member tokens escaped `~0/~1`, array indices canonical decimal; use input indices for malformed/unsorted arrays, not a sorted reconstruction. Multi-error path comparison uses numeric indices and schema declaration ordinals, not lexical `"/10"<"/2"`. Unknown members use UTF-8 member order only as stated. Parser/UTF-8 faults use root plus bounded byte offset; missing fields name their expected pointer, even when absent. Include only enum/path/offset and typed expected/actual limit/reference details, never original unbounded input text. End offsets, capacity types and optional diagnostic presence need exact bounded schema if diagnostics themselves become persisted or hashed.
Typed read/import/job errors are a closed enum: unsupported_kind,
unsupported_format_version, unsupported_rules_version, catalog_mismatch,
noncanonical_bytes, duplicate_key, unknown_field, invalid_enum, invalid_range,
byte_capacity, depth_capacity, record_capacity, missing_reference,
wrong_reference_kind, identity_collision, duplicate_identity, invalid_graph,
invalid_history_view, allocator_regression, integrity_mismatch, replay_mismatch,
forged_evidence, stale_capture, cancelled, more_work_required, output_capacity,
store_generation_conflict, unavailable_artifact, durable_commit_completed.
Failure includes its schema path and exact capacity/reference as applicable,
never unbounded original input text. Platform-specific I/O failures remain typed
adapter outcomes; an I/O error does not become unsupported_version.

## Frozen inherited initial catalog values

This input snapshot supplies only the inherited record values mentioned above;
the mandatory V3 catalog schema, roles/workforce/stress/resource/technology
additions and new kinds replace its V2 envelope. These bytes are documentary
source evidence, not a loadable V3 pack or new acceptance vector.

```json
{
  "kind": "NYON_LIVING_GALAXY_DATA",
  "format_version": 2,
  "rules_version": 2,
  "pack_id": "nyon_living_core",
  "pack_version": 1,
  "star_archetypes": [
    {
      "id": "main_sequence",
      "name": "Main Sequence"
    },
    {
      "id": "red_dwarf",
      "name": "Red Dwarf"
    }
  ],
  "world_archetypes": [
    {
      "id": "barren",
      "name": "Barren"
    },
    {
      "id": "temperate",
      "name": "Temperate"
    }
  ],
  "resources": [
    {
      "id": "alloy",
      "name": "Alloy",
      "storage_limit": 10000
    },
    {
      "id": "energy",
      "name": "Energy",
      "storage_limit": 10000
    },
    {
      "id": "ore",
      "name": "Ore",
      "storage_limit": 10000
    }
  ],
  "industry_definitions": [
    {
      "id": "defense_battery",
      "name": "Defense Battery",
      "alloy_cost": 40,
      "build_ticks": 200,
      "slot_cost": 1,
      "max_per_colony": 1,
      "effect": {
        "type": "defense",
        "hit_points": 40,
        "damage_per_round": 4
      }
    },
    {
      "id": "extractor",
      "name": "Extractor",
      "alloy_cost": 30,
      "build_ticks": 150,
      "slot_cost": 1,
      "max_per_colony": 6,
      "effect": {
        "type": "recipe",
        "period_ticks": 10,
        "deposit_units": 4,
        "inputs": [
          {
            "resource": "energy",
            "quantity": 2
          }
        ],
        "outputs": [
          {
            "resource": "ore",
            "quantity": 4
          }
        ]
      }
    },
    {
      "id": "foundry",
      "name": "Foundry",
      "alloy_cost": 40,
      "build_ticks": 200,
      "slot_cost": 1,
      "max_per_colony": 6,
      "effect": {
        "type": "recipe",
        "period_ticks": 20,
        "deposit_units": 0,
        "inputs": [
          {
            "resource": "energy",
            "quantity": 4
          },
          {
            "resource": "ore",
            "quantity": 4
          }
        ],
        "outputs": [
          {
            "resource": "alloy",
            "quantity": 2
          }
        ]
      }
    },
    {
      "id": "shipyard",
      "name": "Shipyard",
      "alloy_cost": 60,
      "build_ticks": 300,
      "slot_cost": 1,
      "max_per_colony": 1,
      "effect": {
        "type": "hull_assembly",
        "concurrent_jobs": 1
      }
    },
    {
      "id": "solar_array",
      "name": "Solar Array",
      "alloy_cost": 20,
      "build_ticks": 100,
      "slot_cost": 1,
      "max_per_colony": 6,
      "effect": {
        "type": "recipe",
        "period_ticks": 10,
        "deposit_units": 0,
        "inputs": [],
        "outputs": [
          {
            "resource": "energy",
            "quantity": 8
          }
        ]
      }
    }
  ],
  "hull_definitions": [
    {
      "id": "colony_ark",
      "name": "Colony Ark",
      "alloy_cost": 60,
      "build_ticks": 200,
      "hit_points": 20,
      "damage": 0
    },
    {
      "id": "escort",
      "name": "Escort",
      "alloy_cost": 20,
      "build_ticks": 100,
      "hit_points": 10,
      "damage": 2
    },
    {
      "id": "scout",
      "name": "Scout",
      "alloy_cost": 10,
      "build_ticks": 50,
      "hit_points": 10,
      "damage": 0
    }
  ],
  "hazard_definitions": [
    {
      "id": "ion_storm",
      "name": "Ion Storm",
      "effect": {
        "type": "freight_batch_scale",
        "numerator": 1,
        "denominator": 2
      }
    }
  ],
  "policy_definitions": [
    {
      "id": "expansionist",
      "name": "Expansionist",
      "bonuses": [
        {
          "intent": "build_ark",
          "bonus": 20
        },
        {
          "intent": "settle",
          "bonus": 20
        }
      ]
    },
    {
      "id": "guardian",
      "name": "Guardian",
      "bonuses": [
        {
          "intent": "build_battery",
          "bonus": 20
        },
        {
          "intent": "build_escort",
          "bonus": 20
        },
        {
          "intent": "defend",
          "bonus": 20
        }
      ]
    },
    {
      "id": "industrialist",
      "name": "Industrialist",
      "bonuses": [
        {
          "intent": "build_extractor",
          "bonus": 20
        },
        {
          "intent": "build_foundry",
          "bonus": 20
        }
      ]
    },
    {
      "id": "neutral",
      "name": "Neutral",
      "bonuses": []
    },
    {
      "id": "trader",
      "name": "Trader",
      "bonuses": [
        {
          "intent": "establish_trade_route",
          "bonus": 20
        },
        {
          "intent": "supply",
          "bonus": 20
        }
      ]
    }
  ]
}
```


## Integrated planning and admission, 2026-10-02

These amendments integrate the scoped reviewed scoring corrections and remain
Proposed pending full package review. Scoring §§1–8 owns exact numeric weights,
200-boundary counterfactuals, four category predicates, policy order, complete
enumeration, checked arithmetic, force selection, central ledger and stable tuple.
The field/tag registry below owns their canonical representation. No numerical
choice is delegated to a codec.

On every50-tick decision boundary capture all actors' permitted planning records
from the same immutable post-phase7 authority input. Candidate planning is logical work that
can prepare both phase8/9 lanes before admission without installing phase9 actions.
Phase8 on multiples100 and
phase9 share these captures; negotiations/reservations do not recapture/rescore.
All capture references have phase7; Decision.phase records actual8/9 execution.
Central reservation protects category0 plans globally before other material work,
then considers older pending offers in recipient/offer order against remaining
funds/capacity. Attempts use exact scoring queue/merge order and recorded reasons.
New permissions affect actual phase10 dispatch but not this boundary's forecast.
Free phase8 demand postings are derived from their owner's phase7 facts, but newly
posted/refreshed records are not retroactively inserted into other actors' captured
planning inputs. Central real offer response may validate current demand/eligibility
against actual state and ledger without changing any actor's enumerated score.
Own demand posting is free protocol maintenance, never a material category0 debit.

At due needs boundaries phase5 first completes jobs, then holds actual local
Food/energy based on population at phase5 production entry and due phase6 needs.
No population-changing operation occurs between those entries. Held=min(stock,need)
for each resource; absent colonies/non-due/zero population hold0. Guard all due
industry/research/repair/military-supply debits; solar/no-input output needs no
waiver. Recompute holds after actual guarded stock changes. A held-energy Food
recipe executes atomically only when exact next Needs exposure decreases and no
previously fully funded resource becomes short; guard facts record the physical
stock/hold/required amount. Denials leave inputs untouched and use the operation's
existing blocked-clock rule. Supply denial records the latest attempt at that global100-tick supply boundary
with outcome unavailable/paid0; the next global100 boundary applies (no additional
Supply clock field), preserving prior successful payment/deadline and
readiness. Hold state is transient, not canonical escrow. Phase6 consumes actual
stock under the unchanged needs/demographic formulas; release holds after needs.

## Integrated scoring, admission and privacy payloads

All declarations use frozen scalar/ID/nullable/sort/JSON grammar. They replace
earlier partial scoring overrides; canonical state roster remains unchanged.
Candidate { action:IntentAction, admission_terms:AdmissionTerms,
category:DecisionTier, deadline:Option<Tick>, components:ScoreComponents,
score:Score, tie_key:TieKey }. DecisionTier is imminent_needs_recovery | recovery | obligation | ordinary,
in category order0..3.
ScoreComponents is a closed tagged enum, type first:

- economy: delta_first_exposure:Score, delta_all_exposure:Score,
  delta_shortage_loss:Score, delta_capacity_deficit:Score, delta_food_produced:Score,
  delta_energy_produced:Score, delta_ore_produced:Score, delta_alloy_produced:Score,
  delta_research_points:Score, public_request_coverage:Score, role_value:Score,
  policy_bonus:Score, energy_paid:Score, ore_paid:Score, alloy_paid:Score, travel_ticks:Score.
- offer: kind:TreatyKind, batch:u32, reachable_worlds:u32, threatened_worlds:u32,
  min_relation:u32, policy_bonus:Score.
- war: kind:ObjectiveKind, hostile_escorts:u32, occupation:bool, own_population:u32,
  accessible_worlds:u32, shortage_route_worlds:u32, own_routes:u32,
  completed_facilities:u32, own_route_endpoints:u32, defense_strength:u32,
  travel_ticks:u32, policy_bonus:Score.

Economy is mandatory for non-offer/non-war material candidates. Offer Supply
sets batch, Access reachable_worlds, Defense threatened_worlds; other two count
fields are0. min_relation and policy_bonus are required for all three. War Defend
sets hostile_escorts/occupation/own_population; Access accessible_worlds/
shortage_route_worlds/own_routes; Occupy completed_facilities/own_route_endpoints/
defense_strength. Irrelevant war counts are0 and occupation=false. Every war
sets travel_ticks and policy_bonus; null-force defender travel is0 and its audit
explicitly force_unachievable. Raw components are unweighted; scoring §§4/5 gives
the exact weights and U. No family puts offer/war utility into economy.role_value
or a guessed PolicyScore bucket. Type/kind are tags, not numeric components.

AdmissionTerms is a closed tagged enum with type first and these exact variants:

| type tag | Required payload fields in order |
| --- | --- |
| construction | world:ID, definition:Slug, scrap:Option<ID>, alloy_paid:u32, slot_count:u32 |
| hull | world:ID, definition:Slug, target_fleet:Option<ID>, alloy_paid:u32, reserve_new_fleet:bool |
| route | source:ID, destination:ID, resource:Resource, existing_route:Option<ID>, kind:RouteKind, batch_size:u32, cadence_ticks:u32, source_reserve:u32, source_owner:ID, receiver_owner:ID, commitment:Option<ID>, next_due:Tick |
| movement | fleet:ID, kind:FleetOrderKind, target:ID, lane_path:Vec<ID>, departure_not_before:Tick, departure_payments:Sorted<ResourceAmount>, objective:Option<ObjectiveKey>, defense_request:Option<DefenseActionBinding> |
| board | hull:ID, world:ID, count:u32 |
| offer | proposer:ID, recipient:ID, kind:TreatyKind, terms:Terms, demand:Option<DemandKey>, renewal_of:Option<ID> |
| war | peer:ID, declarer_objective:WarObjective, declarer_force:ForceWitness |
| research_target | technology:Slug |
| agreement | peer:ID, kind:AgreementKind |

Construction/Hull/Route/Explore/Move/Settle/Defend/Attack/Return/Blockade/Board/
Offer/War/SelectResearch/Agreement map only to their corresponding AdmissionTerms
variant; motion variants use movement, blockade target is its known world, and
settlement moves its already-existing passengers. ResourceAmount here means
resource slug/quantity under the frozen exact catalog. Cadence/source reserve/
funding must satisfy source formulas and physical account checks. New-route
source_reserve is max(20,known next Food/energy need) for needs resources,20 for
others; enumerate batches1..10, not every arbitrary reserve integer. No new
resource-cost or period is selected through these terms. Paid/treaty source/receiver
ownership must equal captured eligible participants. An offer uses the Offer
record's deterministic prospective ID only when its successful ordinal is0
under the one-outgoing rule. Audit identity includes that prospective action ID
and full terms; its derivation excludes audit/receipt/current-event digests.
Considering alternatives consumes no allocation; only accepted issuance does. War objectives begin
pending/count0/witness null. Hull target null iff reserve_new_fleet true.

ForceWitness { fleet:ID, hulls:Sorted<ID>, departure_world:ID,
arrival_endpoint:ID, lane_path:Vec<ID>, travel_ticks:u32,
required_departure_payments:Sorted<ResourceAmount> }. Its sorted hulls are the
entire live fleet roster, payments use the exact departure-world inventory;
path order is traversal order, not sorted ID order. Identity includes this entire
witness. War AdmissionTerms contains only declarer's mandatory force witness;
the central-admission attached defender objective is absent from declarer identity
and is not a private defender-force grant.
DefenderObjectiveAudit { actor:ID, snapshot:SnapshotReference,
objective:WarObjective, force:Option<ForceWitness>, force_unachievable:bool, components:ScoreComponents,
score:Score }. force null iff force_unachievable true; an eligible variant
has false and nonnull force. The defender_objective_audit EventKind/Facts is
`{record:DefenderObjectiveAudit}` in this schema. The defender computes
this audit under its own permitted snapshot; its private witness does not enter
declarer Candidate identity/utility. Actor-filtered snapshots expose only the
public negotiated objective, never the private audit or its local stock accounts.

WarAdmissionAudit { declarer:ID, declarer_snapshot:SnapshotReference,
candidate_identity_bytes:String, defender:ID, defender_snapshot:SnapshotReference,
defender_selection:Option<DefenderObjectiveAudit>, installed_war:Option<Pair> }.
The war_admission_audit EventKind/Facts is `{record:WarAdmissionAudit}` and DecisionReason has the unit tag no_valid_defender_objective. candidate_identity_bytes
recomputes the attempted scoring §3 identity unchanged. defender_selection null iff no
valid defender objective was available; installed_war is nonnull only on actual
successful atomic admission. The full audit is authority/creator/replay-only, since it contains BOTH private
actor captures and candidate/selection fields. Each actor sees its own CandidateAudit
or DefenderObjectiveAudit plus successful public War objectives, not this full
record and not a redacted canonical copy.
These admission records are outside enumeration digest and contain no receipt or
future EventID. Subsequent War state contains both objectives as this schema
requires; no intermediate one-objective War state is valid.

TieKey { category:u32, urgency_deadline:Option<Tick>, score:Score,
lane:u32, family_rank:u32, offer_kind_rank:u32, primary_target:Option<ID>,
counterpart:Option<ID>, actor:ID, identity_bytes:String }. identity_bytes is the
scoring §3 canonical action+admission_terms JSON encoded as UTF-8 text with canonical JSON
escaping. Validation recomputes each field. Comparison is exactly scoring §7: score
descending, all other scalars/IDs/bytes ascending, absent options last. No second
offer comparator or action-tag shortcut is allowed. War-family local queue uses
its explicit objective-class/force tie before exposing its head; the audit records
that class and sequence. This supplement replaces the earlier incompatible TieKey.

CandidateAudit { actor:ID, lane:u32, snapshot:SnapshotReference,
enumerated_count:u64, trace_digest:Digest,
family_bests:Vec<Candidate>, attempts:Vec<CandidateAttempt> }.
CandidateAttempt { candidate:Candidate, outcome:DecisionOutcome,
reasons:Vec<DecisionReason> }. Family_bests uses the literal roster family order
in scoring §3; attempts preserves actual central admission order. The audit hash inputs
are the exact context and eligibility trace records declared in scoring §7, never this audit's
own digest or Receipt/Event IDs. Derived CandidateAudit and each attempted action
are exact replay outputs. Receipt EventKind contains candidate_audit with Facts
`{record:CandidateAudit}`; it does not suppress existing action/denial events.

NeedsReserved blocked reason fields in exact order: world:ID, resource:Resource,
physical_stock:u32, held:u32, required:u32. held<=physical_stock, available funds
are physical_stock-held, and required exceeds that amount; the protected due
needs result and category guard are recorded. This distinct reason does not claim
physical inventory is missing. BlockedReason and DecisionReason both include needs_reserved with these
exact fields; RecipeFacts.blocked can carry it. Guard facts accompany
due supply/repair denial where no RecipeFacts exists.


NeedsGuardOperation = facility { id:ID } | repair { fleet:ID } | supply { fleet:ID }
 | hub_fallback { world:ID } | battery_repair { facility:ID }.
Tagged JSON has type first, then the sole payload field; variant order is the
order above. Facts.needs_guard retains operation/reason/outcome field order.
NeedsGuardOutcome = denied | food_waiver. A denied guard consumes no inputs; its
NeedsReserved also appears in the corresponding producer's actual blocked-reason
carrier where one exists; never fabricate RecipeFacts for Hub/repair/Supply. A food_waiver is only an otherwise
valid atomic Food-producing facility recipe meeting scoring §6's exact exposure/
funded-payment predicate; it emits this guard fact plus ordinary produced/resource
facts, with no blocked reason on the successful recipe. Required/held/physical_stock
refer to pre-recipe values; replay computes the exact without/with Needs comparison,
never claims the hypothetical comparison is an already-paid needs result.

### Exhaustive Needs guard subject, producer and privacy mapping (Proposed)

| Operation | Typed subject and local account linkage | Genuine producer and companion cardinality | Legal outcomes |
| --- | --- | --- | --- |
| facility{id} | Facility kind7; Facility.world equals reason.world kind3; catalog effect is exact due recipe | phase5 periodic production/lab; guard immediately before primary facility event; guarded ordinary recipe has two events, successful Lab additionally retains its existing research event | denied; food_waiver only exact positive Farm waiver |
| repair{fleet} | Fleet kind10; exact eligible damaged Hull selected by global entity-ID traversal; Docked World equals reason.world | phase5 Hull repair; one guard then one operation_changed with containing Fleet Change; no invented Hull field or per-Fleet aggregation | denied |
| supply{fleet} | Fleet kind10; owned Docked World equals reason.world; exact live Hull count determines required Energy | phase5 Supply100; one guard then fleet_supply_unavailable | denied |
| hub_fallback{world} | World kind3; operation.world equals reason.world; exact World Hub fallback branch, not a Facility | phase5 due Hub Alloy fallback; exactly one needs_reserved, no RecipeFacts or generic duplicate | denied |
| battery_repair{facility} | Facility kind7; catalog Defense effect and Facility.world equals reason.world | phase5 battery repair; one guard then one operation_changed with battery Facility Change | denied |

Every row uses actual phase5 pre-group full State source/latest creator cursor and
null actor under §Production phase5 actual ownership and reports. This is neither
AUTO allocation nor a new successful allocation ordinal. All exact producers use
Energy guards; Resource remains the carrier type. All reasons retain field order
world/resource/physical_stock/held/required. Farm waiver has no successful blocked
reason. Hub's single event, repair's paired event, Supply's paired denial and Lab's
additional research carrier remain their distinct genuine producers.

Operation IDs and reason.world resolve to the typed records above in authenticated
producer-prefix replay, never a current resident State after removal. Under §V3
decoder stages, stage8 checks only supplied typed graph/history-locator evidence
available WITHOUT executing/authenticating a producer. Truly preliminary supplied
edge absence/foreign kind retains stage8 missing_reference/wrong_reference_kind;
opaque IDs reveal no kind bits. Stage9 framed integrity precedes all historical
producer reconstruction. Stage10 establishes original-prefix membership/kinds,
operation/world/local-account/ownership links, selected Hull, amounts and genuine
producer cardinality under the existing integrated replay order. A missing
prerequisite replay reference remains stage10 missing_reference; reproduced facts
differing from claimed subject/kind/link/amount/cardinality use stage10
replay_mismatch. Existing stage10 Arithmetic faults retain invalid_range. Never
backdate a replay-discovered fault to stage8 or substitute current resident State.
Repair authenticates the selected Hull from traversal and exact Fleet Change;
no new portable Hull reference or diagnostic is introduced.

For ALL five rows, the full canonical guard contains private stock/hold quantities.
Authority, creator and authenticated replay may read the full carrier. Own actor
inspection may read it only when authenticated producer-prefix World.owner for
reason.world equals that actor, with the corresponding owned Facility/Fleet links
verified for the row. Remove the entire guard for every other actor; never redact
canonical Event bytes or add guard stock/hold data to Actor32. Present ownership,
current lookup, direct sight or receiving cargo grants no historical private data.
Missing historical proof fails closed. This finite historical ownership rule is a
Proposed clarification for independent review, not a pre-existing classifier;
no new public safe-notice vocabulary is introduced.

DeclarationHold { actor:ID, snapshot:SnapshotReference, candidate_identity_bytes:String,
fleet:ID, world:ID, resources:Sorted<ResourceAmount>, released:bool }. Emit a hold
fact with released=false after validation and a matching released=true fact at
rejection/publication; no resource debit/refund is inferred from either. Those
facts may only refer to the declarer's own ForceWitness funding. They contain no
private defender fields. All audit/event traversal shares the existing visit budget.

Candidate identity/terms text and trace records are canonical JSON byte strings,
not Slug/Name; bounded by existing8 MiB receipt and depth64 before allocation.
Each audited family_bests includes only actual eligible bests in roster order;
absent families are omitted, never fabricated null candidates. trace records
order fields action_tag,action,admission_terms,candidate with nonnull Candidate
as scoring §7 specifies. No ineligible diagnostic affects count/trace bytes. trace_digest hashes its context and length-prefixed records, excluding
own hash and all current Receipt/Event IDs. PolicyScore records every numeric component's raw value in the tagged family's
field order, including zeros; kind/type tags are not numeric reasons, occupation
bool records0/1. No optional Forecast reason enters this exact material-attempt
reason vector; complete signed economy deltas remain in components. Forecasts
remain counterfactual, not witnesses.

Authority receipts retain all typed audits for omniscient creator/replay. Actor
projections filter private audits: own CandidateAudit only; DefenderObjectiveAudit
only its actor; full WarAdmissionAudit only omniscient creator/replay, never either actor.
Defender sees its separate own DefenderObjectiveAudit; declarer sees its unchanged candidate/attempt/outcome and successful public objective
through War state/war-created change fact, not a redacted canonical audit blob or
private digest. Filtering never edits canonical bytes or grants hidden root facts.
WarAdmissionAudit.defender_selection may be nonnull with installed_war null on a
later atomic guard/capacity failure; null means no valid selection was available.
Full package review must validate bounded enumeration/audit output, and full gap6
remains open. No completed shape/text check establishes runtime/performance proof.


## Integrated correction I1: permitted environmental forecast facts

KnownLaneHazard and KnownEnvironmentEvent are immutable positive facts. Their
canonical observation containers are COMPLETE scope assertions, never partial
lists: StellarScopeObservation covers one named system's entire observable
stellar schedule; HazardScopeObservation covers one named lane's entire observable
hazard schedule. The closed records above have no completeness toggle. An empty
events array is an explicit negative assertion at that scope's original observed
tick, not absence inferred from missing data. Each child's originating_observer/observed_tick equals its
container's, and system/lane equals its scope. Events sort by ID without duplicates;
warning_tick<=observed_tick<end_tick and warning<=start<end. A direct observation
copies ALL root events satisfying these conditions in the authorized scope,
including warnings not yet active. Hidden future warnings are outside the scope's
observable schedule and their omission says nothing about their future existence.
No star activity clock/seed or unrelated root placement is disclosed.

At phase7 DirectVisible(A,w) is exactly the Actor32 source registry predicate,
computed from validated base rows before producing observations, never from
submitted Observation rows. Its fresh EnvironmentObservation
contains exactly one complete stellar_scope for the world's system at T. Its
ObservationFacts.hazard_scopes contains exactly one complete assertion for EVERY
known lane incident to that directly visible world, sorted by lane ID, including
empty assertions. Known lane means a lane already in that actor's permitted
known_lanes; observation never reveals an unknown remote lane. All scope observers
are the observing actor. Stellar facts copy exact star/system, warning/start/end,
habitability impact and creator-edited fraction; hazard fraction resolves from its
immutable catalog definition. Copying shared intelligence preserves every scope
and child byte, observer and tick; received_tick grants no freshness. Requests
copy the same immutable ObservationFacts. No partial scope record is valid.

**Proposed visibility refinement:** traversing a lane with an own fleet alone
no longer refreshes environmental schedule facts. Fresh lane schedule visibility
requires an incident directly visible world as above. Fleet movement/location
and existing combat sight remain unchanged. This replaces round1's unsupported
fleet-only schedule read, supplies a canonical existing World observation carrier,
and creates no new implicit fleet cache or root read during scoring. Stale/shared
world observations and unrelated worlds grant no fresh scope read.

Each stellar_scope.events and each hazard scope.events caps128; hazard_scopes
caps256 (topology lane cap), with at most one entry per incident known lane. The
observation's stellar system must match its observed world system. Every scope
observed_tick equals the enclosing original world observation observed_tick, never
received_tick, and is <=capture T. Its originating_observer is the original direct
observer (Observation actor, SharedIntelligence.originating_actor or request
originating_observer respectively). Replay validates complete membership against
that authenticated observation boundary, not the current hidden schedule. Scope
system/lane references must resolve through the permitted known topology. Existing source
record/visit/byte limits still apply. A scope assertion does not expire with its
last event: retain empty coverage and its timestamp as long as the owning
observation/shared/request record is retained. New direct world refresh replaces
its full fact container; sharing retention follows existing record bounds. Removal
of the last permitted scope record also removes its negative knowledge. No hidden
persistent tombstone or authority-root cancellation list is inferred.

Actor capture collects scope assertions from ALL its permitted direct/own,
shared-intelligence and DefenseRequest observation records before any whole-world
newest-observation projection. Shared same-system facts from a different world
therefore compete with the actor's fresh system observation. For EACH family and
scope ID choose greatest observed_tick, then lowest original observer ID, then
lexicographically lowest canonical complete scope bytes. This single tuple governs
positive/empty conflicts alike; own sources have no extra priority. Equal-tick
contradictions retain original source records for inspection but only the winning
scope enters forecasting. Never union event arrays of losing scope assertions.
Missing scope assertion grants no negative evidence for that scope or any other.

known_stellar_scopes (cap64, sorted by system) and known_hazard_scopes (cap256,
sorted by lane) in the actor payload contain these winners INCLUDING empty ones.
Their bytes enter permitted_snapshot_digest. Flatten ONLY winning scopes, INCLUDING
all retained positive children whose recorded end_tick<=T. First choose exactly one
child per event ID by greatest observed_tick, then lowest originating_observer,
then lexicographically lowest canonical child bytes. ONLY AFTER identity selection
drop the chosen child when its recorded end_tick<=T. Never retry an older losing
child after expiry. This order applies separately to stellar and hazard families.
The actor's known_stellar_events/known_hazards are those flattened winners sorted
by ID, each cap128 distinct IDs. Thus a fresh empty scope suppresses every older
positive copy of that scope regardless of originating world/source; a fresh move
wins event identity over a stale other scope; an unrelated empty scope never
suppresses a positive. The mandatory order is scope supersession, event-ID newest-child selection
INCLUDING expired children, selected-child expiry filtering, then actor positive
roster cap checks. If either resulting roster
exceeds128, fault unpublished capture with record_capacity at actor/roster path;
never truncate known modifiers or scan root for hidden cancellation. The preceding
boundary remains valid/editable. This Proposed capture availability limit remains
explicit and requires capacity fixtures.

For forecast, apply selected positive facts only in recorded start<=tick<end.
A known stellar fraction applies to a lane with that system as an endpoint; a
known hazard only to its named lane. Apply active modifiers in event-ID order to
current integer batch. Habitability uses the same selected stellar fact. An unseen
cancellation/edit leaves a stale counterfactual fact until permitted newer coverage
supersedes it or recorded expiry passes; no scoring-time root scan is permitted.
Expiry filtering changes only the derived positive roster, never retained scope
contents. A later complete refresh replaces that source's scope, and source eviction
removes its scope from the retained inputs: these can remove the last newer child
and allow a still-retained older other-scope record to win. That is explicit loss
of retained knowledge, distinct from filtering an expired child still present in
a winning scope; no permanent event-ID tombstone is added. A newer empty scope
suppresses older children only in its own scope, not globally by identity.
All completeness, visibility, negative-evidence and ordering choices remain Proposed.

Written cases: shared world Q/system S at10 contains E/end200; direct world P in
S at20 has empty scope, so E is absent at capture20. Empty system R at20 leaves
S/E intact. Hidden cancellation without any newer S observation retains S/E.
Moving E from visible S to hidden R yields fresh empty S and removes the stale
forecast without disclosing R. Fresh R/E at20 with stale S/E at10 selects R/E.
Same-tick empty and positive S assertions choose lower observer ID, then canonical
scope bytes, identically for every actor given the same permitted records. A
missing hazard scope for unrelated lane L2 says nothing about L1. Empty winners
remain in capture hash even if both positive rosters are empty.


## Integrated correction I2: actor trace context and outer authentication

ActorInputContext { domain:u32, catalog_hash:Digest, branch:BranchID, tick:Tick,
phase:u16, permitted_snapshot_digest:Digest }. Copy these fields from the actor's
phase7 SnapshotReference, in that order. Exclude authority state_digest and
revision: a hidden creator edit may change them without changing permitted facts.
CandidateAudit trace hash is SHA256(NYON_LIVING_V3_CANDIDATE_AUDIT+NUL || actor16 ||
lane:u32le || L(C(ActorInputContext)) || each L(C(CandidateTrace))). L is u32le byte
length. Its outer CandidateAudit still carries the FULL SnapshotReference; replay
validates it against that authority branch/revision/tick/State and recomputes the
permitted payload/context/trace. The authenticated Receipt includes full audit
bytes. Equal actor/branch/catalog/tick/permitted inputs yield identical trace
records/digest despite differing hidden State/revision; outer authority audits
and Receipt/Event digests may differ. Do not redefine State digest or pretend
that the full provenance-bound receipt must be invariant. No enclosing audit,
receipt or event digest enters ActorInputContext or its trace.


## Integrated correction I5: creator fleet-order objective binding

SetFleetOrder's required objective and defense_request are explicit even when null.
kind=null requires target_world=null, lane_path empty and BOTH bindings null,
and sets order=null. A
non-null kind builds the exact order from all submitted fields and both bindings;
null objective clears any old binding, nonnull replaces it. No implicit preservation
or inferred War objective is permitted. Return/Explore/Settle/Blockade require
objective=null. Attack/Defend/Move may bind only as follows: ObjectiveKey names an
existing ascending War pair, actor=fleet.owner and that actor's valid pending
objective. Defend objective binds Defend at its named owned target; Occupy binds
Attack at its named enemy target; Secure-access binds Move or Attack at one of its
named corridor endpoints. Attack still needs declared war/normal target permission;
Move still needs actual movement permission. All underlying order/path/clock/
capacity validation remains required. An incompatible binding rejects the entire
batch, consumes no IDs/funds and leaves the Fleet byte-identical.

EVERY genuine SetWarObjective replaces that exact objective instance even for
byte-equal submitted kind/target/pair, resets pending/count0/evaluated null/witness
null and clears the entire War.peace. It cancels every Fleet order bound to that
actor/pair; it does not silently retarget it. ForcePeace/war expiry/removal cancels
all orders with that pair and clears any Blockade with that pair. reviewed_effects
must contain each exact before/after Fleet and these cancellations. Already-paid
location/leg, manifests, hull HP/passengers, return credits and consumed resources
remain unchanged; clearing future order/Blockade never refunds or teleports.
Creator order edits/cancels show objective and actual binding validation/cascades
in the typed preview. They cannot manufacture a military fulfillment witness.
These explicit Proposed field/cancellation choices complete the inherited command.


## Integrated correction I6: coordinator pre-parse byte boundary

Coordinator canonical input cap is504 UTF-8 bytes, derived from its exact fixed
kind/authority/version strings, field names/order/punctuation, slot/branch32hex,
catalog/integrity64hex, nonnull revision64hex, and20-digit Tick/generation maxima.
All are ASCII. Replacing nonnull quoted revision66 bytes with null4 gives maximal
null-revision width442. Digest content does not alter its64-byte width. This is
a Proposed derived schema envelope maximum, not an arbitrary KiB allowance.
Check raw byte length BEFORE UTF8/JSON decoding, nesting/schema/version/kind or
integrity checks. Any505+ bytes returns byte_capacity even if malformed, unsupported
kind/version or invalid UTF8; preserve resident and prohibit old-slot fallback.
At<=504 apply existing canonical grammar/closed errors/integrity/view validation.
Whitespace/newline bytes count toward the cap and never become canonical. Export
and store reads use the same boundary. No other document cap is changed.


## Integrated correction I4: eligible trace and separate central reasons

CandidateTrace { action_tag:Slug, action:IntentAction, admission_terms:AdmissionTerms,
candidate:Candidate }. All fields must agree with the complete identity and
Candidate; candidate is nonnull. CandidateAudit.enumerated_count counts only
these emitted eligible deduplicated records, in scoring family order then identity
byte order. ineligible/eligible/reasons fields are not permitted in CandidateTrace.

Candidate formation uses only exact captured own records, finite enabled catalog
definitions and observed/party-permitted counterpart/target sets in scoring §3.
Construction enumerates each actual colony/definition/no-scrap or actual legal
scrap; hull each actual shipyard/definition/actual eligible target or one new-fleet
reservation; route each observed direct endpoint/resource/matching existing route
or new variant with batch1..10; movement each actual owned fleet/known target with
its uniquely chosen permitted path; board exact existing ark/origin/count100;
offer exact party/kind/formed terms under a live permitted demand/access/threat;
War own objective/whole eligible force/endpoint with exact path. These finite
inputs never include unknown IDs, guessed catalogs, missing paths or unformed
terms. Before trace, require all structural/reference/catalog/ownership/path/
intelligence/supply/staff/physical-funds/capacity/needs predicates in the rules
against the SHARED captured state, including actual scrap refunds. Failure emits
no trace/Decision/ID and no canonical local diagnostic. No-scrap full-slot option
is excluded; viable scrap alternatives are tested individually, not suppressed.
Deduplicate full formed action+terms before scoring/hash. A scoring arithmetic
fault aborts the unpublished boundary; it is not an omitted candidate. Nonpositive
ordinary U is still a legal scored trace record but cannot beat Wait0/admit.

Central-attempt outcome reasons are a DIFFERENT exact contract. Accepted material
attempt/Decision reasons contain PolicyScore for every numeric component in the
tagged component field order (raw unweighted value, including0, occupation0/1,
policy=captured actor policy). Rejected reasons are exactly ONE failed guard: the
first failed stage in the ordered table below, then lowest subject ID, then lowest
canonical reason bytes if multiple violations remain in that stage. No score
reason, optional Forecast, extra local diagnostic or insertion-order reason is
appended to a rejection. Candidate retains its exact scored components separately.
Captured-state ineligibility never creates a central attempt merely to log a reason.

| Central stage order | Canonical failure reason selection |
| --- | --- |
|1 reference/definition/order compatibility | invalid_target { target:subject, rule:reference_missing/wrong_reference_kind/definition_absent/order_incompatible } |
|2 permitted intelligence | missing_intelligence {world} or stale_observation {world,observed_tick,maximum_age:300} |
|3 actual ownership/permission/path | invalid_target { target:subject, rule:ownership_changed/permission_changed/path_unavailable } |
|4 fleet readiness | supply {fleet,readiness} |
|5 workforce | workforce {world,required,available} |
|6 protected needs | needs_reserve {world,food,energy}, where amounts are baseline next due needs at the earliest harmed funded payment; ties world ID |
|7 capacity | admission_capacity { capacity:AdmissionCapacity, subject:ID, limit:u32, reserved:u32 } |
|8 physical local funding | insufficient_resource {world,resource,required,available,reserve}; subject is the affected world, resource tag then canonical bytes resolves its multiple accounts |
|9 incompatible reservations | target_reserved {target,winner}, winner the already admitted actor ID; target is the exact actual fleet/hull/world/route asset |
|10 private defender selection | no_valid_defender_objective |

Each guard uses actual central state/ledger, not hidden input for scoring. Predicate
parameters/ranges come from this package's exact rules; stages never invent costs.
Subject is the affected actual issued ID. A local capacity failure names its
world/shipyard/target-fleet account; a global collection capacity names the
Candidate's actor ID. Thus local hull slots and global hull capacity are not
unidentified accounts; absent/fabricated IDs are never substituted as zero IDs.
AdmissionCapacity = industrial_slots | facilities | construction_jobs | hull_jobs | fleets |
hulls | routes | shipments | offers | agreements_commitments | defense_requests |
settlement_claims. Unit tags are closed; among multiple capacity failures compare
this listed order before subject/reason bytes. limit is the published corresponding
cap and reserved its actual occupancy+holds. The new DecisionReason tagged variant
admission_capacity has precisely capacity,subject,limit,reserved in that order. Other stage
reasons use their existing exact payloads. Stage1/3 rule slugs are a closed registry
listed in the table, not arbitrary text. Stage2 missing precedes stale at the same world/subject; within
stage8 lowest affected world/subject ID, then resource tag,canonical reason bytes
applies. required is the full atomic debit, available is current ledger stock and
reserve is its required protected/source floor, all actual local quantities. Successful/rejected
CandidateAttempt and its Decision must have identical reason bytes/outcome; free
research/maintenance retains its separately specified policy/protocol facts.

Absent path/unknown target/multiple local eligibility failures produce identical
zero-record exclusions without undefined diagnostic ordering. Full-slot replacement
produces every distinct viable replacement identity. A later global reservation
loss changes only attempt evidence, never the eligible trace digest/count/order.
All skipped checks/enumeration work still consume the common record-visit budget;
eligible-only hashing is not permission to hide traversal cost or truncate records.


### I5 exact edit/cancel/peace example contract

Let F be the complete valid Fleet record with id=f,owner=a,location=L,
order={kind:attack,target_world:w,not_before_tick:t,lane_path:P,
objective:{war_a:a,war_b:b,actor:a},defense_request:null},hulls=H,supply=S,blockade=null.
Here a<b, that actual War's a objective is pending Occupy w, and L/H/S/P are the
original canonical validated values. These symbols stand for their entire input
bytes, not omitted defaults. Complete return-after record is
{id:f,owner:a,location:L,order:{kind:return,target_world:origin,
not_before_tick:t2,lane_path:R,objective:null,defense_request:null},hulls:H,supply:S,blockade:null},
where origin/R/t2 are submitted validated return parameters. Complete cancel-after
and ForcePeace-after record is
{id:f,owner:a,location:L,order:null,hulls:H,supply:S,blockade:null}.
For any other valid Fleet G with Blockade.war_a=a,war_b=b, ForcePeace clears that
Blockade; if G.order is bound to the pair it also becomes null, otherwise its
permitted unbound order bytes stay exactly G.order. All other full G fields are
byte-identical. Each delta is an explicit fleets Change before/after Record in
reviewed_effects, ordered by TypedKey. Replay must recompute the same complete
records and compare them; an old binding left on Return/cancel or a changed L/H/S
rejects the preview/replay, rather than being silently repaired. Independent
populated valid/invalid fixture construction must establish these stated reference/
order preconditions before deriving exact canonical bytes. This written equality
contract is not claimed as an executed or generated authority fixture.
