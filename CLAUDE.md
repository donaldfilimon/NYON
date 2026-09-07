# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`AGENTS.md` is the canonical repository guidance and wins on any conflict. It owns
the gate commands, the authority/storage rules, and the frozen GPU and renderer
contracts. Read it first; this file is a routing map for the parts it does not
carry, and deliberately does not restate it.

## Gates

Run the four native commands, the fuzz pair, and the wasm sequence exactly as
`AGENTS.md` lists them. Two things it emphasizes and this file repeats only as a
pointer: `--workspace` is mandatory because `default-members = ["."]` scopes bare
cargo invocations to the root package and skips `crates/nyon-workshop-core`
entirely, and the fuzz directory is its own workspace that no root command reaches.

Focused runs during development:

```sh
cargo test --lib <filter>                                   # unit tests in src/
cargo test --test workshop_ui_sdf                           # one root integration binary
cargo test --test campaign -- <filter> --nocapture          # one case, with output
cargo test -p nyon-workshop-core --test two_system_forge    # core authority crate
cargo run --release                                         # native app
```

`tools/` holds the scripted paths: `build-web.sh` (runs both backend builds),
`check-workshop.sh` (artifact shape, freshness, static contracts, three Rust
suites), `benchmark-workshop.sh`.

## Layout

`src/lib.rs` declares twelve modules under `forbid(unsafe_code)`:

- `game` is Classic truth. `model` and `simulation` hold integer fixed-tick state
  and take no dependency on winit, wgpu, storage, the editor, advisory, clocks or
  presentation. `view` is the read side.
- `crates/nyon-workshop-core` is the separate pure authority/history/archive crate
  behind Galaxy Workshop (`model`, `simulation`, `command`, `history`, `archive`,
  `pack`, `ids`). It must never depend back on `nyon`; a facade test pins that.
- `engine` is the wgpu layer: `gpu`, `backend`, `resources`, `shader`, `render`,
  `render_frame`, `scene_renderer`, `ui_renderer`, `primitives`, `quality`,
  `input`, `time`.
- `ui` is the SDF platform UI. `platform.rs` is the entry with children
  `platform/shell.rs` and `platform/workshop.rs`; alongside it sit
  `platform_sdf`, `platform_projection`, `platform_inspector`, and the split
  native/web backends `platform_native.rs` / `platform_web.rs`. Workshop screens
  live in `workshop*.rs`, plus `creator`, `guide`, `accessibility`,
  `virtual_list`, `start_marker`.
- `workshop` is the client side of the Workshop: `session` and `store`, with the
  browser IndexedDB/localStorage path in `store/web.rs`.
- `app` is the runtime: `core` (and `core/session.rs`), `client_runtime`,
  `input_router`, `durable_exit`, `onboarding`, `settings`.
- `platform` splits the entry points into `native.rs` and `web.rs`; `main.rs` is
  native only, and `src/lib.rs` is also the cdylib the wasm build exports.
- `scenario` (`codec`, `store`) and `preferences` (`store`) own versioned,
  size-capped persistence with the read-only legacy fallback rule.
- `presentation` (`camera`, `interaction`, `picking`, `scene`, `ui`, `workshop`),
  `editor` (`layout`, `render`), `advisory` (`gpu`), and `classic` are the
  remaining view, editor and advisory surfaces. None of them enqueue commands.

## Build topology

Features: `default = ["native-backends", "webgpu-backend"]`. `webgl-backend` is
never part of a default build. `crate-type = ["rlib", "cdylib"]`, so the same lib
is both the native library and the wasm module.

A browser release is a pair of artifacts, not one: `build-web-webgpu.sh` and
`build-web-webgl.sh` each disable default features, build `--lib` into their own
target directory, and emit `dist/webgpu` and `dist/webgl`. `web/loader.js` picks
between them before graphics initialization; `web/` is served beside the artifact
rather than compiled into it. Everything under `assets/` (shaders, the UI atlas,
`workshop/core-pack-v1.json`) does reach the wasm, through `include_str!` and
`include_bytes!`, which is why it counts as a build input for freshness.

`tools/ui-atlas/` is an independent Cargo project with its own lockfile and its
own ignored target dir. It runs `tools/build-ui-atlas.rs` to rasterize Inter plus
the Lucide icon set into the committed `assets/ui/atlas.r8`, `atlas-metrics.json`
and `hashes.json`. `tests/ui_assets.rs` re-hashes those committed files and
validates the metrics, so regenerating the atlas without updating the manifest
fails the root gate.

## Tests

Root `tests/` maps to subsystems: `rules_v1_facade` (the frozen RulesV1 oracle,
seeds and digest in `AGENTS.md`), `campaign`, `scenario`, `scenario_editor`,
`presentation`, `renderer_contracts`, `shaders`, `ui_assets`, `advisory`,
`advisory_gpu` (SKIP-tolerant, not parity evidence), `app`, `identity`,
`preferences`, `usability`, `player_guide`, `browser_contract`, `workshop_web`,
`workshop_store_web`, and the `workshop_*` family covering session, store,
recovery, client, presentation, accessibility and the four `workshop_ui_*` UI
suites. Shared fixtures are in `tests/common/`.

## Docs

`docs/superpowers/specs/2026-09-02-nyon-v2-design.md` is the accepted Workshop
behavior; `docs/superpowers/plans/2026-09-02-nyon-v2.md` is the master plan, with
`reviews/` recording acceptance. Plans state targets, not proof: read manifests,
source and tests for what is actually implemented. `docs/PLAYER-MANUAL.md` and
`README.md` describe shipped controls, storage slots and current limitations.

## Traps

- Tests that read project files resolve `env!("CARGO_MANIFEST_DIR")` at compile
  time and Cargo does not fingerprint it, so after the checkout is moved or
  renamed a stale binary reads the old absolute path and fails with
  `No such file or directory`. Run `cargo clean -p nyon` before treating that as
  a source defect.
- `check-workshop.sh` compares `dist/` mtimes against build inputs. Rebuilding
  out of order, or touching a source after a build, produces a freshness failure
  that is not a code problem. Rebuild with `./tools/build-web.sh`, then re-run it.
- `advisory_gpu` printing `SKIP:` is a pass on a machine without a usable adapter.
  Do not report it as GPU coverage.
