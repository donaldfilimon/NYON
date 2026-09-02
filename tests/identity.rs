use std::{fs, path::PathBuf};

fn project_file(path: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(root.join(path)).unwrap_or_else(|error| panic!("read {path}: {error}"))
}

#[test]
fn cargo_and_web_artifacts_use_the_breaking_nyon_identity() {
    let manifest = project_file("Cargo.toml");
    let build = project_file("tools/build-web.sh");
    let html = project_file("web/index.html");

    assert_eq!(env!("CARGO_PKG_NAME"), "nyon");
    assert!(manifest.contains("[package]\nname = \"nyon\""));
    assert!(manifest.contains("[lib]\nname = \"nyon\""));
    assert!(!manifest.contains("name = \"intergalactic_warfare\""));
    assert!(build.contains("--out-name nyon"));
    assert!(build.contains("release/nyon.wasm"));
    assert!(html.contains("<title>NYON</title>"));
    assert!(html.contains("../dist/nyon.js"));
    assert!(html.contains("NYON startup failed"));
}

#[test]
fn visible_native_identity_and_readme_are_nyon() {
    let app = project_file("src/app.rs");
    let gpu = project_file("src/engine/gpu.rs");
    let main = project_file("src/main.rs");
    let ui = project_file("src/ui.rs");
    let readme = project_file("README.md");

    assert!(app.contains("with_title(\"NYON // Sector Command\")"));
    assert!(gpu.contains("label: Some(\"NYON device\")"));
    assert!(main.contains("nyon::platform::native::run()"));
    assert!(main.contains("NYON startup failed"));
    assert!(ui.contains("pub const TITLE: &str = \"NYON\""));
    assert!(readme.starts_with("# NYON\n\nNYON is "));
}

#[test]
fn integration_tests_link_the_nyon_library_crate() {
    let _: fn() -> nyon::scenario::ScenarioDraft = nyon::scenario::ScenarioDraft::factory_default;
}
