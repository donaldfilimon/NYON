use nyon::game::model::{DEFAULT_SEED, Faction, RulesV1, Tick};
use nyon::game::simulation::Simulation;
use nyon::scenario::codec::{self, CodecError};
use nyon::scenario::store::{
    MemoryScenarioStore, NativePathEnvironment, NativePlatform, NativeScenarioStore, ScenarioStore,
    StoreError, legacy_native_scenario_path, native_scenario_path,
};
use nyon::scenario::{IssueSeverity, ScenarioDraft, ScenarioIssueCode};

#[test]
fn factory_scenario_preserves_the_default_campaign_contract() {
    let draft = ScenarioDraft::factory_default();
    let scenario = draft.validated().expect("factory scenario must validate");
    let simulation = Simulation::from_scenario(&scenario);

    assert_eq!(scenario.seed(), DEFAULT_SEED);
    assert_eq!(scenario.rules().base_fleet_speed, 23);
    assert_eq!(simulation.state().next_tick, Tick(0));
    assert!(simulation.state().fleets.is_empty());
    assert!(simulation.state().active_hazard.is_none());
    assert_eq!(simulation.pending_command_count(), 0);
    assert!(
        simulation
            .state()
            .worlds
            .iter()
            .all(|world| { world.production_remainder == 0 && world.regeneration_remainder == 0 })
    );
    assert_eq!(simulation.canonical_fingerprint(), 0x67D9_6E98_3D6C_9330);
    assert_eq!(
        simulation.scenario_fingerprint(),
        Some(scenario.fingerprint())
    );
    assert_eq!(
        simulation.replay_identity().unwrap().state,
        simulation.canonical_fingerprint()
    );
}

#[test]
fn validation_reports_relational_errors_and_stable_warnings() {
    let mut draft = ScenarioDraft::factory_default();
    draft.rules.win_world_count = 1;
    draft.rules.hazard_duration_ticks = draft.rules.hazard_period_ticks + 1;
    draft.rules.minimum_launch.0 = draft.rules.maximum_energy.0 / 2 + 1;
    draft.rules.field_lower_refund.0 = draft.rules.field_raise_cost.0 + 1;
    draft.worlds[1].x = draft.worlds[0].x;
    draft.worlds[1].y = draft.worlds[0].y;

    let report = draft.validation_report();
    assert!(!report.is_valid());
    assert!(report.issues().iter().any(|issue| {
        issue.severity == IssueSeverity::Error && issue.code == ScenarioIssueCode::WinWorldCount
    }));
    assert!(
        report
            .issues()
            .iter()
            .any(|issue| issue.code == ScenarioIssueCode::DuplicatePosition)
    );

    let warnings = ScenarioDraft::factory_default().validation_report();
    assert!(warnings.is_valid());
    assert_eq!(warnings.warning_count(), 0);
}

#[test]
fn validation_covers_names_boundaries_and_worst_case_arithmetic() {
    let mut draft = ScenarioDraft::factory_default();
    draft.worlds[0].name = "lowercase".into();
    draft.worlds[1].x = -1;
    draft.worlds[2].atmosphere = 11;
    draft.worlds[3].base_regeneration = u64::MAX;
    draft.rules.base_fleet_speed = u64::MAX;
    let report = draft.validation_report();
    for code in [
        ScenarioIssueCode::WorldName,
        ScenarioIssueCode::WorldCoordinate,
        ScenarioIssueCode::WorldFieldLevel,
        ScenarioIssueCode::RegenerationArithmetic,
        ScenarioIssueCode::FleetSpeedArithmetic,
    ] {
        assert!(report.issues().iter().any(|issue| issue.code == code));
    }
}

#[test]
fn every_validation_relation_has_a_blocking_error() {
    macro_rules! assert_error {
        ($code:expr, $change:expr) => {{
            let mut draft = ScenarioDraft::factory_default();
            $change(&mut draft);
            assert!(
                draft
                    .validation_report()
                    .issues()
                    .iter()
                    .any(|issue| issue.severity == IssueSeverity::Error && issue.code == $code)
            );
        }};
    }

    assert_error!(ScenarioIssueCode::TickRate, |draft: &mut ScenarioDraft| {
        draft.rules.tick_hz = 59
    });
    assert_error!(ScenarioIssueCode::AiPeriod, |draft: &mut ScenarioDraft| {
        draft.rules.ai_period_ticks = 0
    });
    assert_error!(
        ScenarioIssueCode::HazardPeriod,
        |draft: &mut ScenarioDraft| { draft.rules.hazard_period_ticks = 0 }
    );
    assert_error!(
        ScenarioIssueCode::HazardDuration,
        |draft: &mut ScenarioDraft| { draft.rules.hazard_duration_ticks = 0 }
    );
    assert_error!(
        ScenarioIssueCode::HazardBoundaryArithmetic,
        |draft: &mut ScenarioDraft| { draft.rules.hazard_first_tick = u64::MAX }
    );
    assert_error!(ScenarioIssueCode::EnergyCap, |draft: &mut ScenarioDraft| {
        draft.rules.maximum_energy.0 = 0
    });
    assert_error!(
        ScenarioIssueCode::DefenseCap,
        |draft: &mut ScenarioDraft| { draft.rules.maximum_defense.0 = 0 }
    );
    assert_error!(
        ScenarioIssueCode::MinimumLaunch,
        |draft: &mut ScenarioDraft| { draft.rules.minimum_launch.0 = 0 }
    );
    assert_error!(
        ScenarioIssueCode::FieldCostRelation,
        |draft: &mut ScenarioDraft| {
            draft.rules.field_raise_cost.0 = draft.rules.maximum_energy.0 + 1
        }
    );
    assert_error!(
        ScenarioIssueCode::WorldEnergy,
        |draft: &mut ScenarioDraft| { draft.worlds[0].energy = draft.rules.maximum_energy.0 + 1 }
    );
    assert_error!(
        ScenarioIssueCode::WorldDefense,
        |draft: &mut ScenarioDraft| { draft.worlds[0].defense = draft.rules.maximum_defense.0 + 1 }
    );
    assert_error!(
        ScenarioIssueCode::MissingUnion,
        |draft: &mut ScenarioDraft| { draft.worlds[0].owner = None }
    );
    assert_error!(
        ScenarioIssueCode::InitialVictory,
        |draft: &mut ScenarioDraft| {
            for world in &mut draft.worlds[..5] {
                world.owner = Some(Faction::Union);
            }
        }
    );
}

#[test]
fn inclusive_boundaries_and_maximum_proven_arithmetic_validate() {
    let mut draft = ScenarioDraft::factory_default();
    draft.rules.win_world_count = 2;
    draft.rules.hazard_duration_ticks = draft.rules.hazard_period_ticks;
    draft.rules.minimum_launch.0 = 1;
    draft.rules.field_lower_refund.0 = draft.rules.field_raise_cost.0;
    draft.worlds[0].name = "ABCDEFGHIJKLMNOPQRSTUVWX".into();
    draft.worlds[0].x = 0;
    draft.worlds[0].y = 10_000;
    draft.worlds[0].atmosphere = 0;
    draft.worlds[0].hydrosphere = 10;
    draft.worlds[0].topology = 0;
    draft.worlds[0].base_output = u64::MAX;
    draft.worlds[0].base_regeneration = u64::MAX - 1_000;
    draft.rules.base_fleet_speed = (u128::from(u64::MAX) * 10_000 / 14_000) as u64;
    assert!(draft.validation_report().is_valid());

    draft.rules.win_world_count = 7;
    assert!(draft.validation_report().is_valid());
}

#[test]
fn zero_base_fleet_speed_is_valid() {
    let mut draft = ScenarioDraft::factory_default();
    draft.rules.base_fleet_speed = 0;

    assert!(draft.validation_report().is_valid());
}

#[test]
fn fingerprint_includes_names_rules_and_every_world_field() {
    let original = ScenarioDraft::factory_default();
    let base = original.validated().unwrap();

    macro_rules! assert_changes {
        ($change:expr) => {{
            let mut changed = original.clone();
            $change(&mut changed);
            assert_ne!(
                base.fingerprint(),
                changed.validated().unwrap().fingerprint()
            );
        }};
    }

    assert_changes!(|draft: &mut ScenarioDraft| draft.seed += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.rules.win_world_count = 4);
    assert_changes!(|draft: &mut ScenarioDraft| draft.rules.ai_period_ticks += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.rules.hazard_first_tick += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.rules.hazard_period_ticks += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.rules.hazard_duration_ticks += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.rules.maximum_energy.0 += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.rules.maximum_defense.0 += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.rules.minimum_launch.0 += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.rules.field_raise_cost.0 += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.rules.field_lower_refund.0 += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.rules.base_fleet_speed += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.worlds[0].name = "ASTER PRIME".into());
    assert_changes!(|draft: &mut ScenarioDraft| draft.worlds[0].x += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.worlds[0].y += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.worlds[3].owner = Some(Faction::Union));
    assert_changes!(|draft: &mut ScenarioDraft| draft.worlds[3].energy += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.worlds[3].defense += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.worlds[3].base_output += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.worlds[3].base_regeneration += 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.worlds[3].atmosphere -= 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.worlds[3].hydrosphere -= 1);
    assert_changes!(|draft: &mut ScenarioDraft| draft.worlds[3].topology -= 1);

    let mut named = original.clone();
    named.worlds[0].name = "ASTER PRIME".into();
    let named = named.validated().unwrap();
    assert_eq!(
        Simulation::from_scenario(&base).canonical_fingerprint(),
        Simulation::from_scenario(&named).canonical_fingerprint()
    );
}

#[test]
fn warnings_have_a_stable_non_blocking_order() {
    let mut draft = ScenarioDraft::factory_default();
    draft.worlds[1].name = draft.worlds[0].name.clone();
    draft.worlds[1].x = draft.worlds[0].x + 1;
    draft.worlds[1].y = draft.worlds[0].y;
    draft.worlds[1].owner = None;
    draft.worlds[2].owner = None;
    draft.worlds[0].energy = draft.rules.minimum_launch.0 * 2 - 1;
    let report = draft.validation_report();
    assert!(report.is_valid());
    assert_eq!(
        report
            .issues()
            .iter()
            .map(|issue| issue.code)
            .collect::<Vec<_>>(),
        vec![
            ScenarioIssueCode::DuplicateName,
            ScenarioIssueCode::ClosePosition,
            ScenarioIssueCode::MissingHelix,
            ScenarioIssueCode::MissingChoir,
            ScenarioIssueCode::NoLaunchableUnion,
        ]
    );
}

#[test]
fn strict_json_round_trips_and_rejects_noncanonical_input() {
    let scenario = ScenarioDraft::factory_default().validated().unwrap();
    let encoded = codec::encode(&scenario).unwrap();
    assert_eq!(codec::decode(&encoded).unwrap(), scenario);

    let unknown = encoded.replacen("{", "{\"unknown\":true,", 1);
    assert!(codec::decode(&unknown).is_err());
    assert!(codec::decode(&" ".repeat(65_537)).is_err());
    assert!(encoded.contains(&format!("\"seed\":\"{DEFAULT_SEED:016X}\"")));

    let leading_zero = encoded.replacen("\"250000\"", "\"0250000\"", 1);
    assert!(codec::decode(&leading_zero).is_err());
    let lowercase_seed = encoded.replacen(&format!("{DEFAULT_SEED:016X}"), "abcdef0123456789", 1);
    assert!(codec::decode(&lowercase_seed).is_err());

    assert!(matches!(
        codec::decode(&" ".repeat(65_536)),
        Err(CodecError::Json(_))
    ));
    let numeric_u64 = encoded.replacen("\"ai_period_ticks\":\"60\"", "\"ai_period_ticks\":60", 1);
    assert!(codec::decode(&numeric_u64).is_err());
    let nested_unknown = encoded.replacen("\"tick_hz\":60", "\"extra\":0,\"tick_hz\":60", 1);
    assert!(codec::decode(&nested_unknown).is_err());
    let mut six_worlds: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    six_worlds["worlds"].as_array_mut().unwrap().pop();
    assert!(codec::decode(&serde_json::to_string(&six_worlds).unwrap()).is_err());
}

#[test]
fn memory_store_is_a_one_slot_test_double() {
    let mut store = MemoryScenarioStore::default();
    assert_eq!(store.load().unwrap(), None);
    store.save("first").unwrap();
    assert_eq!(store.load().unwrap().as_deref(), Some("first"));
    store.fail_next_save("injected failure");
    assert!(store.save("second").is_err());
    assert_eq!(store.load().unwrap().as_deref(), Some("first"));
}

#[test]
fn codec_owner_values_are_explicit() {
    let mut draft = ScenarioDraft::factory_default();
    draft.worlds[3].owner = Some(Faction::Union);
    let json = codec::encode(&draft.validated().unwrap()).unwrap();
    assert!(json.contains("\"owner\":\"UNION\""));
}

#[test]
fn rules_remain_the_existing_value_type() {
    let draft = ScenarioDraft::factory_default();
    let _: RulesV1 = draft.rules;
}

#[test]
fn native_store_round_trips_and_failed_persist_preserves_the_old_slot() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("nested/scenario-v1.json");
    let mut store = NativeScenarioStore::at_path(&path);
    assert_eq!(store.load().unwrap(), None);
    store.save("first").unwrap();
    assert_eq!(store.load().unwrap().as_deref(), Some("first"));

    let failure = store.save_with_before_persist("second", || {
        Err(StoreError::BeforePersist("injected".into()))
    });
    assert!(failure.is_err());
    assert_eq!(store.load().unwrap().as_deref(), Some("first"));
    store.save("second").unwrap();
    assert_eq!(store.load().unwrap().as_deref(), Some("second"));
}

#[test]
fn native_store_rejects_a_slot_above_the_json_byte_limit() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scenario-v1.json");
    std::fs::write(&path, vec![b'X'; 65_537]).unwrap();
    let store = NativeScenarioStore::at_path(path);

    let error = store
        .load()
        .expect_err("an oversized scenario slot must not be loaded");
    assert!(matches!(
        error,
        StoreError::OversizedSlot { max_bytes: 65_536 }
    ));
}

#[test]
fn native_paths_are_derived_from_injected_environment_without_mutation() {
    let environment = NativePathEnvironment {
        home: Some("/users/example".into()),
        appdata: Some("C:/Users/example/AppData/Roaming".into()),
        xdg_data_home: Some("/xdg/data".into()),
    };
    assert!(
        native_scenario_path(NativePlatform::MacOs, &environment)
            .unwrap()
            .ends_with("Library/Application Support/NYON/scenario-v1.json")
    );
    assert!(
        native_scenario_path(NativePlatform::Windows, &environment)
            .unwrap()
            .ends_with("NYON/scenario-v1.json")
    );
    assert_eq!(
        native_scenario_path(NativePlatform::Linux, &environment).unwrap(),
        std::path::PathBuf::from("/xdg/data/nyon/scenario-v1.json")
    );
    let fallback = NativePathEnvironment {
        xdg_data_home: None,
        ..environment.clone()
    };
    assert_eq!(
        native_scenario_path(NativePlatform::Linux, &fallback).unwrap(),
        std::path::PathBuf::from("/users/example/.local/share/nyon/scenario-v1.json")
    );

    assert!(
        legacy_native_scenario_path(NativePlatform::MacOs, &fallback)
            .unwrap()
            .ends_with("Library/Application Support/Intergalactic Warfare/scenario-v1.json")
    );
    assert!(
        legacy_native_scenario_path(NativePlatform::Windows, &environment)
            .unwrap()
            .ends_with("Intergalactic Warfare/scenario-v1.json")
    );
    assert_eq!(
        legacy_native_scenario_path(NativePlatform::Linux, &environment).unwrap(),
        std::path::PathBuf::from("/xdg/data/intergalactic-warfare/scenario-v1.json")
    );
}

#[test]
fn linux_empty_or_relative_xdg_uses_the_absolute_home_fallback() {
    let expected = std::path::PathBuf::from("/users/example/.local/share/nyon/scenario-v1.json");

    for xdg_data_home in [std::path::PathBuf::new(), "relative/data".into()] {
        let environment = NativePathEnvironment {
            home: Some("/users/example".into()),
            appdata: None,
            xdg_data_home: Some(xdg_data_home),
        };

        assert_eq!(
            native_scenario_path(NativePlatform::Linux, &environment).unwrap(),
            expected
        );
    }
}

#[test]
fn native_store_imports_legacy_only_when_the_nyon_slot_is_absent() {
    let directory = tempfile::tempdir().unwrap();
    let primary = directory.path().join("nyon/scenario-v1.json");
    let legacy = directory.path().join("legacy/scenario-v1.json");
    std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    std::fs::write(&legacy, "legacy payload").unwrap();
    let store = NativeScenarioStore::at_paths(&primary, &legacy);

    assert_eq!(store.load().unwrap().as_deref(), Some("legacy payload"));

    std::fs::create_dir_all(primary.parent().unwrap()).unwrap();
    std::fs::write(&primary, "invalid primary payload").unwrap();
    assert_eq!(
        store.load().unwrap().as_deref(),
        Some("invalid primary payload")
    );
}

#[test]
fn native_store_propagates_corrupt_nyon_slot_errors_without_legacy_fallback() {
    let directory = tempfile::tempdir().unwrap();
    let primary = directory.path().join("nyon/scenario-v1.json");
    let legacy = directory.path().join("legacy/scenario-v1.json");
    std::fs::create_dir_all(primary.parent().unwrap()).unwrap();
    std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    std::fs::write(&legacy, "valid legacy payload").unwrap();
    let store = NativeScenarioStore::at_paths(&primary, &legacy);

    std::fs::write(&primary, [0xFF]).unwrap();
    assert!(matches!(
        store.load(),
        Err(StoreError::Io(error)) if error.kind() == std::io::ErrorKind::InvalidData
    ));

    std::fs::write(&primary, vec![b'X'; codec::MAX_JSON_BYTES + 1]).unwrap();
    assert!(matches!(
        store.load(),
        Err(StoreError::OversizedSlot {
            max_bytes: codec::MAX_JSON_BYTES
        })
    ));
}

#[test]
fn native_store_saves_only_to_nyon_and_leaves_legacy_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let primary = directory.path().join("nyon/scenario-v1.json");
    let legacy = directory.path().join("legacy/scenario-v1.json");
    std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    std::fs::write(&legacy, "legacy sentinel").unwrap();
    let mut store = NativeScenarioStore::at_paths(&primary, &legacy);

    store.save("nyon payload").unwrap();

    assert_eq!(std::fs::read_to_string(primary).unwrap(), "nyon payload");
    assert_eq!(std::fs::read_to_string(legacy).unwrap(), "legacy sentinel");
}

#[test]
fn linux_home_fallback_must_be_absolute() {
    let environment = NativePathEnvironment {
        home: Some("relative/home".into()),
        appdata: None,
        xdg_data_home: None,
    };

    assert!(native_scenario_path(NativePlatform::Linux, &environment).is_err());
}
