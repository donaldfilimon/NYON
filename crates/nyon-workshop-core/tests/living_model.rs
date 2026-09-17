//! Validation obligations of the Living Galaxy V2 authoritative state schema.
//!
//! Two families of test live here. The first walks every collection maximum in
//! the section 1 limits table at its limit and one over it, because a bound
//! that is only ever tested far from its edge is a bound nobody has checked.
//! The second mutates one reference or one declared range at a time, so a
//! failure names the invariant that broke rather than "the state is invalid".

use nyon_workshop_core::living::{
    LIVING_MAX_CIVILIZATIONS_V2, LIVING_MAX_DEPOSITS_V2, LIVING_MAX_FLEETS_V2,
    LIVING_MAX_HAZARDS_V2, LIVING_MAX_HULLS_PER_FLEET_V2, LIVING_MAX_HULLS_V2,
    LIVING_MAX_INDUSTRIES_V2, LIVING_MAX_LANES_V2, LIVING_MAX_ROUTES_V2, LIVING_MAX_SHIPMENTS_V2,
    LIVING_MAX_STARS_V2, LIVING_MAX_SYSTEMS_V2, LIVING_MAX_WORLDS_V2,
    LIVING_RESOURCE_STORAGE_LIMIT_V2, LivingAgreementKindV2, LivingAgreementV2,
    LivingCivilizationStatusV2, LivingCivilizationV2, LivingColonyV2, LivingConstructionJobV2,
    LivingDepositV2, LivingEntityIdV2, LivingFacilityStatusV2, LivingFacilityV2,
    LivingFleetLocationV2, LivingFleetOrderKindV2, LivingFleetOrderV2, LivingFleetV2,
    LivingFreightRouteV2, LivingGalaxyStateV2, LivingHazardV2, LivingHubV2, LivingHullJobV2,
    LivingHullV2, LivingInventoryV2, LivingLaneV2, LivingNameV2, LivingObservationV2,
    LivingOccupationV2, LivingPolicyV2, LivingRelationReasonsV2, LivingRelationV2,
    LivingResourceV2, LivingRouteKindV2, LivingSettlementClaimV2, LivingShipmentDispositionV2,
    LivingShipmentV2, LivingSlugV2, LivingSortedVecV2, LivingStarV2, LivingSystemV2, LivingTickV2,
    LivingValidationErrorV2, LivingWarV2, LivingWorldV2, ValidatedLivingCatalogPackV2,
    living_core_pack_v2,
};

// --------------------------------------------------------------- identifiers

/// Identities are big-endian so their byte order equals their numeric order,
/// which keeps every fixture collection sorted by construction.
fn id(value: u32) -> LivingEntityIdV2 {
    let mut bytes = [0_u8; 16];
    bytes[12..].copy_from_slice(&value.to_be_bytes());
    LivingEntityIdV2(bytes)
}

const SYSTEM_BASE: u32 = 1_000_000;
const STAR_BASE: u32 = 2_000_000;
const WORLD_BASE: u32 = 3_000_000;
const LANE_BASE: u32 = 4_000_000;
const CIVILIZATION_BASE: u32 = 5_000_000;
const DEPOSIT_BASE: u32 = 6_000_000;
const FACILITY_BASE: u32 = 7_000_000;
const CONSTRUCTION_BASE: u32 = 8_000_000;
const HULL_JOB_BASE: u32 = 9_000_000;
const FLEET_BASE: u32 = 10_000_000;
const HULL_BASE: u32 = 11_000_000;
const ROUTE_BASE: u32 = 12_000_000;
const SHIPMENT_BASE: u32 = 13_000_000;
const HAZARD_BASE: u32 = 14_000_000;

fn name(text: &str) -> LivingNameV2 {
    LivingNameV2::new(text).expect("valid name")
}

fn slug(text: &str) -> LivingSlugV2 {
    LivingSlugV2::new(text).expect("valid slug")
}

fn sorted<T>(items: Vec<T>) -> LivingSortedVecV2<T>
where
    T: nyon_workshop_core::living::LivingKeyedV2,
{
    LivingSortedVecV2::new(items).expect("fixture collection is sorted")
}

fn catalog() -> ValidatedLivingCatalogPackV2 {
    living_core_pack_v2().expect("the built-in pack validates")
}

// ------------------------------------------------------------ row generators

fn systems(count: usize) -> Vec<LivingSystemV2> {
    (0..count)
        .map(|index| LivingSystemV2 {
            id: id(SYSTEM_BASE + index as u32),
            name: name("System"),
        })
        .collect()
}

fn stars(count: usize) -> Vec<LivingStarV2> {
    (0..count)
        .map(|index| LivingStarV2 {
            id: id(STAR_BASE + index as u32),
            system: id(SYSTEM_BASE),
            archetype: slug("main_sequence"),
            name: name("Star"),
        })
        .collect()
}

fn worlds(count: usize) -> Vec<LivingWorldV2> {
    (0..count)
        .map(|index| LivingWorldV2 {
            id: id(WORLD_BASE + index as u32),
            system: id(SYSTEM_BASE),
            archetype: slug("temperate"),
            name: name("World"),
            owner: None,
            inventory: LivingInventoryV2::default(),
        })
        .collect()
}

fn lanes(count: usize) -> Vec<LivingLaneV2> {
    (0..count)
        .map(|index| LivingLaneV2 {
            id: id(LANE_BASE + index as u32),
            system_a: id(SYSTEM_BASE),
            system_b: id(SYSTEM_BASE + 1),
            distance_units: 100,
        })
        .collect()
}

fn civilizations(count: usize) -> Vec<LivingCivilizationV2> {
    (0..count)
        .map(|index| LivingCivilizationV2 {
            id: id(CIVILIZATION_BASE + index as u32),
            name: name("Civilization"),
            policy: LivingPolicyV2(slug("neutral")),
            status: LivingCivilizationStatusV2::Active,
        })
        .collect()
}

fn deposits(count: usize) -> Vec<LivingDepositV2> {
    (0..count)
        .map(|index| LivingDepositV2 {
            id: id(DEPOSIT_BASE + index as u32),
            world: id(WORLD_BASE),
            resource: LivingResourceV2::Ore,
            remaining_units: 20_000,
        })
        .collect()
}

fn facilities(count: usize) -> Vec<LivingFacilityV2> {
    (0..count)
        .map(|index| LivingFacilityV2 {
            id: id(FACILITY_BASE + index as u32),
            world: id(WORLD_BASE),
            definition: slug("solar_array"),
            status: LivingFacilityStatusV2::Operational,
            paid_alloy: 20,
            hit_points: 0,
            next_due: Some(LivingTickV2(10)),
            next_repair: None,
            blocked_reasons: LivingSortedVecV2::default(),
        })
        .collect()
}

fn routes(count: usize) -> Vec<LivingFreightRouteV2> {
    (0..count)
        .map(|index| LivingFreightRouteV2 {
            id: id(ROUTE_BASE + index as u32),
            kind: LivingRouteKindV2::Internal,
            source_world: id(WORLD_BASE),
            destination_world: id(WORLD_BASE + 1),
            resource: LivingResourceV2::Ore,
            batch_size: 10,
            cadence_ticks: 10,
            source_reserve: 20,
            source_owner: id(CIVILIZATION_BASE),
            receiver_owner: id(CIVILIZATION_BASE),
            next_due: LivingTickV2(10),
            suspended: false,
        })
        .collect()
}

fn shipments(count: usize) -> Vec<LivingShipmentV2> {
    (0..count)
        .map(|index| LivingShipmentV2 {
            id: id(SHIPMENT_BASE + index as u32),
            dispatch_owner: id(CIVILIZATION_BASE),
            intended_receiver: id(CIVILIZATION_BASE),
            source_world: id(WORLD_BASE),
            destination_world: id(WORLD_BASE + 1),
            resource: LivingResourceV2::Ore,
            units: 10,
            departure_tick: LivingTickV2(10),
            arrival_tick: LivingTickV2(20),
            disposition: LivingShipmentDispositionV2::Outbound,
        })
        .collect()
}

fn hazards(count: usize) -> Vec<LivingHazardV2> {
    (0..count)
        .map(|index| LivingHazardV2 {
            id: id(HAZARD_BASE + index as u32),
            definition: slug("ion_storm"),
            lane: id(LANE_BASE),
            start_tick: LivingTickV2(10),
            end_tick: LivingTickV2(20),
        })
        .collect()
}

fn hulls(count: usize, offset: u32) -> Vec<LivingHullV2> {
    (0..count)
        .map(|index| LivingHullV2 {
            id: id(HULL_BASE + offset + index as u32),
            definition: slug("escort"),
            hit_points: 10,
            return_credit: true,
            next_repair: None,
        })
        .collect()
}

fn fleets(count: usize, hulls_each: usize) -> Vec<LivingFleetV2> {
    (0..count)
        .map(|index| LivingFleetV2 {
            id: id(FLEET_BASE + index as u32),
            owner: id(CIVILIZATION_BASE),
            location: LivingFleetLocationV2::Docked {
                world: id(WORLD_BASE),
            },
            order: None,
            hulls: sorted(hulls(hulls_each, (index * hulls_each) as u32)),
        })
        .collect()
}

// ------------------------------------------------------------------ fixtures

/// A small state that satisfies every obligation, used as the starting point
/// for every single-mutation test below.
fn valid_state() -> LivingGalaxyStateV2 {
    let mut worlds = worlds(3);
    worlds[0].owner = Some(id(CIVILIZATION_BASE));
    worlds[0].inventory = LivingInventoryV2 {
        energy: 100,
        ore: 100,
        alloy: 120,
    };
    worlds[2].owner = Some(id(CIVILIZATION_BASE + 1));

    LivingGalaxyStateV2 {
        tick: LivingTickV2(1_000),
        accepted_sequence: 7,
        branch_sequence: 2,
        systems: sorted(systems(2)),
        stars: sorted(stars(2)),
        worlds: sorted(worlds),
        lanes: sorted(lanes(1)),
        civilizations: sorted(civilizations(2)),
        deposits: sorted(deposits(1)),
        colonies: sorted(vec![LivingColonyV2 {
            world: id(WORLD_BASE),
            owner: id(CIVILIZATION_BASE),
            hub: LivingHubV2 {
                energy_next_due: LivingTickV2(1_010),
                ore_next_due: LivingTickV2(1_100),
                fallback_next_due: LivingTickV2(1_100),
                blocked_reasons: LivingSortedVecV2::default(),
            },
        }]),
        facilities: sorted(facilities(1)),
        construction_jobs: sorted(vec![LivingConstructionJobV2 {
            id: id(CONSTRUCTION_BASE),
            world: id(WORLD_BASE),
            owner: id(CIVILIZATION_BASE),
            definition: slug("foundry"),
            accepted_tick: LivingTickV2(900),
            completion_tick: LivingTickV2(1_100),
            paid_alloy: 40,
        }]),
        hull_jobs: sorted(vec![LivingHullJobV2 {
            id: id(HULL_JOB_BASE),
            world: id(WORLD_BASE),
            owner: id(CIVILIZATION_BASE),
            definition: slug("escort"),
            accepted_tick: LivingTickV2(950),
            completion_tick: LivingTickV2(1_050),
            paid_alloy: 20,
            target_fleet: Some(id(FLEET_BASE)),
        }]),
        fleets: sorted(fleets(1, 2)),
        routes: sorted(routes(1)),
        shipments: sorted(shipments(1)),
        relations: sorted(vec![
            relation(id(CIVILIZATION_BASE), id(CIVILIZATION_BASE + 1)),
            relation(id(CIVILIZATION_BASE + 1), id(CIVILIZATION_BASE)),
        ]),
        agreements: sorted(vec![LivingAgreementV2 {
            kind: LivingAgreementKindV2::Trade,
            participant_a: id(CIVILIZATION_BASE),
            participant_b: id(CIVILIZATION_BASE + 1),
            start_tick: LivingTickV2(0),
            end_tick: LivingTickV2(1_200),
        }]),
        wars: sorted(vec![LivingWarV2 {
            participant_a: id(CIVILIZATION_BASE),
            participant_b: id(CIVILIZATION_BASE + 1),
            declarer: id(CIVILIZATION_BASE + 1),
            start_tick: LivingTickV2(900),
            end_tick: LivingTickV2(2_100),
        }]),
        observations: sorted(vec![LivingObservationV2 {
            civilization: id(CIVILIZATION_BASE),
            world: id(WORLD_BASE + 1),
            observed_tick: LivingTickV2(900),
            owner: None,
            remaining_deposit: 0,
            facilities: LivingSortedVecV2::default(),
            stationed_hulls: LivingSortedVecV2::default(),
        }]),
        hazards: sorted(hazards(1)),
        settlement_claims: sorted(vec![LivingSettlementClaimV2 {
            world: id(WORLD_BASE + 1),
            civilization: id(CIVILIZATION_BASE),
            arrival_tick: LivingTickV2(980),
            close_tick: LivingTickV2(1_030),
        }]),
        occupations: sorted(vec![LivingOccupationV2 {
            world: id(WORLD_BASE + 2),
            claimant: id(CIVILIZATION_BASE),
            progress_ticks: 5,
        }]),
        counters: nyon_workshop_core::living::LivingCountersV2::default(),
    }
}

fn relation(from: LivingEntityIdV2, to: LivingEntityIdV2) -> LivingRelationV2 {
    LivingRelationV2 {
        from,
        to,
        base_disposition: 10,
        reasons: LivingRelationReasonsV2::default(),
    }
}

/// A topology-only state, so a capacity test can grow one collection without
/// dragging the whole fixture's cross-references along.
fn scaffold() -> LivingGalaxyStateV2 {
    let mut worlds = worlds(2);
    worlds[0].owner = Some(id(CIVILIZATION_BASE));
    LivingGalaxyStateV2 {
        systems: sorted(systems(2)),
        worlds: sorted(worlds),
        lanes: sorted(lanes(1)),
        civilizations: sorted(civilizations(1)),
        ..LivingGalaxyStateV2::default()
    }
}

#[track_caller]
fn expect_valid(state: &LivingGalaxyStateV2) {
    state.validate(&catalog()).expect("state validates");
}

#[track_caller]
fn expect_error(state: &LivingGalaxyStateV2) -> LivingValidationErrorV2 {
    state
        .validate(&catalog())
        .expect_err("state must fail validation")
}

#[track_caller]
fn expect_capacity(state: &LivingGalaxyStateV2, collection: &str) {
    match expect_error(state) {
        LivingValidationErrorV2::Capacity {
            collection: found, ..
        } => assert_eq!(found, collection),
        other => panic!("expected a capacity failure for {collection}, found {other:?}"),
    }
}

#[track_caller]
fn expect_dangling(state: &LivingGalaxyStateV2) {
    let error = expect_error(state);
    assert!(
        matches!(error, LivingValidationErrorV2::DanglingReference { .. }),
        "expected a dangling reference, found {error:?}"
    );
}

#[track_caller]
fn expect_range(state: &LivingGalaxyStateV2, field: &str) {
    match expect_error(state) {
        LivingValidationErrorV2::Range { field: found } => assert_eq!(found, field),
        other => panic!("expected a range failure for {field}, found {other:?}"),
    }
}

#[track_caller]
fn expect_interval(state: &LivingGalaxyStateV2, field: &str) {
    match expect_error(state) {
        LivingValidationErrorV2::Interval { field: found } => assert_eq!(found, field),
        other => panic!("expected an interval failure for {field}, found {other:?}"),
    }
}

// ------------------------------------------------------------- the baselines

#[test]
fn the_reference_fixture_satisfies_every_obligation() {
    expect_valid(&valid_state());
}

#[test]
fn an_empty_state_satisfies_every_obligation() {
    expect_valid(&LivingGalaxyStateV2::default());
}

// -------------------------------------------------- section 1 authority maxima

#[test]
fn systems_are_bounded_at_sixty_four() {
    let mut state = LivingGalaxyStateV2 {
        systems: sorted(systems(LIVING_MAX_SYSTEMS_V2)),
        ..LivingGalaxyStateV2::default()
    };
    expect_valid(&state);
    state.systems = sorted(systems(LIVING_MAX_SYSTEMS_V2 + 1));
    expect_capacity(&state, "systems");
}

#[test]
fn stars_are_bounded_at_one_hundred_and_twenty_eight() {
    let mut state = scaffold();
    state.stars = sorted(stars(LIVING_MAX_STARS_V2));
    expect_valid(&state);
    state.stars = sorted(stars(LIVING_MAX_STARS_V2 + 1));
    expect_capacity(&state, "stars");
}

#[test]
fn worlds_are_bounded_at_five_hundred_and_twelve() {
    let mut state = scaffold();
    state.worlds = sorted(worlds(LIVING_MAX_WORLDS_V2));
    expect_valid(&state);
    state.worlds = sorted(worlds(LIVING_MAX_WORLDS_V2 + 1));
    expect_capacity(&state, "worlds");
}

#[test]
fn lanes_are_bounded_at_two_hundred_and_fifty_six() {
    let mut state = scaffold();
    state.lanes = sorted(lanes(LIVING_MAX_LANES_V2));
    expect_valid(&state);
    state.lanes = sorted(lanes(LIVING_MAX_LANES_V2 + 1));
    expect_capacity(&state, "lanes");
}

#[test]
fn civilizations_are_bounded_at_sixteen() {
    let mut state = scaffold();
    state.civilizations = sorted(civilizations(LIVING_MAX_CIVILIZATIONS_V2));
    expect_valid(&state);
    state.civilizations = sorted(civilizations(LIVING_MAX_CIVILIZATIONS_V2 + 1));
    expect_capacity(&state, "civilizations");
}

#[test]
fn deposits_are_bounded_at_one_thousand_and_twenty_four() {
    let mut state = scaffold();
    state.deposits = sorted(deposits(LIVING_MAX_DEPOSITS_V2));
    expect_valid(&state);
    state.deposits = sorted(deposits(LIVING_MAX_DEPOSITS_V2 + 1));
    expect_capacity(&state, "deposits");
}

#[test]
fn facilities_are_bounded_at_the_industry_maximum() {
    let mut state = scaffold();
    state.facilities = sorted(facilities(LIVING_MAX_INDUSTRIES_V2));
    expect_valid(&state);
    state.facilities = sorted(facilities(LIVING_MAX_INDUSTRIES_V2 + 1));
    expect_capacity(&state, "facilities");
}

#[test]
fn routes_are_bounded_at_two_thousand_and_forty_eight() {
    let mut state = scaffold();
    state.routes = sorted(routes(LIVING_MAX_ROUTES_V2));
    expect_valid(&state);
    state.routes = sorted(routes(LIVING_MAX_ROUTES_V2 + 1));
    expect_capacity(&state, "routes");
}

#[test]
fn shipments_are_bounded_at_four_thousand_and_ninety_six() {
    let mut state = scaffold();
    state.shipments = sorted(shipments(LIVING_MAX_SHIPMENTS_V2));
    expect_valid(&state);
    state.shipments = sorted(shipments(LIVING_MAX_SHIPMENTS_V2 + 1));
    expect_capacity(&state, "shipments");
}

#[test]
fn fleets_are_bounded_at_two_hundred_and_fifty_six() {
    let mut state = scaffold();
    state.fleets = sorted(fleets(LIVING_MAX_FLEETS_V2, 1));
    expect_valid(&state);
    state.fleets = sorted(fleets(LIVING_MAX_FLEETS_V2 + 1, 1));
    expect_capacity(&state, "fleets");
}

#[test]
fn hazards_are_bounded_at_one_hundred_and_twenty_eight() {
    let mut state = scaffold();
    state.hazards = sorted(hazards(LIVING_MAX_HAZARDS_V2));
    expect_valid(&state);
    state.hazards = sorted(hazards(LIVING_MAX_HAZARDS_V2 + 1));
    expect_capacity(&state, "hazards");
}

#[test]
fn one_fleet_holds_at_most_sixteen_hulls() {
    let mut state = scaffold();
    state.fleets = sorted(fleets(1, LIVING_MAX_HULLS_PER_FLEET_V2));
    expect_valid(&state);
    state.fleets = sorted(fleets(1, LIVING_MAX_HULLS_PER_FLEET_V2 + 1));
    expect_capacity(&state, "fleet hulls");
}

#[test]
fn total_hulls_are_bounded_independently_of_the_per_fleet_bound() {
    // 128 full fleets is exactly the total bound; 129 exceeds it while every
    // individual fleet still holds a legal sixteen.
    let at_limit = LIVING_MAX_HULLS_V2 / LIVING_MAX_HULLS_PER_FLEET_V2;
    let mut state = scaffold();
    state.fleets = sorted(fleets(at_limit, LIVING_MAX_HULLS_PER_FLEET_V2));
    expect_valid(&state);
    state.fleets = sorted(fleets(at_limit + 1, LIVING_MAX_HULLS_PER_FLEET_V2));
    expect_capacity(&state, "hulls");
}

#[test]
fn colonies_cannot_outnumber_worlds() {
    let mut state = scaffold();
    let colonies: Vec<LivingColonyV2> = (0..3)
        .map(|index| LivingColonyV2 {
            world: id(WORLD_BASE + index),
            owner: id(CIVILIZATION_BASE),
            hub: LivingHubV2 {
                energy_next_due: LivingTickV2(10),
                ore_next_due: LivingTickV2(100),
                fallback_next_due: LivingTickV2(100),
                blocked_reasons: LivingSortedVecV2::default(),
            },
        })
        .collect();
    state.colonies = sorted(colonies);
    expect_capacity(&state, "colonies");
}

// ---------------------------------------------- global identity and references

#[test]
fn one_identity_cannot_name_two_objects() {
    let mut state = valid_state();
    let mut rows = state.deposits.into_vec();
    rows[0].id = id(WORLD_BASE);
    state.deposits = sorted(rows);
    assert!(matches!(
        expect_error(&state),
        LivingValidationErrorV2::DuplicateIdentity { .. }
    ));
}

#[test]
fn a_nested_hull_shares_the_one_global_identity_space() {
    let mut state = valid_state();
    let mut rows = state.fleets.into_vec();
    let mut fleet_hulls = rows[0].hulls.clone().into_vec();
    fleet_hulls[0].id = id(FACILITY_BASE);
    rows[0].hulls = sorted(fleet_hulls);
    state.fleets = sorted(rows);
    assert!(matches!(
        expect_error(&state),
        LivingValidationErrorV2::DuplicateIdentity { .. }
    ));
}

#[test]
fn a_star_names_an_existing_system() {
    let mut state = valid_state();
    let mut rows = state.stars.into_vec();
    rows[0].system = id(1);
    state.stars = sorted(rows);
    expect_dangling(&state);
}

#[test]
fn a_star_names_a_catalog_archetype() {
    let mut state = valid_state();
    let mut rows = state.stars.into_vec();
    rows[0].archetype = slug("no_such_archetype");
    state.stars = sorted(rows);
    expect_dangling(&state);
}

#[test]
fn a_world_names_an_existing_owner() {
    let mut state = valid_state();
    let mut rows = state.worlds.into_vec();
    rows[0].owner = Some(id(1));
    state.worlds = sorted(rows);
    expect_dangling(&state);
}

#[test]
fn world_inventory_respects_the_storage_limit() {
    let mut state = valid_state();
    let mut rows = state.worlds.into_vec();
    rows[0].inventory.ore = LIVING_RESOURCE_STORAGE_LIMIT_V2;
    state.worlds = sorted(rows);
    expect_valid(&state);

    let mut rows = state.worlds.into_vec();
    rows[0].inventory.ore = LIVING_RESOURCE_STORAGE_LIMIT_V2 + 1;
    state.worlds = sorted(rows);
    expect_range(&state, "world inventory");
}

#[test]
fn a_lane_joins_two_distinct_ordered_systems() {
    let mut state = valid_state();
    let mut rows = state.lanes.into_vec();
    rows[0].system_b = rows[0].system_a;
    state.lanes = sorted(rows);
    expect_range(&state, "lane endpoints");
}

#[test]
fn a_lane_declares_a_nonzero_distance() {
    let mut state = valid_state();
    let mut rows = state.lanes.into_vec();
    rows[0].distance_units = 0;
    state.lanes = sorted(rows);
    expect_range(&state, "lane distance_units");
}

#[test]
fn a_civilization_names_a_catalog_policy() {
    let mut state = valid_state();
    let mut rows = state.civilizations.into_vec();
    rows[0].policy = LivingPolicyV2(slug("no_such_policy"));
    state.civilizations = sorted(rows);
    expect_dangling(&state);
}

#[test]
fn a_colony_owner_matches_its_worlds_owner() {
    let mut state = valid_state();
    let mut rows = state.colonies.into_vec();
    rows[0].owner = id(CIVILIZATION_BASE + 1);
    state.colonies = sorted(rows);
    expect_range(&state, "colony owner");
}

#[test]
fn a_colony_names_an_existing_world() {
    let mut state = valid_state();
    let mut rows = state.colonies.into_vec();
    rows[0].world = id(1);
    state.colonies = sorted(rows);
    expect_dangling(&state);
}

#[test]
fn a_facility_names_a_catalog_industry() {
    let mut state = valid_state();
    let mut rows = state.facilities.into_vec();
    rows[0].definition = slug("no_such_industry");
    state.facilities = sorted(rows);
    expect_dangling(&state);
}

#[test]
fn a_construction_job_completes_after_it_is_accepted() {
    let mut state = valid_state();
    let mut rows = state.construction_jobs.into_vec();
    rows[0].completion_tick = rows[0].accepted_tick;
    state.construction_jobs = sorted(rows);
    expect_interval(&state, "construction job duration");
}

#[test]
fn a_hull_job_names_an_existing_target_fleet() {
    let mut state = valid_state();
    let mut rows = state.hull_jobs.into_vec();
    rows[0].target_fleet = Some(id(1));
    state.hull_jobs = sorted(rows);
    expect_dangling(&state);
}

#[test]
fn a_hull_job_may_reserve_a_new_fleet_instead_of_naming_one() {
    let mut state = valid_state();
    let mut rows = state.hull_jobs.into_vec();
    rows[0].target_fleet = None;
    state.hull_jobs = sorted(rows);
    expect_valid(&state);
}

#[test]
fn a_travelling_fleet_arrives_after_it_departs() {
    let mut state = valid_state();
    let mut rows = state.fleets.into_vec();
    rows[0].location = LivingFleetLocationV2::Travelling {
        origin_world: id(WORLD_BASE),
        destination_world: id(WORLD_BASE + 1),
        departure_tick: LivingTickV2(50),
        arrival_tick: LivingTickV2(50),
    };
    state.fleets = sorted(rows);
    expect_interval(&state, "fleet leg");
}

#[test]
fn a_fleet_order_names_existing_lanes() {
    let mut state = valid_state();
    let mut rows = state.fleets.into_vec();
    rows[0].order = Some(LivingFleetOrderV2 {
        kind: LivingFleetOrderKindV2::Move,
        target_world: Some(id(WORLD_BASE + 1)),
        not_before_tick: LivingTickV2(1_001),
        lane_path: vec![id(LANE_BASE)],
    });
    state.fleets = sorted(rows);
    expect_valid(&state);

    let mut rows = state.fleets.into_vec();
    rows[0].order.as_mut().expect("an order").lane_path = vec![id(1)];
    state.fleets = sorted(rows);
    expect_dangling(&state);
}

#[test]
fn a_hull_cannot_exceed_its_catalog_hit_points() {
    let mut state = valid_state();
    let mut rows = state.fleets.into_vec();
    let mut fleet_hulls = rows[0].hulls.clone().into_vec();
    fleet_hulls[0].hit_points = 11;
    rows[0].hulls = sorted(fleet_hulls);
    state.fleets = sorted(rows);
    expect_range(&state, "hull hit_points");
}

#[test]
fn a_route_joins_two_distinct_worlds() {
    let mut state = valid_state();
    let mut rows = state.routes.into_vec();
    rows[0].destination_world = rows[0].source_world;
    state.routes = sorted(rows);
    expect_range(&state, "route endpoints");
}

#[test]
fn a_route_declares_a_nonzero_cadence_and_batch() {
    let mut state = valid_state();
    let mut rows = state.routes.into_vec();
    rows[0].cadence_ticks = 0;
    state.routes = sorted(rows);
    expect_range(&state, "route cadence");
}

/// Rules "Freight": "batch size up to ten, cadence ten ticks, source reserve
/// at least 20". The validator checked only nonzero and the storage limit
/// (addendum K5, 2026-09-17).
#[test]
fn a_route_declares_the_rules_batch_cadence_and_reserve() {
    let mut state = valid_state();
    let mut rows = state.routes.into_vec();
    rows[0].batch_size = 11;
    state.routes = sorted(rows);
    expect_range(&state, "route batch_size");

    let mut state = valid_state();
    let mut rows = state.routes.into_vec();
    rows[0].cadence_ticks = 11;
    state.routes = sorted(rows);
    expect_range(&state, "route cadence");

    let mut state = valid_state();
    let mut rows = state.routes.into_vec();
    rows[0].cadence_ticks = 9;
    state.routes = sorted(rows);
    expect_range(&state, "route cadence");

    let mut state = valid_state();
    let mut rows = state.routes.into_vec();
    rows[0].source_reserve = 19;
    state.routes = sorted(rows);
    expect_range(&state, "route source_reserve");

    // The boundaries themselves are valid.
    let mut state = valid_state();
    let mut rows = state.routes.into_vec();
    rows[0].batch_size = 1;
    rows[0].source_reserve = 20;
    state.routes = sorted(rows);
    state
        .validate(&catalog())
        .expect("batch 1, reserve 20 is valid");
}

#[test]
fn a_shipment_carries_a_nonzero_load_and_arrives_after_it_departs() {
    let mut state = valid_state();
    let mut rows = state.shipments.into_vec();
    rows[0].units = 0;
    state.shipments = sorted(rows);
    expect_range(&state, "shipment units");

    let mut rows = state.shipments.into_vec();
    rows[0].units = 10;
    rows[0].arrival_tick = rows[0].departure_tick;
    state.shipments = sorted(rows);
    expect_interval(&state, "shipment travel");
}

// -------------------------------------------------------- section 6 diplomacy

#[test]
fn a_relation_is_directed_between_two_different_civilizations() {
    let mut state = valid_state();
    let mut rows = state.relations.into_vec();
    rows[0].to = rows[0].from;
    state.relations = sorted(rows);
    expect_range(&state, "relation parties");
}

#[test]
fn a_base_disposition_stays_inside_the_declared_hundred_point_band() {
    let mut state = valid_state();
    let mut rows = state.relations.into_vec();
    rows[0].base_disposition = 101;
    state.relations = sorted(rows);
    expect_range(&state, "relation base_disposition");
}

/// One single-counter mutation applied to a fixture relation.
type ReasonMutation = fn(&mut LivingRelationReasonsV2);

#[test]
fn every_reason_counter_respects_its_own_cap_or_floor() {
    let cases: [(ReasonMutation, &str); 5] = [
        (
            |reasons| reasons.delivered_aid = 21,
            "relation delivered_aid",
        ),
        (
            |reasons| reasons.successful_trade = 21,
            "relation successful_trade",
        ),
        (
            |reasons| reasons.adjacent_rival_settlement = -21,
            "relation adjacent_rival_settlement",
        ),
        (
            |reasons| reasons.war_declared_against_it = -31,
            "relation war_declared_against_it",
        ),
        (
            |reasons| reasons.colony_captured_from_it = -61,
            "relation colony_captured_from_it",
        ),
    ];
    for (mutate, field) in cases {
        let mut state = valid_state();
        let mut rows = state.relations.into_vec();
        mutate(&mut rows[0].reasons);
        state.relations = sorted(rows);
        expect_range(&state, field);
    }
}

#[test]
fn a_delivery_accumulator_never_reaches_a_whole_counter_step() {
    let mut state = valid_state();
    let mut rows = state.relations.into_vec();
    rows[0].reasons.trade_units_remainder = 99;
    state.relations = sorted(rows);
    expect_valid(&state);

    let mut rows = state.relations.into_vec();
    rows[0].reasons.trade_units_remainder = 100;
    state.relations = sorted(rows);
    expect_range(&state, "relation delivery accumulator");
}

#[test]
fn an_agreement_stores_its_participants_in_ascending_order() {
    let mut state = valid_state();
    let mut rows = state.agreements.into_vec();
    let a = rows[0].participant_a;
    rows[0].participant_a = rows[0].participant_b;
    rows[0].participant_b = a;
    state.agreements = sorted(rows);
    expect_range(&state, "agreement participants");
}

#[test]
fn an_agreement_interval_is_half_open_and_nonempty() {
    let mut state = valid_state();
    let mut rows = state.agreements.into_vec();
    rows[0].end_tick = rows[0].start_tick;
    state.agreements = sorted(rows);
    expect_interval(&state, "agreement");
}

#[test]
fn a_war_declarer_is_one_of_its_two_participants() {
    let mut state = valid_state();
    let mut rows = state.wars.into_vec();
    rows[0].declarer = id(1);
    state.wars = sorted(rows);
    expect_range(&state, "war declarer");
}

// --------------------------------------- observations, hazards, claims, sieges

#[test]
fn an_observation_cannot_be_taken_after_the_current_boundary() {
    let mut state = valid_state();
    let mut rows = state.observations.into_vec();
    rows[0].observed_tick = LivingTickV2(state.tick.0 + 1);
    state.observations = sorted(rows);
    expect_range(&state, "observation observed_tick");
}

#[test]
fn a_hazard_names_an_existing_lane_and_a_catalog_definition() {
    let mut state = valid_state();
    let mut rows = state.hazards.into_vec();
    rows[0].lane = id(1);
    state.hazards = sorted(rows);
    expect_dangling(&state);

    let mut state = valid_state();
    let mut rows = state.hazards.into_vec();
    rows[0].definition = slug("no_such_hazard");
    state.hazards = sorted(rows);
    expect_dangling(&state);
}

#[test]
fn every_claim_on_one_world_shares_its_arbitration_boundary() {
    let mut state = valid_state();
    state.settlement_claims = sorted(vec![
        LivingSettlementClaimV2 {
            world: id(WORLD_BASE + 1),
            civilization: id(CIVILIZATION_BASE),
            arrival_tick: LivingTickV2(980),
            close_tick: LivingTickV2(1_030),
        },
        LivingSettlementClaimV2 {
            world: id(WORLD_BASE + 1),
            civilization: id(CIVILIZATION_BASE + 1),
            arrival_tick: LivingTickV2(1_000),
            close_tick: LivingTickV2(1_030),
        },
    ]);
    expect_valid(&state);

    let mut rows = state.settlement_claims.into_vec();
    rows[1].close_tick = LivingTickV2(1_050);
    state.settlement_claims = sorted(rows);
    expect_range(&state, "claim close_tick");
}

#[test]
fn occupation_progress_stops_at_the_transfer_boundary() {
    let mut state = valid_state();
    let mut rows = state.occupations.into_vec();
    rows[0].progress_ticks = 100;
    state.occupations = sorted(rows);
    expect_valid(&state);

    let mut rows = state.occupations.into_vec();
    rows[0].progress_ticks = 101;
    state.occupations = sorted(rows);
    expect_range(&state, "occupation progress_ticks");
}

// ------------------------------------------------------------ canonical bytes

#[test]
fn the_reference_fixture_round_trips_through_canonical_bytes() {
    let state = valid_state();
    let bytes = state.canonical_bytes().expect("encodes");
    let decoded: LivingGalaxyStateV2 =
        nyon_workshop_core::living::decode_canonical_v2(&bytes, bytes.len()).expect("decodes");
    assert_eq!(decoded, state);
    assert_eq!(
        decoded.digest().expect("digests"),
        state.digest().expect("digests")
    );
}

#[test]
fn the_historical_sequences_are_carried_through_encoding_unchanged() {
    let state = valid_state();
    let bytes = state.canonical_bytes().expect("encodes");
    let text = str::from_utf8(&bytes).expect("utf-8");
    assert!(text.starts_with(r#"{"tick":1000,"accepted_sequence":7,"branch_sequence":2,"#));
}

// ---------------------------------------------------------------------------
// Record-level field-order pin (review finding F1)
//
// WHAT THIS IS, STATED PRECISELY, BECAUSE THE DISTINCTION IS LOAD-BEARING.
//
// This is a DRIFT REGRESSION PIN DERIVED FROM THIS IMPLEMENTATION. It is *not*
// independent evidence and must never be described as such. Section 10 of the
// rules spec publishes the 24-field order of the state itself, but publishes no
// field list for any individual record, so there is no upstream text from which
// a record's field order could be independently derived. `model.rs`'s own
// header says exactly that and declares itself the authority for record order.
//
// The consequence is unavoidable and better stated than hidden: this pin cannot
// tell you the current order is *correct*, because nothing outside this crate
// defines correct. It tells you the order has not *changed*. That is precisely
// the failure it exists to catch. A reordered record is a canonical format
// break that moves every `state_digest` of any state containing that record,
// and before this test such a reorder passed the whole suite with zero
// failures: the only byte-exact pin was over the *empty* state, where all
// twenty collections are `[]`, so it constrained the 24 top-level names and
// nothing inside them.
//
// It pins ORDER ONLY, never values, so editing a fixture value does not touch
// it and there is no routine pressure to regenerate it.
//
// IF THIS TEST FAILS, DO NOT PASTE IN THE NEW SEQUENCE. Read the diff, find
// what moved, and establish whether the move was intended. If it was, every
// frozen digest covering a state containing that record is now invalid and must
// be re-derived -- outside this crate, by the same non-Rust path the rest of the
// corpus uses.

/// Returns every object key in the byte stream, in document order.
///
/// Deliberately a scanner rather than a `serde_json::Value` walk: serde_json's
/// default map is sorted, so parsing would discard the very property under test.
fn canonical_key_sequence(bytes: &[u8]) -> Vec<String> {
    let mut keys = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'"' {
            index += 1;
            continue;
        }
        let start = index + 1;
        let mut end = start;
        while end < bytes.len() {
            match bytes[end] {
                b'\\' => end += 2,
                b'"' => break,
                _ => end += 1,
            }
        }
        if end >= bytes.len() {
            break;
        }
        // A string is a key exactly when the next byte is ':'. The canonical
        // encoding is compact, so no whitespace skip is needed.
        if bytes.get(end + 1) == Some(&b':') {
            keys.push(String::from_utf8_lossy(&bytes[start..end]).into_owned());
        }
        index = end + 1;
    }
    keys
}

#[test]
fn every_record_field_order_is_pinned_against_silent_drift() {
    // `valid_state()` populates all twenty-four fields with every collection
    // non-empty, so this sequence reaches inside every record type the schema
    // defines -- including the nested hulls inside a fleet and the nested
    // inventories inside a world and a colony.
    let state = valid_state();
    let bytes = state
        .canonical_bytes()
        .expect("the reference fixture encodes");
    let keys = canonical_key_sequence(&bytes);

    #[rustfmt::skip]
    const EXPECTED: &[&str] = &[
        "tick", "accepted_sequence", "branch_sequence", "systems",
        "id", "name", "id", "name",
        "stars", "id", "system", "archetype",
        "name", "id", "system", "archetype",
        "name", "worlds", "id", "system",
        "archetype", "name", "owner", "inventory",
        "energy", "ore", "alloy", "id",
        "system", "archetype", "name", "owner",
        "inventory", "energy", "ore", "alloy",
        "id", "system", "archetype", "name",
        "owner", "inventory", "energy", "ore",
        "alloy", "lanes", "id", "system_a",
        "system_b", "distance_units", "civilizations", "id",
        "name", "policy", "status", "id",
        "name", "policy", "status", "deposits",
        "id", "world", "resource", "remaining_units",
        "colonies", "world", "owner", "hub",
        "energy_next_due", "ore_next_due", "fallback_next_due", "blocked_reasons",
        "facilities", "id", "world", "definition",
        "status", "paid_alloy", "hit_points", "next_due",
        "next_repair", "blocked_reasons", "construction_jobs", "id",
        "world", "owner", "definition", "accepted_tick",
        "completion_tick", "paid_alloy", "hull_jobs", "id",
        "world", "owner", "definition", "accepted_tick",
        "completion_tick", "paid_alloy", "target_fleet", "fleets",
        "id", "owner", "location", "type",
        "world", "order", "hulls", "id",
        "definition", "hit_points", "return_credit", "next_repair",
        "id", "definition", "hit_points", "return_credit",
        "next_repair", "routes", "id", "kind",
        "source_world", "destination_world", "resource", "batch_size",
        "cadence_ticks", "source_reserve", "source_owner", "receiver_owner",
        "next_due", "suspended", "shipments", "id",
        "dispatch_owner", "intended_receiver", "source_world", "destination_world",
        "resource", "units", "departure_tick", "arrival_tick",
        "disposition", "relations", "from", "to",
        "base_disposition", "reasons", "delivered_aid", "successful_trade",
        "adjacent_rival_settlement", "war_declared_against_it", "colony_captured_from_it", "aid_units_remainder",
        "trade_units_remainder", "from", "to", "base_disposition",
        "reasons", "delivered_aid", "successful_trade", "adjacent_rival_settlement",
        "war_declared_against_it", "colony_captured_from_it", "aid_units_remainder", "trade_units_remainder",
        "agreements", "kind", "participant_a", "participant_b",
        "start_tick", "end_tick", "wars", "participant_a",
        "participant_b", "declarer", "start_tick", "end_tick",
        "observations", "civilization", "world", "observed_tick",
        "owner", "remaining_deposit", "facilities", "stationed_hulls",
        "hazards", "id", "definition", "lane",
        "start_tick", "end_tick", "settlement_claims", "world",
        "civilization", "arrival_tick", "close_tick", "occupations",
        "world", "claimant", "progress_ticks", "counters",
    ];

    if keys != EXPECTED {
        panic!(
            "canonical field order changed ({} keys, expected {}).\n\
             Read the note above this test before touching the expected list.\n\
             actual = {:#?}",
            keys.len(),
            EXPECTED.len(),
            keys
        );
    }
}
