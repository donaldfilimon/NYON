//! The Living Galaxy V2 genesis manifest.
//!
//! The positive fixture is section 3's declared starting content read
//! literally: "A procedural home starts with hub, solar array, extractor,
//! foundry, shipyard, 100 energy, 100 ore, 120 alloy, and an ore deposit of
//! 20000 units. It also starts with one scout and two escorts in one docked
//! fleet." Its clocks are section 2's genesis rule: "hub energy, solar, and
//! extractor at G+10; foundry at G+20; hub ore/fallback at G+100."
//!
//! No expected hash is written down anywhere in this file. Every digest
//! assertion is a structural identity between two things the crate derives, so
//! nothing here can become a recording of the implementation's own output. The
//! frozen byte vectors are a separate task and are derived outside this crate.

use nyon_workshop_core::living::{
    LIVING_GENESIS_FORMAT_VERSION_V2, LIVING_GENESIS_KIND_V2, LIVING_HUB_ENERGY_PERIOD_TICKS_V2,
    LIVING_HUB_FALLBACK_PERIOD_TICKS_V2, LIVING_HUB_ORE_PERIOD_TICKS_V2, LIVING_RULES_VERSION,
    LivingCivilizationStatusV2, LivingCivilizationV2, LivingColonyV2, LivingDepositV2,
    LivingEntityIdV2, LivingFacilityStatusV2, LivingFacilityV2, LivingFleetLocationV2,
    LivingFleetV2, LivingGalaxyStateV2, LivingGenesisErrorV2, LivingGenesisGeneratorV2,
    LivingGenesisManifestV2, LivingHubV2, LivingHullV2, LivingInventoryV2, LivingKeyedV2,
    LivingNameV2, LivingPolicyV2, LivingResourceV2, LivingSlugV2, LivingSortedVecV2, LivingStarV2,
    LivingStateDigestV2, LivingSystemV2, LivingTickV2, LivingValidationErrorV2, LivingWireErrorV2,
    LivingWorldV2, ValidatedLivingCatalogPackV2, decode_living_genesis_manifest_v2,
    living_core_pack_v2, root_branch_id_v2, state_digest_v2, validate_living_genesis_manifest_v2,
};

// -------------------------------------------------------------- fixture parts

const SYSTEM: u32 = 1_000_000;
const STAR: u32 = 2_000_000;
const WORLD: u32 = 3_000_000;
const CIVILIZATION: u32 = 5_000_000;
const DEPOSIT: u32 = 6_000_000;
const FACILITY: u32 = 7_000_000;
const FLEET: u32 = 10_000_000;
const HULL: u32 = 11_000_000;

const SEED: [u8; 32] = [0x5e; 32];

fn id(value: u32) -> LivingEntityIdV2 {
    let mut bytes = [0_u8; 16];
    bytes[12..].copy_from_slice(&value.to_be_bytes());
    LivingEntityIdV2(bytes)
}

fn name(text: &str) -> LivingNameV2 {
    LivingNameV2::new(text).expect("valid name")
}

fn slug(text: &str) -> LivingSlugV2 {
    LivingSlugV2::new(text).expect("valid slug")
}

fn sorted<T: LivingKeyedV2>(items: Vec<T>) -> LivingSortedVecV2<T> {
    LivingSortedVecV2::new(items).expect("fixture collection is sorted")
}

fn catalog() -> ValidatedLivingCatalogPackV2 {
    living_core_pack_v2().expect("the built-in pack validates")
}

/// One facility of the declared home, with the clock section 2 gives it.
fn facility(offset: u32, definition: &str, next_due: Option<u64>) -> LivingFacilityV2 {
    LivingFacilityV2 {
        id: id(FACILITY + offset),
        world: id(WORLD),
        definition: slug(definition),
        status: LivingFacilityStatusV2::Operational,
        paid_alloy: 0,
        hit_points: 0,
        next_due: next_due.map(LivingTickV2),
        next_repair: None,
        blocked_reasons: LivingSortedVecV2::default(),
    }
}

fn hull(offset: u32, definition: &str, hit_points: u32) -> LivingHullV2 {
    LivingHullV2 {
        id: id(HULL + offset),
        definition: slug(definition),
        hit_points,
        return_credit: false,
        next_repair: None,
    }
}

/// Section 3's declared procedural home at the genesis boundary.
fn home_state() -> LivingGalaxyStateV2 {
    LivingGalaxyStateV2 {
        tick: LivingTickV2(0),
        accepted_sequence: 0,
        branch_sequence: 0,
        systems: sorted(vec![LivingSystemV2 {
            id: id(SYSTEM),
            name: name("Home"),
        }]),
        stars: sorted(vec![LivingStarV2 {
            id: id(STAR),
            system: id(SYSTEM),
            archetype: slug("main_sequence"),
            name: name("Home Star"),
        }]),
        worlds: sorted(vec![LivingWorldV2 {
            id: id(WORLD),
            system: id(SYSTEM),
            archetype: slug("temperate"),
            name: name("Home World"),
            owner: Some(id(CIVILIZATION)),
            inventory: LivingInventoryV2 {
                energy: 100,
                ore: 100,
                alloy: 120,
            },
        }]),
        lanes: LivingSortedVecV2::default(),
        civilizations: sorted(vec![LivingCivilizationV2 {
            id: id(CIVILIZATION),
            name: name("Founders"),
            policy: LivingPolicyV2(slug("neutral")),
            status: LivingCivilizationStatusV2::Active,
        }]),
        deposits: sorted(vec![LivingDepositV2 {
            id: id(DEPOSIT),
            world: id(WORLD),
            resource: LivingResourceV2::Ore,
            remaining_units: 20_000,
        }]),
        colonies: sorted(vec![LivingColonyV2 {
            world: id(WORLD),
            owner: id(CIVILIZATION),
            hub: LivingHubV2 {
                energy_next_due: LivingTickV2(LIVING_HUB_ENERGY_PERIOD_TICKS_V2),
                ore_next_due: LivingTickV2(LIVING_HUB_ORE_PERIOD_TICKS_V2),
                fallback_next_due: LivingTickV2(LIVING_HUB_FALLBACK_PERIOD_TICKS_V2),
                blocked_reasons: LivingSortedVecV2::default(),
            },
        }]),
        facilities: sorted(vec![
            facility(0, "solar_array", Some(10)),
            facility(1, "extractor", Some(10)),
            facility(2, "foundry", Some(20)),
            facility(3, "shipyard", None),
        ]),
        construction_jobs: LivingSortedVecV2::default(),
        hull_jobs: LivingSortedVecV2::default(),
        fleets: sorted(vec![LivingFleetV2 {
            id: id(FLEET),
            owner: id(CIVILIZATION),
            location: LivingFleetLocationV2::Docked { world: id(WORLD) },
            order: None,
            hulls: sorted(vec![
                hull(0, "scout", 10),
                hull(1, "escort", 10),
                hull(2, "escort", 10),
            ]),
        }]),
        routes: LivingSortedVecV2::default(),
        shipments: LivingSortedVecV2::default(),
        relations: LivingSortedVecV2::default(),
        agreements: LivingSortedVecV2::default(),
        wars: LivingSortedVecV2::default(),
        observations: LivingSortedVecV2::default(),
        hazards: LivingSortedVecV2::default(),
        settlement_claims: LivingSortedVecV2::default(),
        occupations: LivingSortedVecV2::default(),
        counters: nyon_workshop_core::living::LivingCountersV2 {},
    }
}

fn manifest(state: LivingGalaxyStateV2) -> LivingGenesisManifestV2 {
    LivingGenesisManifestV2 {
        kind: LIVING_GENESIS_KIND_V2.to_owned(),
        format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
        rules_version: LIVING_RULES_VERSION,
        state,
    }
}

fn home_manifest() -> LivingGenesisManifestV2 {
    manifest(home_state())
}

fn accept(
    document: LivingGenesisManifestV2,
) -> nyon_workshop_core::living::ValidatedLivingGenesisV2 {
    validate_living_genesis_manifest_v2(document, &catalog()).expect("the fixture is valid genesis")
}

fn reject(document: LivingGenesisManifestV2) -> LivingGenesisErrorV2 {
    validate_living_genesis_manifest_v2(document, &catalog())
        .expect_err("this manifest must be refused")
}

/// Mutate the state inside an otherwise valid manifest.
fn mutated(change: impl FnOnce(&mut LivingGalaxyStateV2)) -> LivingGenesisErrorV2 {
    let mut document = home_manifest();
    change(&mut document.state);
    reject(document)
}

// ------------------------------------------------------------- the happy path

#[test]
fn the_declared_procedural_home_is_valid_genesis() {
    let genesis = accept(home_manifest());

    assert_eq!(genesis.format_version(), LIVING_GENESIS_FORMAT_VERSION_V2);
    assert_eq!(genesis.rules_version(), LIVING_RULES_VERSION);
    assert_eq!(genesis.state().tick, LivingTickV2(0));
    assert_eq!(genesis.catalog_hash(), catalog().catalog_hash());
}

#[test]
fn genesis_carries_section_threes_declared_starting_content() {
    let genesis = accept(home_manifest());
    let state = genesis.state();

    let world = &state.worlds.as_slice()[0];
    assert_eq!(world.inventory.energy, 100);
    assert_eq!(world.inventory.ore, 100);
    assert_eq!(world.inventory.alloy, 120);

    assert_eq!(state.deposits.as_slice()[0].remaining_units, 20_000);
    assert_eq!(state.deposits.as_slice()[0].resource, LivingResourceV2::Ore);

    let mut definitions: Vec<&str> = state
        .facilities
        .as_slice()
        .iter()
        .map(|row| row.definition.as_str())
        .collect();
    definitions.sort_unstable();
    assert_eq!(
        definitions,
        ["extractor", "foundry", "shipyard", "solar_array"]
    );
    assert_eq!(state.colonies.as_slice().len(), 1, "the home has one hub");

    let fleet = &state.fleets.as_slice()[0];
    assert!(matches!(
        fleet.location,
        LivingFleetLocationV2::Docked { .. }
    ));
    let mut hulls: Vec<&str> = fleet
        .hulls
        .as_slice()
        .iter()
        .map(|row| row.definition.as_str())
        .collect();
    hulls.sort_unstable();
    assert_eq!(hulls, ["escort", "escort", "scout"]);
}

// -------------------------------------------------------------- the envelope

#[test]
fn a_manifest_of_another_kind_is_refused_before_anything_else() {
    let mut document = home_manifest();
    document.kind = "NYON_LIVING_GALAXY_DATA".to_owned();
    // The state is simultaneously ruined, so this also proves the envelope is
    // checked first rather than incidentally.
    document.state.tick = LivingTickV2(9);
    assert_eq!(reject(document), LivingGenesisErrorV2::Kind);
}

#[test]
fn an_unknown_format_version_is_refused_with_the_version_found() {
    let mut document = home_manifest();
    document.format_version = 3;
    assert_eq!(
        reject(document),
        LivingGenesisErrorV2::FormatVersion { found: 3 }
    );
}

#[test]
fn another_rules_version_is_refused_with_the_version_found() {
    let mut document = home_manifest();
    document.rules_version = 1;
    assert_eq!(
        reject(document),
        LivingGenesisErrorV2::RulesVersion { found: 1 }
    );
}

// ------------------------------------------------------- the genesis boundary

#[test]
fn genesis_is_the_zero_boundary() {
    assert_eq!(
        mutated(|state| state.tick = LivingTickV2(1)),
        LivingGenesisErrorV2::Tick { found: 1 }
    );
}

#[test]
fn genesis_has_accepted_nothing_and_forked_nothing() {
    assert_eq!(
        mutated(|state| state.accepted_sequence = 1),
        LivingGenesisErrorV2::Sequence {
            field: "accepted_sequence",
            found: 1,
        }
    );
    assert_eq!(
        mutated(|state| state.branch_sequence = 4),
        LivingGenesisErrorV2::Sequence {
            field: "branch_sequence",
            found: 4,
        }
    );
}

// ---------------------------------------------------------- the clock rule

#[test]
fn the_hub_energy_clock_is_ten_boundaries_out() {
    let error = mutated(|state| {
        let mut colony = state.colonies.as_slice()[0].clone();
        colony.hub.energy_next_due = LivingTickV2(9);
        state.colonies = sorted(vec![colony]);
    });
    assert_eq!(
        error,
        LivingGenesisErrorV2::Clock {
            clock: "hub energy_next_due",
            found: Some(9),
            expected: Some(10),
        }
    );
}

#[test]
fn the_hub_ore_and_fallback_clocks_are_one_hundred_boundaries_out() {
    let ore = mutated(|state| {
        let mut colony = state.colonies.as_slice()[0].clone();
        colony.hub.ore_next_due = LivingTickV2(10);
        state.colonies = sorted(vec![colony]);
    });
    assert_eq!(
        ore,
        LivingGenesisErrorV2::Clock {
            clock: "hub ore_next_due",
            found: Some(10),
            expected: Some(100),
        }
    );

    let fallback = mutated(|state| {
        let mut colony = state.colonies.as_slice()[0].clone();
        colony.hub.fallback_next_due = LivingTickV2(101);
        state.colonies = sorted(vec![colony]);
    });
    assert_eq!(
        fallback,
        LivingGenesisErrorV2::Clock {
            clock: "hub fallback_next_due",
            found: Some(101),
            expected: Some(100),
        }
    );
}

#[test]
fn a_recurring_facility_is_due_one_of_its_own_periods_after_genesis() {
    // Solar and extractor share a ten-tick period; the foundry's is twenty.
    let solar = mutated(|state| {
        state.facilities = sorted(vec![
            facility(0, "solar_array", Some(11)),
            facility(1, "extractor", Some(10)),
            facility(2, "foundry", Some(20)),
            facility(3, "shipyard", None),
        ]);
    });
    assert_eq!(
        solar,
        LivingGenesisErrorV2::Clock {
            clock: "facility next_due",
            found: Some(11),
            expected: Some(10),
        }
    );

    let foundry = mutated(|state| {
        state.facilities = sorted(vec![
            facility(0, "solar_array", Some(10)),
            facility(1, "extractor", Some(10)),
            facility(2, "foundry", Some(10)),
            facility(3, "shipyard", None),
        ]);
    });
    assert_eq!(
        foundry,
        LivingGenesisErrorV2::Clock {
            clock: "facility next_due",
            found: Some(10),
            expected: Some(20),
        }
    );
}

#[test]
fn a_facility_with_no_recurring_recipe_has_no_due_boundary() {
    let error = mutated(|state| {
        state.facilities = sorted(vec![
            facility(0, "solar_array", Some(10)),
            facility(1, "extractor", Some(10)),
            facility(2, "foundry", Some(20)),
            facility(3, "shipyard", Some(10)),
        ]);
    });
    assert_eq!(
        error,
        LivingGenesisErrorV2::Clock {
            clock: "facility next_due",
            found: Some(10),
            expected: None,
        }
    );

    let missing = mutated(|state| {
        state.facilities = sorted(vec![
            facility(0, "solar_array", None),
            facility(1, "extractor", Some(10)),
            facility(2, "foundry", Some(20)),
            facility(3, "shipyard", None),
        ]);
    });
    assert_eq!(
        missing,
        LivingGenesisErrorV2::Clock {
            clock: "facility next_due",
            found: None,
            expected: Some(10),
        }
    );
}

// ------------------------------------------- composition, not re-implementation

#[test]
fn a_state_the_authority_contract_refuses_is_not_valid_genesis() {
    // A dangling world reference is `LivingGalaxyStateV2::validate`'s rule.
    // Genesis surfaces it through that call rather than restating it.
    let error = mutated(|state| {
        let mut colony = state.colonies.as_slice()[0].clone();
        colony.world = id(WORLD + 7);
        state.colonies = sorted(vec![colony]);
    });
    assert!(
        matches!(error, LivingGenesisErrorV2::State(_)),
        "expected a state error, found {error:?}"
    );
}

/// Measured, not assumed: deleting the `validate` call from `build` leaves this
/// test passing, because the clock rule keeps a defensive fallback that raises
/// the identical error when a definition does not resolve. So this test pins the
/// reported error, and `a_state_the_authority_contract_refuses_is_not_valid_genesis`
/// above is the one that proves the composed call is actually made.
#[test]
fn an_undeclared_facility_definition_reports_a_dangling_reference() {
    let error = mutated(|state| {
        state.facilities = sorted(vec![
            facility(0, "fusion_torus", Some(10)),
            facility(1, "extractor", Some(10)),
            facility(2, "foundry", Some(20)),
            facility(3, "shipyard", None),
        ]);
    });
    assert_eq!(
        error,
        LivingGenesisErrorV2::State(LivingValidationErrorV2::DanglingReference {
            reference: "facility definition",
        })
    );
}

// ------------------------------------------------------ digest and root branch

#[test]
fn the_manifest_digest_is_the_digest_of_the_materialized_state() {
    let genesis = accept(home_manifest());
    let state_digest = genesis.state().digest().expect("the state encodes");

    assert_eq!(genesis.manifest_digest(), state_digest);
    assert_eq!(
        genesis.manifest_digest(),
        state_digest_v2(genesis.state_canonical_bytes())
    );
    assert_eq!(
        genesis.state_canonical_bytes(),
        genesis
            .state()
            .canonical_bytes()
            .expect("the state encodes")
            .as_slice()
    );
}

#[test]
fn the_manifest_digest_hashes_the_state_and_not_the_envelope() {
    let genesis = accept(home_manifest());
    // The envelope bytes are strictly longer than the state bytes, so a digest
    // taken over the wrong document could not coincide with this one.
    assert!(genesis.canonical_bytes().len() > genesis.state_canonical_bytes().len());
    assert_ne!(
        genesis.manifest_digest(),
        state_digest_v2(genesis.canonical_bytes())
    );
}

#[test]
fn the_root_branch_follows_the_published_formula() {
    let genesis = accept(home_manifest());
    assert_eq!(
        genesis.root_branch_id(SEED),
        root_branch_id_v2(catalog().catalog_hash(), SEED, genesis.manifest_digest())
    );
}

#[test]
fn a_different_seed_begins_a_different_root_branch() {
    let genesis = accept(home_manifest());
    assert_ne!(
        genesis.root_branch_id(SEED),
        genesis.root_branch_id([0; 32])
    );
}

#[test]
fn a_changed_genesis_moves_the_root_branch() {
    let first = accept(home_manifest());
    let second = accept(manifest({
        let mut state = home_state();
        let mut world = state.worlds.as_slice()[0].clone();
        world.inventory.alloy = 121;
        state.worlds = sorted(vec![world]);
        state
    }));

    assert_ne!(first.manifest_digest(), second.manifest_digest());
    assert_ne!(first.root_branch_id(SEED), second.root_branch_id(SEED));
}

// ------------------------------------------------------------- the byte path

#[test]
fn a_manifest_round_trips_through_its_canonical_bytes() {
    let built = accept(home_manifest());
    let decoded = decode_living_genesis_manifest_v2(built.canonical_bytes(), &catalog())
        .expect("its own canonical bytes decode");

    assert_eq!(decoded, built);
    assert_eq!(decoded.manifest_digest(), built.manifest_digest());
}

#[test]
fn a_rejected_document_yields_no_value_at_all() {
    let mut document = home_manifest();
    document.state.tick = LivingTickV2(3);
    let bytes = serde_json::to_vec(&document).expect("the fixture serializes");

    let outcome = decode_living_genesis_manifest_v2(&bytes, &catalog());
    assert!(outcome.is_err(), "a rejected manifest must not be built");
    assert_eq!(
        outcome.unwrap_err(),
        LivingGenesisErrorV2::Tick { found: 3 }
    );
}

#[test]
fn noncanonical_bytes_are_refused_by_the_wire_before_any_genesis_rule() {
    let bytes = serde_json::to_vec_pretty(&home_manifest()).expect("the fixture serializes");
    let error = decode_living_genesis_manifest_v2(&bytes, &catalog())
        .expect_err("whitespace is not canonical");
    assert!(
        matches!(error, LivingGenesisErrorV2::Wire(_)),
        "expected a wire error, found {error:?}"
    );
}

#[test]
fn an_unknown_manifest_field_is_refused() {
    let text =
        String::from_utf8(serde_json::to_vec(&home_manifest()).expect("the fixture serializes"))
            .expect("canonical bytes are text");
    let widened = text.replacen('{', "{\"extra\":0,", 1);

    let error = decode_living_genesis_manifest_v2(widened.as_bytes(), &catalog())
        .expect_err("an unknown field is refused");
    assert_eq!(
        error,
        LivingGenesisErrorV2::Wire(LivingWireErrorV2::Json),
        "an unknown field is a decode failure, not a genesis rule"
    );
}

// --------------------------------------------------------------- provenance

#[test]
fn generator_provenance_is_an_identifier_and_a_version() {
    let provenance = LivingGenesisGeneratorV2 {
        generator_id: slug("procedural_home"),
        generator_version: 1,
    };
    let bytes = serde_json::to_vec(&provenance).expect("provenance serializes");
    assert_eq!(
        bytes,
        br#"{"generator_id":"procedural_home","generator_version":1}"#
    );

    let decoded: LivingGenesisGeneratorV2 =
        serde_json::from_slice(&bytes).expect("provenance decodes");
    assert_eq!(decoded, provenance);

    // Provenance is not part of the manifest, so it cannot reach the digest.
    let genesis = accept(home_manifest());
    let manifest_text =
        String::from_utf8(genesis.canonical_bytes().to_vec()).expect("canonical bytes are text");
    assert!(!manifest_text.contains("generator"));
}

#[test]
fn the_state_digest_wrapper_is_the_one_genesis_publishes() {
    let genesis = accept(home_manifest());
    let digest: LivingStateDigestV2 = genesis.manifest_digest();
    assert_eq!(digest.0.len(), 32);
}
