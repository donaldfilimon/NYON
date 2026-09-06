use std::{fs, path::PathBuf, process::Command};

#[allow(dead_code)]
#[path = "../src/engine/backend.rs"]
mod backend;

use backend::BackendKind;

fn project_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn project_file(path: &str) -> String {
    fs::read_to_string(project_path(path)).unwrap_or_else(|error| panic!("read {path}: {error}"))
}

fn assert_in_order(source: &str, first: &str, second: &str) {
    let first_index = source
        .find(first)
        .unwrap_or_else(|| panic!("missing first marker: {first}"));
    let second_index = source
        .find(second)
        .unwrap_or_else(|| panic!("missing second marker: {second}"));
    assert!(
        first_index < second_index,
        "expected {first:?} before {second:?}"
    );
}

#[test]
fn backend_diagnostics_are_derived_from_wgpu_and_stably_labeled() {
    let cases = [
        (wgpu::Backend::Metal, BackendKind::Metal, "METAL"),
        (wgpu::Backend::Dx12, BackendKind::Dx12, "DX12"),
        (wgpu::Backend::Vulkan, BackendKind::Vulkan, "VULKAN"),
        (wgpu::Backend::Gl, BackendKind::Gl, "GL"),
        (wgpu::Backend::BrowserWebGpu, BackendKind::WebGpu, "WEBGPU"),
    ];

    for (wgpu_backend, expected, label) in cases {
        assert_eq!(BackendKind::from_wgpu(wgpu_backend), Some(expected));
        assert_eq!(expected.label(), label);
    }
    assert_eq!(BackendKind::WebGl2Low.label(), "WEBGL2 LOW");
    assert!(BackendKind::WebGl2Low.is_low_capability());
    assert_eq!(BackendKind::from_wgpu(wgpu::Backend::Noop), None);

    let source = project_file("src/engine/backend.rs");
    assert!(source.contains("#[cfg(target_arch = \"wasm32\")]"));
    assert!(source.contains("Some(Self::WebGl2Low)"));
    assert!(source.contains("adapter.get_info().backend"));
}

#[test]
fn browser_semantic_mirror_carries_shared_bounds_and_hides_unmaterialized_actions() {
    let source = project_file("src/ui/platform_web.rs");
    assert!(source.contains("data-nyon-bounds"));
    assert!(source.contains("if !node.visible"));
    assert!(source.contains("aria-hidden"));
    assert!(source.contains("tabindex"));
}

#[test]
fn browser_modal_source_keeps_visible_enabled_ancestry_to_actual_dialogs() {
    use nyon::ui::{
        accessibility::SemanticNode,
        platform::build_workshop_platform_frame_for_view,
        workshop::{CreatorTool, WorkshopUiContext, WorkshopUiModel},
        workshop_layout::WorkshopLayout,
        workshop_view::WorkshopViewState,
    };
    use nyon_workshop_core::{
        BatchLocalId, CreatorBatchV1, CreatorOpV1, GalaxyPointV1, ObjectName, WorkshopHistory,
        WorkshopTick,
    };

    fn assert_browser_path(
        node: &SemanticNode,
        target: &str,
        ancestors_visible: bool,
        ancestors_enabled: bool,
    ) -> bool {
        let path_visible = ancestors_visible && node.visible;
        let path_enabled = ancestors_enabled && node.enabled;
        if node.id.as_str() == target {
            assert!(
                path_visible,
                "browser would inherit aria-hidden for {target}"
            );
            assert!(
                path_enabled,
                "browser would inherit aria-disabled for {target}"
            );
            return true;
        }
        node.children
            .iter()
            .any(|child| assert_browser_path(child, target, path_visible, path_enabled))
    }

    let catalog = nyon_workshop_core::decode_catalog_pack(include_bytes!(
        "../assets/workshop/core-pack-v1.json"
    ))
    .unwrap();
    let mut history = WorkshopHistory::from_seed_u64(catalog, 73);
    history
        .submit(CreatorBatchV1 {
            expected_cursor: None,
            expected_tick: WorkshopTick(0),
            operations: vec![CreatorOpV1::CreateSystem {
                local: BatchLocalId(0),
                name: ObjectName::new("Browser modal system").unwrap(),
                position: GalaxyPointV1::new(0, 0).unwrap(),
            }],
        })
        .unwrap();
    let target = *history.active_state().systems.keys().next().unwrap();
    let session = nyon::workshop::session::WorkshopSession::new(history);
    let layout = WorkshopLayout::resolve(glam::Vec2::new(1280.0, 480.0), 1.0).unwrap();
    for (context, dialog_id) in [
        (
            WorkshopUiContext {
                creator_form: Some(CreatorTool::CreateWorld),
                ..WorkshopUiContext::default()
            },
            "workshop.creator-dialog",
        ),
        (
            WorkshopUiContext {
                pending_removal: Some(target),
                ..WorkshopUiContext::default()
            },
            "workshop.removal-dialog",
        ),
    ] {
        let model = WorkshopUiModel::build(session.snapshot(), context);
        let frame = build_workshop_platform_frame_for_view(
            &model,
            layout.clone(),
            &WorkshopViewState::default(),
            None,
        );
        assert!(
            assert_browser_path(&frame.semantics.root, dialog_id, true, true),
            "missing browser dialog {dialog_id}"
        );
        for control in frame.controls.iter().filter(|control| control.enabled) {
            assert!(frame.semantics.nodes_depth_first().into_iter().any(|node| {
                node.visible && node.enabled && node.action_id.as_ref() == Some(&control.action_id)
            }));
        }
    }

    let mirror = project_file("src/ui/platform_web.rs");
    assert!(mirror.contains("let interactable = node.enabled && modal_allowed && node.visible"));
    assert!(mirror.contains("if !node.visible"));
}

#[test]
fn sdf_installation_preserves_browser_semantic_identity_and_offline_assets() {
    use nyon::{
        app::client_runtime::ClientScreen,
        engine::primitives::PrimitiveBatch,
        ui::{AtlasMetrics, UiBatch, platform::*},
    };
    let frame = build_shell_platform_frame(ShellPlatformInput {
        screen: ClientScreen::Settings,
        capabilities: &[],
        credits_visible: false,
        recovery_message: None,
        continue_available: false,
        backend: None,
        preferences: Default::default(),
        viewport: glam::Vec2::new(723.0, 802.0),
        focused: None,
    });
    let before = frame.semantics.clone();
    let mut ui = UiBatch::default();
    let mut overlay = PrimitiveBatch::default();
    let mut metrics = AtlasMetrics::embedded().unwrap();
    install_platform_batches(&frame, &metrics, &mut ui, &mut overlay).unwrap();
    assert_eq!(frame.semantics, before);
    metrics.entries.clear();
    assert!(install_platform_batches(&frame, &metrics, &mut ui, &mut overlay).is_err());
    assert_eq!(frame.semantics, before);
    for node in before.nodes_depth_first() {
        if node.visible
            && node.enabled
            && let Some(action) = &node.action_id
        {
            assert!(frame.action(action).is_some());
        }
    }
    // Adapter and canvas linkage remain source contracts, not live DOM evidence.
    let mirror = project_file("src/ui/platform_web.rs");
    assert!(mirror.contains("element.set_attribute(\"id\", node.id.as_str())"));
    assert!(mirror.contains("element.set_attribute(\"data-nyon-action\", action_id.as_str())"));
    assert!(mirror.contains("element.set_attribute(\"tabindex\", \"-1\")"));
    assert!(mirror.contains("if !node.visible"));
    let app = project_file("src/app.rs");
    assert!(app.contains("install_platform_batches("));
    assert!(app.contains("install_guide_batches("));
    assert!(app.contains("self.sync_semantics(&frame.semantics)"));
    for path in ["src/ui/platform.rs", "src/ui/platform_sdf.rs", "src/ui.rs"] {
        let source = project_file(path);
        assert!(
            !source.contains("https://")
                && !source.contains("http://")
                && !source.contains("fetch(")
        );
    }
    assert!(project_file("src/ui.rs").contains("include_bytes!"));
}

#[test]
fn browser_builds_are_pinned_backend_specific_and_emit_exact_paths() {
    let webgpu = project_file("tools/build-web-webgpu.sh");
    let webgl = project_file("tools/build-web-webgl.sh");

    for script in [&webgpu, &webgl] {
        assert!(script.contains("nightly-2026-09-01"));
        assert!(script.contains("WASM_BINDGEN_VERSION=\"0.2.127\""));
        assert!(script.contains("rustup target add"));
        assert!(script.contains("--no-default-features"));
        assert!(script.contains("--no-typescript"));
        assert!(script.contains("--out-name nyon"));
        assert!(script.contains("nyon.js"));
        assert!(script.contains("nyon_bg.wasm"));
    }

    assert!(webgpu.contains("BACKEND_FEATURE=\"webgpu-backend\""));
    assert!(webgpu.contains("dist/webgpu"));
    assert!(webgpu.contains("target/workshop-web/webgpu"));
    assert!(!webgpu.contains("BACKEND_FEATURE=\"webgl-backend\""));

    assert!(webgl.contains("BACKEND_FEATURE=\"webgl-backend\""));
    assert!(webgl.contains("dist/webgl"));
    assert!(webgl.contains("target/workshop-web/webgl"));
    assert!(!webgl.contains("BACKEND_FEATURE=\"webgpu-backend\""));
}

#[test]
fn loader_preflights_selects_then_retries_once_without_acquiring_a_context() {
    let loader = project_file("web/loader.js");

    assert!(loader.contains("params.get(\"backend\") === \"webgl2\""));
    assert!(loader.contains("navigatorObject.gpu.requestAdapter"));
    assert!(loader.contains("../dist/webgpu/nyon.js"));
    assert!(loader.contains("../dist/webgl/nyon.js"));
    assert!(loader.contains("nyon:graphics-ready"));
    assert!(loader.contains("nyon:graphics-failed"));
    assert!(!loader.contains("getContext("));

    let startup = loader
        .split("export async function startNyon")
        .nth(1)
        .expect("startNyon implementation");
    assert_in_order(
        startup,
        "if (forcedBackend(search) === \"webgl2\")",
        "const preflight = await runPreflight()",
    );
    assert_in_order(
        startup,
        "const preflight = await runPreflight()",
        "return await startArtifact(\"webgpu\", preflight)",
    );

    let webgpu_failure = startup
        .split("} catch (webGpuError) {")
        .nth(1)
        .expect("WebGPU failure path")
        .split("} catch (webGlError) {")
        .next()
        .expect("WebGL fallback path");
    assert_in_order(
        webgpu_failure,
        "teardown()",
        "return await startArtifact(\"webgl2\", fallbackReason)",
    );
    assert_eq!(
        webgpu_failure.matches("startArtifact(\"webgl2\"").count(),
        1,
        "WebGPU failure must attempt WebGL exactly once"
    );
}

#[test]
fn loader_state_machine_handles_forcing_preflight_and_fallback() {
    let node = match Command::new("node").arg("--version").output() {
        Ok(output) if output.status.success() => "node",
        _ => {
            eprintln!("SKIP: node is unavailable; source-level loader contracts still ran");
            return;
        }
    };
    let loader = project_path("web/loader.js");
    let script = r#"
      const loader = await import(`file://${process.env.NYON_LOADER_PATH}`);
      const assert = (condition, message) => { if (!condition) throw new Error(message); };
      const quiet = () => {};

      let eventDetail = loader.normalizeGraphicsEventDetail("device request failed");
      assert(eventDetail.backend === undefined, "string detail has no structured backend");
      assert(eventDetail.message === "device request failed", "string detail is a failure message");
      eventDetail = loader.normalizeGraphicsEventDetail({
        backend: "WEBGPU",
        message: "structured failure",
      });
      assert(eventDetail.backend === "WEBGPU", "structured detail backend");
      assert(eventDetail.message === "structured failure", "structured detail message");

      let preflight = await loader.preflightWebGpu({});
      assert(!preflight.ok && preflight.detail.includes("navigator.gpu"), "missing WebGPU API");
      preflight = await loader.preflightWebGpu({ gpu: { requestAdapter: async () => null } });
      assert(!preflight.ok && preflight.detail.includes("no adapter"), "null WebGPU adapter");
      preflight = await loader.preflightWebGpu({
        gpu: { requestAdapter: async () => ({ info: { vendor: "test-vendor" } }) },
      });
      assert(preflight.ok && preflight.detail.includes("test-vendor"), "successful adapter preflight");

      let calls = [];
      let preflightCalls = 0;
      let result = await loader.startNyon({
        search: "?backend=webgl2",
        preflight: async () => { preflightCalls += 1; return { ok: true, detail: "unused" }; },
        loadArtifact: async (backend) => { calls.push(backend); },
        report: quiet,
      });
      assert(result.backend === "webgl2", "forced fallback result");
      assert(calls.join(",") === "webgl2", "forced fallback attempts only WebGL");
      assert(preflightCalls === 0, "forced fallback skips WebGPU preflight");

      calls = [];
      result = await loader.startNyon({
        preflight: async () => ({ ok: false, detail: "no adapter" }),
        loadArtifact: async (backend) => { calls.push(backend); },
        report: quiet,
      });
      assert(result.backend === "webgl2", "failed preflight result");
      assert(calls.join(",") === "webgl2", "failed preflight never imports WebGPU");

      calls = [];
      let teardowns = 0;
      result = await loader.startNyon({
        preflight: async () => ({ ok: true, detail: "adapter ok" }),
        loadArtifact: async (backend) => {
          calls.push(backend);
          if (backend === "webgpu") throw new Error("device failed");
        },
        teardown: () => { teardowns += 1; },
        report: quiet,
      });
      assert(result.backend === "webgl2", "WebGPU failure falls back");
      assert(calls.join(",") === "webgpu,webgl2", "fallback order and count");
      assert(teardowns === 1, "incomplete WebGPU surface is torn down once");

      calls = [];
      let failure;
      try {
        await loader.startNyon({
          preflight: async () => ({ ok: true, detail: "adapter detail" }),
          loadArtifact: async (backend) => { calls.push(backend); throw new Error(`${backend} failed`); },
          teardown: quiet,
          report: quiet,
        });
      } catch (error) {
        failure = error;
      }
      assert(failure instanceof loader.NyonStartupError, "both failures are typed");
      assert(calls.join(",") === "webgpu,webgl2", "both paths run once");
      assert(failure.message.includes("adapter detail"), "failure retains adapter detail");
    "#;

    let output = Command::new(node)
        .args(["--input-type=module", "--eval", script])
        .env("NYON_LOADER_PATH", loader)
        .output()
        .expect("run loader contract with node");
    assert!(
        output.status.success(),
        "loader contract failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn browser_shell_exposes_visible_semantic_recovery_status() {
    let html = project_file("web/index.html");
    assert!(html.contains("id=\"nyon-status\""));
    assert!(html.contains("role=\"status\""));
    assert!(html.contains("aria-live=\"polite\""));
    assert!(html.contains("aria-atomic=\"true\""));
    assert!(html.contains("id=\"nyon-backend\""));
    assert!(html.contains("id=\"nyon-message\""));
    assert!(html.contains("type=\"module\" src=\"./loader.js\""));
    assert!(!html.contains("../dist/nyon.js"));
}

/// Source-level CSS contract only. Live browser acceptance separately verifies
/// the rendered placement at the representative compact viewport widths.
#[test]
fn ready_diagnostic_collapses_without_covering_compact_workshop_controls() {
    let html = project_file("web/index.html");
    let ready_rule = html
        .split_once("#nyon-status[data-state=\"ready\"] {")
        .expect("ready status CSS rule")
        .1
        .split_once('}')
        .expect("ready status CSS rule terminator")
        .0;
    let error_rule = html
        .split_once("#nyon-status[data-state=\"error\"] {")
        .expect("error status CSS rule")
        .1
        .split_once('}')
        .expect("error status CSS rule terminator")
        .0;

    // A ready announcement remains in the live region, but uses a
    // viewport-independent visually hidden treatment.
    assert!(ready_rule.contains("inline-size: 1px"));
    assert!(ready_rule.contains("block-size: 1px"));
    assert!(ready_rule.contains("overflow: hidden"));
    assert!(ready_rule.contains("clip-path: inset(50%)"));
    assert!(ready_rule.contains("padding: 0"));
    assert!(ready_rule.contains("border: 0"));
    assert!(ready_rule.contains("background: transparent"));
    assert!(ready_rule.contains("box-shadow: none"));
    assert!(ready_rule.contains("pointer-events: none"));

    // Startup failure remains a visible, centered, actionable diagnostic.
    assert!(error_rule.contains("top: 50%"));
    assert!(error_rule.contains("left: 50%"));
    assert!(error_rule.contains("transform: translate(-50%, -50%)"));
    assert!(error_rule.contains("pointer-events: auto"));
    assert!(!error_rule.contains("clip-path"));

    let loader = project_file("web/loader.js");
    assert!(loader.contains("state === \"error\" ? \"alert\" : \"status\""));
}

/// The DOM mirror rebuilds every element whenever the semantic signature
/// changes, and the signature covers node values, so a rebuild happens on every
/// accepted keystroke. Keyboard editing is only possible if that rebuild hands
/// the caret back.
///
/// This is a source contract, not browser evidence. There is no wasm runtime
/// harness in this repository, so the browser keyboard row stays pending.
#[test]
fn dom_mirror_returns_the_caret_after_it_rebuilds_editable_nodes() {
    let mirror = project_file("src/ui/platform_web.rs");

    assert_in_order(
        &mirror,
        "self.capture_focus()",
        "self.root.set_text_content(None)",
    );
    assert_in_order(&mirror, "append_node(", "self.restore_focus(");
    assert!(mirror.contains("fn capture_focus(&self) -> Option<FocusRestore>"));
    assert!(mirror.contains("self.root.contains(Some(&active))"));
    assert!(mirror.contains("input.selection_start()"));
    assert!(mirror.contains("input.selection_end()"));
    assert!(mirror.contains("set_selection_range(start.min(length), end.min(length))"));

    // Select-all belongs to entry, never to a rebuild the user did not ask for.
    let restore = mirror
        .split_once("fn restore_focus")
        .expect("restore_focus is defined")
        .1;
    let restore = restore
        .split_once("\n    pub fn drain_actions")
        .expect("restore_focus precedes drain_actions")
        .0;
    assert!(!restore.contains("select()"));

    // The rebuilt element is seeded from authoritative state, so the live input
    // value is the truth and needs no guess about what the browser replaced.
    assert!(mirror.contains("push_back((action_id.clone(), input.value()))"));
    assert!(!mirror.contains("normalize_select_all_input"));
    assert!(!project_file("src/ui/accessibility.rs").contains("normalize_select_all_input"));
}
