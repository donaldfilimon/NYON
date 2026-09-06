use std::{fs, path::PathBuf};

fn project_file(path: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(root.join(path)).unwrap_or_else(|error| panic!("read {path}: {error}"))
}

#[test]
fn browser_gpu_startup_is_local_and_marker_only() {
    let app = project_file("src/app.rs");
    let gpu = project_file("src/engine/gpu.rs");
    assert!(app.contains("Rc<RefCell<Option<WebGpuInitResult>>>"));
    assert!(app.contains("wasm_bindgen_futures::spawn_local"));
    assert!(app.contains("AppEvent::GpuInitFinished { generation }"));
    assert!(app.contains("with_append(true)"));
    assert!(app.contains("with_prevent_default(true)"));
    assert!(app.contains("with_focusable(true)"));
    let web_attributes = app
        .split("#[cfg(target_arch = \"wasm32\")]\n            let attributes = attributes")
        .nth(1)
        .expect("wasm window attributes")
        .split("match event_loop.create_window")
        .next()
        .unwrap();
    assert!(!web_attributes.contains("with_inner_size"));
    assert!(!web_attributes.contains("with_min_inner_size"));
    assert!(gpu.contains("descriptor.backends = selected_browser_backends()"));
    assert!(gpu.contains("wgpu::Backends::BROWSER_WEBGPU"));
    assert!(gpu.contains("wgpu::Backends::GL"));
    assert!(gpu.contains("feature = \"webgl-backend\""));
}

#[test]
fn browser_store_and_static_loader_use_the_frozen_contract() {
    let platform = project_file("src/platform/web.rs");
    let html = project_file("web/index.html");
    assert!(platform.contains("nyon.scenario.v1"));
    assert!(platform.contains("intergalactic-warfare.scenario.v1"));
    assert!(platform.contains("local_storage()"));
    assert!(platform.contains("load_primary_or_legacy("));
    assert!(platform.contains("get_item(LOCAL_STORAGE_KEY)"));
    assert!(platform.contains("get_item(LEGACY_LOCAL_STORAGE_KEY)"));
    assert!(platform.contains("set_item(LOCAL_STORAGE_KEY, payload)"));
    assert!(!platform.contains("set_item(LEGACY_LOCAL_STORAGE_KEY"));
    let loader = project_file("web/loader.js");
    assert!(html.contains("type=\"module\" src=\"./loader.js\""));
    assert!(!html.contains("../dist/nyon.js"));
    assert!(loader.contains("../dist/webgpu/nyon.js"));
    assert!(loader.contains("../dist/webgl/nyon.js"));
    assert!(loader.contains("nyon:graphics-ready"));
    assert!(loader.contains("nyon:graphics-failed"));
}

#[test]
fn web_build_is_pinned_and_generated_outputs_are_ignored() {
    let manifest = project_file("Cargo.toml");
    let wrapper = project_file("tools/build-web.sh");
    let webgpu = project_file("tools/build-web-webgpu.sh");
    let webgl = project_file("tools/build-web-webgl.sh");
    let ignore = project_file(".gitignore");
    assert!(manifest.contains("crate-type = [\"rlib\", \"cdylib\"]"));
    for dependency in [
        "wasm-bindgen = \"=0.2.127\"",
        "wasm-bindgen-futures = \"=0.4.77\"",
        "console_error_panic_hook = \"=0.1.7\"",
        "console_log = \"=1.1.0\"",
        "version = \"=0.3.104\"",
        "web-time = \"=1.1.0\"",
    ] {
        assert!(manifest.contains(dependency), "missing {dependency}");
    }
    assert!(wrapper.contains("build-web-webgpu.sh"));
    assert!(wrapper.contains("build-web-webgl.sh"));
    for build in [&webgpu, &webgl] {
        assert!(build.contains("nightly-2026-09-01"));
        assert!(build.contains("WASM_BINDGEN_VERSION=\"0.2.127\""));
        assert!(build.contains("rustup target add"));
        assert!(build.contains("target/tools/wasm-bindgen-cli-"));
        assert!(build.contains("--out-name nyon"));
    }
    assert!(ignore.lines().any(|line| line == "/target/"));
    assert!(ignore.lines().any(|line| line == "/dist/"));
}
