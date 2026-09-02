use intergalactic_warfare::{
    preferences::{
        MAX_JSON_BYTES, PreferencesCodecError, PreferencesFailure, UiScale, UserPreferencesV1,
        decode, encode, load_or_default, save_or_default,
        store::{
            LOCAL_STORAGE_KEY, MemoryPreferencesStore, NativePathEnvironment, NativePlatform,
            NativePreferencesStore, PreferencesStore, PreferencesStoreError,
            native_preferences_path,
        },
    },
    presentation::{GraphicsQuality, MotionPreference, PresentationPreferences},
    scenario::store::{NativeScenarioStore, ScenarioStore, native_scenario_path},
};

#[test]
fn defaults_are_conservative_and_convert_to_presentation_preferences() {
    let preferences = UserPreferencesV1::default();
    assert_eq!(preferences.ui_scale, UiScale::Percent100);
    assert_eq!(preferences.ui_scale.percent(), 100);
    assert_eq!(preferences.ui_scale.factor(), 1.0);
    assert_eq!(preferences.motion, MotionPreference::Full);
    assert!(!preferences.high_contrast);
    assert_eq!(preferences.graphics_quality, GraphicsQuality::Auto);
    assert!(!preferences.onboarding_completed);
    assert_eq!(
        preferences.presentation(),
        PresentationPreferences::default()
    );
}

#[test]
fn every_supported_value_round_trips_through_the_versioned_record() {
    for ui_scale in [
        UiScale::Percent85,
        UiScale::Percent100,
        UiScale::Percent115,
        UiScale::Percent130,
    ] {
        for motion in [MotionPreference::Full, MotionPreference::Reduced] {
            for graphics_quality in [
                GraphicsQuality::Auto,
                GraphicsQuality::Low,
                GraphicsQuality::High,
            ] {
                let preferences = UserPreferencesV1 {
                    ui_scale,
                    motion,
                    high_contrast: true,
                    graphics_quality,
                    onboarding_completed: true,
                };
                let payload = encode(&preferences).unwrap();
                assert!(payload.len() <= MAX_JSON_BYTES);
                assert_eq!(decode(&payload).unwrap(), preferences);
                assert!(payload.contains("\"format_version\":1"));
            }
        }
    }
}

#[test]
fn malformed_unknown_and_oversized_records_each_fall_back_once() {
    let valid = encode(&UserPreferencesV1::default()).unwrap();
    let malformed = "not-json".to_string();
    let unknown_field = valid.replacen("{", "{\"unknown\":true,", 1);
    let unknown_version = valid.replacen("\"format_version\":1", "\"format_version\":2", 1);
    let invalid_scale = valid.replacen("\"ui_scale_percent\":100", "\"ui_scale_percent\":99", 1);
    let oversized = " ".repeat(MAX_JSON_BYTES + 1);

    for payload in [
        malformed,
        unknown_field,
        unknown_version,
        invalid_scale,
        oversized,
    ] {
        let outcome = load_or_default(&MemoryPreferencesStore::with_slot(payload));
        assert_eq!(outcome.preferences, UserPreferencesV1::default());
        assert!(outcome.recoverable_failure.is_some());
        assert!(outcome.recoverable_message().is_some());
    }
}

#[test]
fn codec_reports_typed_version_scale_and_size_failures() {
    let valid = encode(&UserPreferencesV1::default()).unwrap();
    let unknown_version = valid.replacen("\"format_version\":1", "\"format_version\":7", 1);
    assert!(matches!(
        decode(&unknown_version),
        Err(PreferencesCodecError::Version)
    ));

    let invalid_scale = valid.replacen("\"ui_scale_percent\":100", "\"ui_scale_percent\":101", 1);
    assert!(matches!(
        decode(&invalid_scale),
        Err(PreferencesCodecError::UiScale(101))
    ));
    assert!(matches!(
        decode(&"X".repeat(MAX_JSON_BYTES + 1)),
        Err(PreferencesCodecError::TooLarge)
    ));
}

#[test]
fn missing_slot_is_a_clean_default_while_denial_is_one_typed_failure() {
    let empty = load_or_default(&MemoryPreferencesStore::default());
    assert_eq!(empty.preferences, UserPreferencesV1::default());
    assert!(empty.recoverable_failure.is_none());

    let mut denied = MemoryPreferencesStore::default();
    denied.deny_load("injected denial");
    let outcome = load_or_default(&denied);
    assert_eq!(outcome.preferences, UserPreferencesV1::default());
    assert!(matches!(
        outcome.recoverable_failure,
        Some(PreferencesFailure::Store(PreferencesStoreError::Browser(_)))
    ));
}

#[test]
fn failed_write_returns_defaults_and_preserves_the_previous_record() {
    let original = UserPreferencesV1::default();
    let changed = UserPreferencesV1 {
        ui_scale: UiScale::Percent130,
        motion: MotionPreference::Reduced,
        high_contrast: true,
        graphics_quality: GraphicsQuality::Low,
        onboarding_completed: true,
    };
    let mut store = MemoryPreferencesStore::default();
    assert!(
        save_or_default(&mut store, &original)
            .recoverable_failure
            .is_none()
    );
    store.fail_next_save("quota exceeded");

    let failed = save_or_default(&mut store, &changed);
    assert_eq!(failed.preferences, UserPreferencesV1::default());
    assert_eq!(
        failed.recoverable_message().as_deref(),
        Some("preference storage failed before atomic persist: quota exceeded")
    );
    assert_eq!(load_or_default(&store).preferences, original);
}

#[test]
fn native_preference_slot_is_beside_but_distinct_from_scenario_data() {
    let environments = [
        (
            NativePlatform::MacOs,
            NativePathEnvironment {
                home: Some("/users/example".into()),
                appdata: None,
                xdg_data_home: None,
            },
        ),
        (
            NativePlatform::Windows,
            NativePathEnvironment {
                home: None,
                appdata: Some("C:/Users/example/AppData/Roaming".into()),
                xdg_data_home: None,
            },
        ),
        (
            NativePlatform::Linux,
            NativePathEnvironment {
                home: Some("/users/example".into()),
                appdata: None,
                xdg_data_home: Some("/xdg/data".into()),
            },
        ),
    ];

    for (platform, environment) in environments {
        let preferences = native_preferences_path(platform, &environment).unwrap();
        let scenario = native_scenario_path(platform, &environment).unwrap();
        assert_eq!(preferences.parent(), scenario.parent());
        assert_eq!(preferences.file_name().unwrap(), "preferences-v1.json");
        assert_eq!(scenario.file_name().unwrap(), "scenario-v1.json");
        assert_ne!(preferences, scenario);
    }
}

#[test]
fn native_store_round_trips_and_atomic_failure_preserves_the_old_slot() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("nested/preferences-v1.json");
    let mut store = NativePreferencesStore::at_path(&path);
    let original = encode(&UserPreferencesV1::default()).unwrap();
    let changed = encode(&UserPreferencesV1 {
        onboarding_completed: true,
        ..UserPreferencesV1::default()
    })
    .unwrap();

    assert_eq!(store.load().unwrap(), None);
    store.save(&original).unwrap();
    let failure = store.save_with_before_persist(&changed, || {
        Err(PreferencesStoreError::BeforePersist("injected".into()))
    });
    assert!(failure.is_err());
    assert_eq!(store.load().unwrap().as_deref(), Some(original.as_str()));
    store.save(&changed).unwrap();
    assert_eq!(store.load().unwrap().as_deref(), Some(changed.as_str()));
}

#[test]
fn native_store_caps_reads_before_parsing_and_rejects_oversized_writes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("preferences-v1.json");
    std::fs::write(&path, vec![b'X'; MAX_JSON_BYTES + 1]).unwrap();
    let mut store = NativePreferencesStore::at_path(path);

    assert!(matches!(
        store.load(),
        Err(PreferencesStoreError::OversizedSlot {
            max_bytes: MAX_JSON_BYTES
        })
    ));
    assert!(matches!(
        store.save(&"X".repeat(MAX_JSON_BYTES + 1)),
        Err(PreferencesStoreError::OversizedSlot {
            max_bytes: MAX_JSON_BYTES
        })
    ));
}

#[test]
fn preference_failures_cannot_read_or_overwrite_the_scenario_slot() {
    let directory = tempfile::tempdir().unwrap();
    let scenario_path = directory.path().join("scenario-v1.json");
    let preferences_path = directory.path().join("preferences-v1.json");
    let mut scenario_store = NativeScenarioStore::at_path(&scenario_path);
    scenario_store.save("scenario sentinel").unwrap();

    std::fs::write(&preferences_path, vec![b'X'; MAX_JSON_BYTES + 1]).unwrap();
    let preference_store = NativePreferencesStore::at_path(&preferences_path);
    let outcome = load_or_default(&preference_store);
    assert_eq!(outcome.preferences, UserPreferencesV1::default());
    assert!(outcome.recoverable_failure.is_some());
    assert_eq!(
        scenario_store.load().unwrap().as_deref(),
        Some("scenario sentinel")
    );
}

#[test]
fn browser_store_source_uses_only_the_frozen_preference_key() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/preferences/store.rs"),
    )
    .unwrap();
    assert_eq!(LOCAL_STORAGE_KEY, "intergalactic-warfare.preferences.v1");
    assert!(source.contains("get_item(LOCAL_STORAGE_KEY)"));
    assert!(source.contains("set_item(LOCAL_STORAGE_KEY, payload)"));
    assert!(!source.contains("intergalactic-warfare.scenario.v1"));
}
