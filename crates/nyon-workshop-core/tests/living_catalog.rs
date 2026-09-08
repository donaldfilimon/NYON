//! Validation and exact-byte tests for the Living Galaxy V2 catalog pack.
//!
//! The numeric content asserted here is normative text from
//! `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` sections 3,
//! 4, 5 and 10, not values read back out of the implementation. The built-in
//! pack's canonical hash is checked against a value computed independently of
//! this crate (see `the_builtin_pack_hash_matches_an_independent_computation`).

use nyon_workshop_core::living::{
    LIVING_MAX_PACK_BYTES_V2, LivingCatalogErrorV2, LivingIndustryEffectV2, LivingWireErrorV2,
    catalog_hash_v2, decode_living_catalog_pack_v2, living_core_pack_v2,
};

const CORE_PACK: &[u8] = include_bytes!("../../../assets/living/core-pack-v2.json");
const MINIMAL_PACK: &[u8] = include_bytes!("fixtures/living-v2/minimal-pack.json");

// ------------------------------------------------------------ built-in pack

#[test]
fn the_builtin_pack_decodes_and_is_byte_identical_to_the_shipped_asset() {
    let pack = living_core_pack_v2().expect("the built-in Living V2 pack validates");
    assert_eq!(pack.canonical_bytes(), CORE_PACK);
    assert_eq!(pack.pack_id(), "nyon_living_core");
    assert_eq!(pack.format_version(), 2);
    assert_eq!(pack.rules_version(), 2);
}

#[test]
fn the_builtin_pack_hash_matches_an_independent_computation() {
    // Computed outside this crate as
    // `sha256(b"NYON-LIVING-PACK-V2\0" + open("assets/living/core-pack-v2.json","rb").read())`.
    // Regenerating this constant from the crate's own output would destroy the
    // evidence it exists to carry.
    const EXPECTED_HEX: &str = include_str!("fixtures/living-v2/core-pack-hash.txt");

    let pack = living_core_pack_v2().expect("the built-in Living V2 pack validates");
    let hash = catalog_hash_v2(pack.canonical_bytes());
    let actual: String = hash.0.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(actual, EXPECTED_HEX.trim());
    assert_eq!(pack.catalog_hash(), hash);
}

#[test]
fn the_builtin_pack_declares_the_three_specified_resources_at_the_storage_cap() {
    let pack = living_core_pack_v2().expect("the built-in Living V2 pack validates");
    let ids: Vec<&str> = pack.resources().iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, ["alloy", "energy", "ore"]);
    for id in ["alloy", "energy", "ore"] {
        let resource = pack.resource(id).expect("declared resource resolves");
        // Section 3: "Inventory is world-local and capped at 10000 units per
        // resource."
        assert_eq!(resource.storage_limit, 10_000, "{id} storage limit");
    }
    assert!(pack.resource("antimatter").is_none());
}

#[test]
fn the_builtin_pack_reproduces_the_facility_table_exactly() {
    let pack = living_core_pack_v2().expect("the built-in Living V2 pack validates");
    let ids: Vec<&str> = pack
        .industries()
        .iter()
        .map(|row| row.id.as_str())
        .collect();
    assert_eq!(
        ids,
        [
            "defense_battery",
            "extractor",
            "foundry",
            "shipyard",
            "solar_array"
        ]
    );

    // Section 3, facility table: alloy cost and build ticks.
    for (id, alloy, ticks) in [
        ("solar_array", 20_u32, 100_u32),
        ("extractor", 30, 150),
        ("foundry", 40, 200),
        ("shipyard", 60, 300),
        ("defense_battery", 40, 200),
    ] {
        let industry = pack.industry(id).expect("declared industry resolves");
        assert_eq!(industry.alloy_cost, alloy, "{id} alloy cost");
        assert_eq!(industry.build_ticks, ticks, "{id} build ticks");
        assert_eq!(industry.slot_cost, 1, "{id} occupies one industrial slot");
    }

    // "Solar array: 8 energy every 10 ticks", with no inputs.
    let solar = pack.industry("solar_array").expect("solar array resolves");
    match &solar.effect {
        LivingIndustryEffectV2::Recipe {
            period_ticks,
            deposit_units,
            inputs,
            outputs,
        } => {
            assert_eq!(*period_ticks, 10);
            assert_eq!(*deposit_units, 0);
            assert!(inputs.as_slice().is_empty());
            assert_eq!(outputs.as_slice().len(), 1);
            assert_eq!(outputs.as_slice()[0].resource.as_str(), "energy");
            assert_eq!(outputs.as_slice()[0].quantity, 8);
        }
        other => panic!("solar array is a recipe, found {other:?}"),
    }

    // "Extractor: 2 energy and 4 deposit units become 4 ore every 10 ticks".
    let extractor = pack.industry("extractor").expect("extractor resolves");
    match &extractor.effect {
        LivingIndustryEffectV2::Recipe {
            period_ticks,
            deposit_units,
            inputs,
            outputs,
        } => {
            assert_eq!(*period_ticks, 10);
            assert_eq!(*deposit_units, 4);
            assert_eq!(inputs.as_slice().len(), 1);
            assert_eq!(inputs.as_slice()[0].resource.as_str(), "energy");
            assert_eq!(inputs.as_slice()[0].quantity, 2);
            assert_eq!(outputs.as_slice().len(), 1);
            assert_eq!(outputs.as_slice()[0].resource.as_str(), "ore");
            assert_eq!(outputs.as_slice()[0].quantity, 4);
        }
        other => panic!("extractor is a recipe, found {other:?}"),
    }

    // "Foundry: 4 energy + 4 ore become 2 alloy every 20 ticks".
    let foundry = pack.industry("foundry").expect("foundry resolves");
    match &foundry.effect {
        LivingIndustryEffectV2::Recipe {
            period_ticks,
            deposit_units,
            inputs,
            outputs,
        } => {
            assert_eq!(*period_ticks, 20);
            assert_eq!(*deposit_units, 0);
            let inputs: Vec<(&str, u32)> = inputs
                .as_slice()
                .iter()
                .map(|row| (row.resource.as_str(), row.quantity))
                .collect();
            assert_eq!(inputs, [("energy", 4), ("ore", 4)]);
            assert_eq!(outputs.as_slice().len(), 1);
            assert_eq!(outputs.as_slice()[0].resource.as_str(), "alloy");
            assert_eq!(outputs.as_slice()[0].quantity, 2);
        }
        other => panic!("foundry is a recipe, found {other:?}"),
    }

    // "Shipyard: builds one hull at a time"; "at most one shipyard".
    let shipyard = pack.industry("shipyard").expect("shipyard resolves");
    assert_eq!(shipyard.max_per_colony, 1);
    match &shipyard.effect {
        LivingIndustryEffectV2::HullAssembly { concurrent_jobs } => {
            assert_eq!(*concurrent_jobs, 1);
        }
        other => panic!("shipyard assembles hulls, found {other:?}"),
    }

    // "Defense battery: 40 HP; 4 damage each combat round", one per colony.
    let battery = pack.industry("defense_battery").expect("battery resolves");
    assert_eq!(battery.max_per_colony, 1);
    match &battery.effect {
        LivingIndustryEffectV2::Defense {
            hit_points,
            damage_per_round,
        } => {
            assert_eq!(*hit_points, 40);
            assert_eq!(*damage_per_round, 4);
        }
        other => panic!("defense battery defends, found {other:?}"),
    }
}

#[test]
fn the_builtin_pack_reproduces_the_hull_table_exactly() {
    let pack = living_core_pack_v2().expect("the built-in Living V2 pack validates");
    let ids: Vec<&str> = pack.hulls().iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, ["colony_ark", "escort", "scout"]);

    // Section 4 hull table: alloy, build ticks, HP, damage.
    for (id, alloy, ticks, hit_points, damage) in [
        ("scout", 10_u32, 50_u32, 10_u32, 0_u32),
        ("colony_ark", 60, 200, 20, 0),
        ("escort", 20, 100, 10, 2),
    ] {
        let hull = pack.hull(id).expect("declared hull resolves");
        assert_eq!(hull.alloy_cost, alloy, "{id} alloy cost");
        assert_eq!(hull.build_ticks, ticks, "{id} build ticks");
        assert_eq!(hull.hit_points, hit_points, "{id} hit points");
        assert_eq!(hull.damage, damage, "{id} damage");
    }
}

#[test]
fn the_builtin_pack_declares_the_ion_storm_as_an_integer_half_batch() {
    let pack = living_core_pack_v2().expect("the built-in Living V2 pack validates");
    let ids: Vec<&str> = pack.hazards().iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, ["ion_storm"]);

    // Section 5: "The built-in ion storm multiplies the route's per-dispatch
    // batch by one-half, rounded down." A ratio, because V2 carries no floats.
    let storm = pack.hazard("ion_storm").expect("ion storm resolves");
    let (numerator, denominator) = storm.effect.freight_batch_scale();
    assert_eq!((numerator, denominator), (1, 2));
    assert_eq!(storm.effect.scale_batch(10), 5);
    assert_eq!(storm.effect.scale_batch(9), 4);
    assert_eq!(storm.effect.scale_batch(1), 0);
}

#[test]
fn the_builtin_pack_declares_the_five_policies_with_their_exact_bonuses() {
    let pack = living_core_pack_v2().expect("the built-in Living V2 pack validates");
    let ids: Vec<&str> = pack.policies().iter().map(|row| row.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "expansionist",
            "guardian",
            "industrialist",
            "neutral",
            "trader"
        ]
    );

    // Section 7: "Expansionist +20 ark/settlement; Industrialist +20
    // extractor/foundry; Guardian +20 escort/battery/defend; Trader +20
    // supply/trade-route establishment. Neutral gets no bonus."
    const EXPANSIONIST: &[(&str, u32)] = &[("build_ark", 20), ("settle", 20)];
    const GUARDIAN: &[(&str, u32)] = &[("build_battery", 20), ("build_escort", 20), ("defend", 20)];
    const INDUSTRIALIST: &[(&str, u32)] = &[("build_extractor", 20), ("build_foundry", 20)];
    const NEUTRAL: &[(&str, u32)] = &[];
    const TRADER: &[(&str, u32)] = &[("establish_trade_route", 20), ("supply", 20)];

    let expected: [(&str, &[(&str, u32)]); 5] = [
        ("expansionist", EXPANSIONIST),
        ("guardian", GUARDIAN),
        ("industrialist", INDUSTRIALIST),
        ("neutral", NEUTRAL),
        ("trader", TRADER),
    ];
    for (id, bonuses) in expected {
        let policy = pack.policy(id).expect("declared policy resolves");
        let actual: Vec<(&str, u32)> = policy
            .bonuses
            .as_slice()
            .iter()
            .map(|row| (row.intent.as_wire_str(), row.bonus))
            .collect();
        assert_eq!(actual, bonuses, "{id} bonuses");
    }

    assert_eq!(pack.policy_bonus("guardian", "build_escort"), 20);
    assert_eq!(pack.policy_bonus("guardian", "build_foundry"), 0);
    assert_eq!(pack.policy_bonus("neutral", "settle"), 0);
    assert_eq!(pack.policy_bonus("no_such_policy", "settle"), 0);
}

// ------------------------------------------------------------- minimal pack

#[test]
fn the_minimal_fixture_pack_validates() {
    let pack = decode_living_catalog_pack_v2(MINIMAL_PACK).expect("the minimal pack validates");
    assert_eq!(pack.pack_id(), "nyon_living_minimal");
    assert_eq!(pack.canonical_bytes(), MINIMAL_PACK);
}

// ------------------------------------------------------------ mutation tests

/// Replace the first occurrence of `from` with `to` in the minimal fixture.
fn mutate(from: &str, to: &str) -> Vec<u8> {
    let text = str::from_utf8(MINIMAL_PACK).expect("the fixture is UTF-8");
    assert!(text.contains(from), "mutation anchor {from:?} is missing");
    text.replacen(from, to, 1).into_bytes()
}

#[test]
fn a_wrong_kind_is_rejected_as_a_kind_error() {
    let bytes = mutate("NYON_LIVING_GALAXY_DATA", "NYON_WORKSHOP_DATA");
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::Kind)
    );
}

#[test]
fn a_v1_rules_version_is_rejected_as_a_rules_version_error() {
    let bytes = mutate(r#""rules_version":2"#, r#""rules_version":1"#);
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::RulesVersion { found: 1 })
    );
}

#[test]
fn a_wrong_format_version_is_rejected_as_a_format_version_error() {
    let bytes = mutate(r#""format_version":2"#, r#""format_version":3"#);
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::FormatVersion { found: 3 })
    );
}

#[test]
fn an_unknown_field_is_rejected() {
    let bytes = mutate(r#""pack_version":1"#, r#""pack_version":1,"extra":0"#);
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::Wire(LivingWireErrorV2::Json))
    );
}

#[test]
fn a_missing_field_is_rejected() {
    let bytes = mutate(r#""pack_version":1,"#, "");
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::Wire(LivingWireErrorV2::Json))
    );
}

#[test]
fn reordered_top_level_fields_are_rejected_as_noncanonical() {
    let bytes = mutate(
        r#""kind":"NYON_LIVING_GALAXY_DATA","format_version":2"#,
        r#""format_version":2,"kind":"NYON_LIVING_GALAXY_DATA""#,
    );
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::Wire(LivingWireErrorV2::NonCanonical))
    );
}

#[test]
fn an_unsorted_definition_array_is_rejected() {
    let bytes = mutate(
        r#""resources":[{"id":"alloy""#,
        r#""resources":[{"id":"zzz_last""#,
    );
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::Wire(LivingWireErrorV2::Json))
    );
}

#[test]
fn a_duplicated_definition_identifier_is_rejected() {
    let bytes = mutate(r#"{"id":"ore","#, r#"{"id":"energy","#);
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::Wire(LivingWireErrorV2::Json))
    );
}

#[test]
fn a_dangling_resource_reference_is_rejected() {
    let bytes = mutate(
        r#"{"resource":"ore","quantity":4}"#,
        r#"{"resource":"unobtainium","quantity":4}"#,
    );
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::DanglingResource)
    );
}

#[test]
fn a_zero_recipe_period_is_rejected() {
    let bytes = mutate(r#""period_ticks":10"#, r#""period_ticks":0"#);
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::ZeroPeriod)
    );
}

#[test]
fn a_zero_build_duration_is_rejected() {
    let bytes = mutate(r#""build_ticks":100"#, r#""build_ticks":0"#);
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::ZeroDuration)
    );
}

#[test]
fn a_storage_limit_above_the_declared_range_is_rejected() {
    let bytes = mutate(r#""storage_limit":10000"#, r#""storage_limit":10001"#);
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::Range)
    );
}

#[test]
fn a_floating_point_value_is_rejected_before_typed_parsing() {
    let bytes = mutate(r#""pack_version":1"#, r#""pack_version":1.0"#);
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::Wire(LivingWireErrorV2::Float))
    );
}

#[test]
fn a_byte_order_mark_is_rejected() {
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend_from_slice(MINIMAL_PACK);
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::Wire(LivingWireErrorV2::ByteOrderMark))
    );
}

#[test]
fn nesting_past_the_declared_depth_is_rejected() {
    // 33 opening brackets, one past LIVING_MAX_CANONICAL_DEPTH_V2.
    let mut bytes = vec![b'['; 33];
    bytes.extend(std::iter::repeat_n(b']', 33));
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::Wire(
            LivingWireErrorV2::DepthExceeded { limit: 32 }
        ))
    );
}

#[test]
fn one_byte_past_the_pack_limit_is_rejected_before_parsing() {
    let bytes = vec![b' '; LIVING_MAX_PACK_BYTES_V2 + 1];
    assert_eq!(
        decode_living_catalog_pack_v2(&bytes),
        Err(LivingCatalogErrorV2::Wire(LivingWireErrorV2::TooLarge {
            actual: LIVING_MAX_PACK_BYTES_V2 + 1,
            limit: LIVING_MAX_PACK_BYTES_V2,
        }))
    );
}
