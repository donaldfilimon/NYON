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
suites), `benchmark-workshop.sh`. `.remember/` and `.superpowers/` at the root are
session tooling, not project files.

## Layout

`src/lib.rs` declares twelve modules under `forbid(unsafe_code)`:

- `game` is Classic truth. `model` and `simulation` hold integer fixed-tick state
  and take no dependency on winit, wgpu, storage, the editor, advisory, clocks or
  presentation. `view` is the read side.
- `crates/nyon-workshop-core` is the separate pure authority/history/archive crate
  behind Galaxy Workshop (`model`, `simulation`, `command`, `history`, `archive`,
  `pack`, `ids`), plus `living`, a second rule set deliberately partitioned from
  all of those (next section). It must never depend back on `nyon`; a facade test
  pins that.
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

## Living Galaxy V2

`crates/nyon-workshop-core/src/living/` (`mod`, `ids`, `wire`) is the Living
Galaxy V2 authority island, landed 2026-09-06 in `1283e55` and `4f3d28e`. As of
2026-09-08 nothing under `src/` consumes it, and the crate root re-exports the V1
names but not the V2 ones, so callers path through `living::`. Its binding
contract is `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md`; its
evidence is `crates/nyon-workshop-core/tests/living_wire.rs` read against
`tests/fixtures/living-v2/vectors.json`. Two rules that suite enforces
mechanically, and one it cannot:

- **A V1 identity and a V2 identity never appear in the same source file.** The
  guard scans every file the crate compiles for the WorkshopV1 vocabulary
  (`RevisionId`, `EntityId`, `BranchId`, `CatalogHash`, `StateDigest`,
  `WorkshopTick`, `BatchLocalId`, `WORKSHOP_RULES_VERSION`, `WORKSHOP_TICK_HZ`)
  alongside any `Living`/`LIVING_` name. There is no conversion in either
  direction by design, even where serialized widths match. Being a source scan it
  cannot catch a conversion whose V1 operand type is never spelled, or one
  written downstream; the test's own doc comment states that boundary.
- **Adding a module to this crate is a two-place edit.** `CRATE_SOURCES` in
  `living_wire.rs` `include_str!`s every file, and a companion test resolves
  every `mod` declaration against that list and asserts the count is exactly 10.
  A new `pub mod` fails the suite until both are updated.
- **`vectors.json` is not a golden file, and no test can tell you that.** Every
  expected digest was derived independently from section 10 of the rules spec,
  never from this crate's output. The suite only compares against the file, so
  regenerating it from a changed implementation passes while destroying the
  evidence. A digest that moves means the spec moved or the code is wrong.

## Build topology

Features: `default = ["native-backends", "webgpu-backend"]`. `webgl-backend` is
never part of a default build. `crate-type = ["rlib", "cdylib"]`, so the same lib
is both the native library and the wasm module.

A browser release is a pair of artifacts, not one: `build-web-webgpu.sh` and
`build-web-webgl.sh` each disable default features, build `--lib` into their own
target directory, and emit `dist/webgpu` and `dist/webgl`. `web/loader.js` picks
between them before graphics initialization, and `?backend=webgl2` forces the
WebGL2 artifact; `web/` is served beside the artifact rather than compiled into
it. Everything under `assets/` (shaders, the UI atlas, `workshop/core-pack-v1.json`)
does reach the wasm, through `include_str!` and `include_bytes!`, which is why it
counts as a build input for freshness.

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

`crates/nyon-workshop-core` has its own `tests/`: `archive`, `creator`,
`history`, `pack`, `simulation`, `two_system_forge`, `living_wire`, with
`common/` and `fixtures/living-v2/`. The suite list in the CI workflow comment
predates `living_wire`; do not read it as complete.

## Docs

`docs/superpowers/specs/2026-09-02-nyon-v2-design.md` is the accepted Workshop
behavior; `docs/superpowers/plans/2026-09-02-nyon-v2.md` is the master plan, with
`reviews/` recording acceptance. Plans state targets, not proof: read manifests,
source and tests for what is actually implemented.

The 2026-09-04 Living Galaxy set is a second document family beside it:
`plans/2026-09-04-nyon-living-galaxy-program.md` is its master, with foundation,
authority, civilizations and experience children, and
`specs/2026-09-04-nyon-living-galaxy-rules.md` is the contract `living` is
measured against. `reviews/` mixes accepted records with unapplied proposals, so
read a review's `Status:` line before treating it as authority.

`docs/PLAYER-MANUAL.md` and `README.md` describe shipped controls, storage slots
and current limitations. `README.md` is stale in two measured places: its
Verification block omits the mandatory `--workspace`, and its "Build and run in a
browser" section says the browser path has no WebGL fallback, while
`web/loader.js` falls back to the `dist/webgl` artifact automatically when the
WebGPU preflight fails or WebGPU initialization throws. Take gate and web facts
from `AGENTS.md`.

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
