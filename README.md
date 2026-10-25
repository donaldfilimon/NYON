# NYON

**Create, explore, and simulate worlds.**

NYON is a Rust game and creative-simulation project built with winit and wgpu.
Its product direction is **creative-first**, with optional Survival and
procedural Geometry Nodes in the broader roadmap. Those goals are not claims
that every planned system already ships.

The current Rust repository contains two distinct experiences:

| Experience | Purpose |
| --- | --- |
| **Galaxy Workshop** | An offline creator/economy sandbox with no victory condition, built beside the preserved game rules. |
| **Classic Sector** | A deterministic single-player strategy game: the Union competes for five of seven worlds. |

Classic's integer fixed-tick simulation owns gameplay truth. Rendering, the
detached scenario editor, and the optional neural advisory cannot issue
simulation commands or decide outcomes.

[Player manual](docs/PLAYER-MANUAL.md) · [Current program](#current-program) ·
[Native setup](#run-natively) · [Browser build](#build-and-run-in-a-browser) ·
[Controls](#controls) · [Verification](#verification)

## Playing NYON

Start with the [player manual](docs/PLAYER-MANUAL.md): a first fleet, a first
solar-energy result, ore/alloy production, controls, saves, and current limits.
No MCP server or AI agent is required to play.

Open **Player guide (F1)** from the main menu or Workshop, or **HELP / H / F1**
in Classic. The guide holds simulation time while open. Classic's first tutorial
marks a Union world with **START HERE**; **SKIP** removes guidance and resumes the
same match. Native gamepad bindings and ray tracing are not implemented.

## Project lineage and scope

This repository, [`donaldfilimon/NYON`](https://github.com/donaldfilimon/NYON),
is the retained Rust workspace. The separate
[`nyon-game`](https://github.com/donaldfilimon/nyon-game) repository preserves the
earlier Zig/raylib sandbox and geometry-node lineage; its commands and save
formats must not be treated as interchangeable with this game's.

NYON is a creative-world project in The Donald Company's broader ecosystem,
not a Quesar model or WDBX database component. LYLX's separate ground-up AI
program does not supersede NYON's existing Rust simulation, storage contracts,
or historical compatibility requirements.

## Current Program

The accepted **NYON Galaxy Workshop V1** behavior is in the
[Workshop design](docs/superpowers/specs/2026-09-02-nyon-v2-design.md).
The [master plan](docs/superpowers/plans/2026-09-02-nyon-v2.md) and its core,
client, and qualification plans organize delivery. Plans define targets, not
proof of completed gameplay or platform acceptance.

Current repository documentation separates these evidence layers:

- **Implemented and locally verified:** exact reviewed RulesV1 reconstruction
  and the breaking NYON package, crate, binary, browser, product, and primary
  storage identity.
- **Implemented:** pure Workshop authority, built-in validated content,
  creator commands, 10 Hz Workshop simulation, branching history/undo/redo,
  archives with compare-and-swap, memory/native/web store adapters, and
  creator/library/inspector tools. WebGPU and WebGL2 have separate artifacts.
- **In progress / qualification work:** recorded live galaxy creation,
  industry/logistics, immutable branch history, crash-safe native/browser
  saves, semantic controls, and the full desktop/browser acceptance matrix.

Do not repeat the completed relocation or baseline reconstruction. The earlier
Intergalactic Warfare design remains a RulesV1 reference, not the current
execution plan. Existing legacy scenario/preferences remain read-only fallback
input; do not introduce copy-on-read migration or rewrite historical saves.

## Run natively

The current [`rust-toolchain.toml`](rust-toolchain.toml) pins **stable Rust
1.99.1**; the workspace uses Edition 2024. The older nightly compiler wording
is superseded by [AGENTS.md](AGENTS.md) and
[the rewrite decision](REWRITE-DECISION-2026-10-19.md).

```bash
cargo run --release
```

For isolated qualification, use an absolute application-data root:

```bash
NYON_PROFILE_DIR=/absolute/test/profile cargo run --release
```

This routes scenario, preferences, Workshop, Living, Library selection, and
Living audio reads/writes into that root with no legacy fallback. Empty or
relative values fail startup. When unset, platform defaults and the documented
read-only legacy fallback remain. File-picker transfers still use explicitly
selected files.

The shared native source targets macOS, Windows, and Linux. Existing live
startup evidence documented here is macOS Metal; compilation and CI definitions
alone are not live acceptance on other systems.

## Build and run in a browser

```bash
./tools/build-web.sh
python3 -m http.server 8000
```

Open `http://127.0.0.1:8000/web/`. The script idempotently installs the pinned
`wasm32-unknown-unknown` standard library and project-local
`wasm-bindgen-cli 0.2.129`. Generated output is ignored under `dist/`.

A browser release is a **pair of artifacts**: `dist/webgpu` and `dist/webgl`.
`web/loader.js` chooses before graphics initialization and falls back to WebGL2
both when WebGPU preflight fails and when initialization throws after preflight.
`?backend=webgl2` forces the fallback artifact.

A successful wasm compile or bundle does not establish browser rendering,
input, storage, or accessibility. Never replace the two backend builds with a
single default-feature wasm build and call it equivalent.

## Controls

| Action | Classic controls |
| --- | --- |
| Select / launch | Click a world; right-click a target from the selected source. |
| Touch launch | Tap a Union source, tap a destination to preview, then tap LAUNCH. Dragging never launches. |
| Confirm tray | Space or on-screen LAUNCH executes the current valid preview; the tray shows typed rejections. |
| Tune fields | 1/2/3 select atmosphere/hydrosphere/topology; Q/E decrease/increase. |
| Time | P pauses/resumes; `[`/`]` select 0x, 1x, 2x, or 4x. Classic retains canonical 60 Hz ticks and discards residual wall time on speed changes. |
| Camera | Drag empty space or use the middle button to orbit; wheel/pinch zooms; C resets. |
| Help / settings | H/F1 or HELP opens the guide; S or SETTINGS opens settings. |
| Scenario | F4 or SCENARIO opens the detached editor; R restarts the validated scenario; Escape clears selection. |

Workshop uses its own documented controls and 10 Hz authority; do not conflate
its timing with Classic's 60 Hz simulation.

The detached editor supports pointer/touch, scrolling, Tab/Shift-Tab, arrows,
Page Up/Down, Enter/Space, Escape, text/IME input, Backspace, and Delete.
REVERT, FACTORY DEFAULTS, REGENERATE FROM SEED, LOAD, SAVE, CANCEL, and
APPLY AND RESTART operate on a draft and require confirmation where state would
be discarded or replaced. The canvas is keyboard-operable but does not provide
screen-reader semantics.

Guidance pauses without accumulating catch-up time. SKIP resumes the same match;
Settings > RESET GUIDANCE restarts the tutorial. Settings include 85%, 100%,
115%, and 130% UI scale, reduced motion, high contrast, and Auto/Low/High
quality. Interactive command-deck controls retain a 44-by-44 logical-pixel
minimum target and visible keyboard focus.

## Scenario persistence

Scenario JSON is versioned, bounded to 65,536 UTF-8 bytes, rejects unknown
fields, uses exactly 16 uppercase hexadecimal seed digits, and encodes every
`u64` as a canonical decimal string. Loading does not apply automatically.
Failed loading, saving, validation, or application preserves the active campaign.

| Platform | Primary scenario slot |
| --- | --- |
| macOS | `~/Library/Application Support/NYON/scenario-v1.json` |
| Windows | `%APPDATA%\NYON\scenario-v1.json` |
| Linux | `$XDG_DATA_HOME/nyon/scenario-v1.json`, otherwise `~/.local/share/nyon/scenario-v1.json` |
| Browser | Origin-scoped `localStorage` key `nyon.scenario.v1` |

Only an absent NYON slot permits the corresponding legacy Intergalactic Warfare
path or `intergalactic-warfare.scenario.v1` key as read-only fallback. A present
NYON slot wins even if invalid. Saving writes only the NYON slot and never
changes legacy storage. Browser storage can be denied, is not confidential,
and must surface recoverable errors.

## Preference persistence

UI scale, motion, contrast, quality, and onboarding use a separate versioned
record limited to 16 KiB: native `preferences-v1.json` beside the scenario slot,
or browser key `nyon.preferences.v1`. Invalid, oversized, denied, or quota-failed
records fall back to defaults with one recoverable message and cannot apply or
overwrite scenario data.

Preference loading follows the same read-only legacy fallback rule; saving
writes only NYON storage. Presentation preferences never enter campaign truth,
scenario JSON, command order, or the canonical simulation digest. Reduced
motion preserves static gameplay cues; high contrast adds non-color patterns.

## Neural advisory

The advisory is a fixed, transparent **12-to-4-to-1 scoring model**, not a
trained service. CPU scores are authoritative for presentation. GPU results
publish as WEBGPU only after freshness and parity checks; unsupported paths or
mismatches keep CPU output and show CPU FALLBACK. The advisory cannot create or
enqueue gameplay commands. No acceleration or performance claim follows.

## Verification

The commands below describe the qualification gates. **AGENTS.md records an
operator pause on tests and CI for the compiler upgrade; listing these commands
does not resume that work or claim they have passed.** Follow the current
operator decision before executing gates.

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo build --release
./tools/build-web.sh
cargo check --target wasm32-unknown-unknown --lib
```

`--workspace` is mandatory: `default-members = ["."]` otherwise excludes
Workshop-core integration suites while a root-only test can still appear green.
The separate `crates/nyon-workshop-core/fuzz` workspace is not reached by these
commands; [AGENTS.md](AGENTS.md) lists its independent gates and the complete
wasm checks.

`cargo test --test advisory_gpu -- --nocapture` is adapter-backed. `SKIP:` is
not GPU parity evidence. Headless tests, native builds, wasm compilation,
bundle generation, provider CI, live startup, browser storage, and visual
acceptance are separate results.

The CI workflow defines native and wasm jobs, including self-hosted macOS arm64
jobs for trusted events; see [runner guidance](docs/SelfHostedRunner.md).
Configuration is not proof a runner executed successfully.

## Contributing

Read [AGENTS.md](AGENTS.md) and the accepted Workshop design before editing.
Preserve RulesV1 oracles, Workshop authority boundaries, save compatibility,
accessibility limits, and the canceled Zig-rewrite decision. Report the exact
checks run and their platform; do not replace expected digests with values
produced by an unverified implementation.
