use std::{fs, mem::size_of, path::Path};

use nyon::{
    classic::{DEFAULT_SEED, GameCommand, RulesV1, ScenarioDraft, Simulation, WORLD_COUNT},
    engine::primitives::Vertex,
    scenario::codec::encode,
};

#[test]
fn classic_facade_preserves_the_frozen_rules_v1_oracles() {
    assert_eq!(DEFAULT_SEED, 0x4947_5731_2026_0902);
    assert_eq!(WORLD_COUNT, 7);
    assert_eq!(RulesV1::default().base_fleet_speed, 23);
    assert_eq!(size_of::<Vertex>(), 36);

    let scenario = ScenarioDraft::factory_default().validated().unwrap();
    assert_eq!(scenario.worlds().len(), 7);
    assert_eq!(
        Simulation::from_scenario(&scenario).canonical_fingerprint(),
        0x67D9_6E98_3D6C_9330
    );

    let canonical = encode(&scenario).unwrap();
    assert_eq!(canonical, encode(&scenario).unwrap());

    let _direct_path: Option<nyon::game::model::GameCommand> = None;
    let _facade_path: Option<GameCommand> = None;
}

#[test]
fn workshop_core_dependency_direction_stays_pure() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join("crates/nyon-workshop-core/Cargo.toml")).unwrap();

    for forbidden in [
        "nyon =",
        "wgpu =",
        "winit =",
        "tokio =",
        "wasm-bindgen =",
        "web-sys =",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "pure core manifest contains forbidden dependency {forbidden}"
        );
    }
    let source_root = root.join("crates/nyon-workshop-core/src");
    let mut pending = vec![source_root];
    let mut checked = 0;
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("rs") {
                continue;
            }
            checked += 1;
            let source = fs::read_to_string(&path).unwrap();
            for forbidden in [
                "use nyon::",
                "use wgpu::",
                "use winit::",
                "use tokio::",
                "use web_sys::",
                "unsafe {",
            ] {
                assert!(
                    !source.contains(forbidden),
                    "{} contains forbidden source boundary {forbidden}",
                    path.display()
                );
            }
        }
    }
    assert!(
        checked >= 7,
        "expected the complete pure-core module boundary"
    );
    let crate_root = fs::read_to_string(root.join("crates/nyon-workshop-core/src/lib.rs")).unwrap();
    assert!(crate_root.starts_with("#![forbid(unsafe_code)]"));
}

#[test]
fn workshop_sha2_assembly_acceleration_excludes_msvc_targets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join("crates/nyon-workshop-core/Cargo.toml")).unwrap();

    assert!(
        manifest.contains("sha2 = \"=0.10.9\""),
        "the portable SHA-256 implementation must remain available on every target"
    );
    assert!(
        manifest.contains(
            "[target.'cfg(not(target_env = \"msvc\"))'.dependencies]\n\
             sha2 = { version = \"=0.10.9\", features = [\"asm\"] }"
        ),
        "SHA-2 assembly acceleration must be target-scoped away from MSVC"
    );
    assert_eq!(
        manifest.matches("features = [\"asm\"]").count(),
        1,
        "SHA-2 assembly acceleration must have one auditable feature owner"
    );
}
