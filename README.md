# NYON

NYON is a deterministic, single-player seven-world strategy game written in Rust with winit and wgpu. The campaign simulation uses integer fixed-tick state. Rendering, the detached scenario editor, and the optional WebGPU neural advisory do not control simulation commands, AI, or outcomes.

## Playing NYON

Start with the [player manual](docs/PLAYER-MANUAL.md). **Classic Sector** is the Union's battle for five of seven worlds; **Galaxy Workshop** is a separate offline creator/economy sandbox with no victory condition. The manual covers a first fleet, a first solar-energy result, ore/alloy production, ordinary controls, saves and current limitations. No MCP server or agent is required.

Open **Player guide (F1)** from the main menu or Workshop, or **HELP / H / F1** in Classic. It holds simulation time while you read. Classic's first tutorial step marks a Union world with **START HERE**; **SKIP** removes that tutorial and resumes the same match. Native gamepad bindings and ray tracing are not implemented.

## Current Program

NYON is being expanded into **NYON Galaxy Workshop V1**, an offline creative galaxy sandbox built beside the frozen RulesV1 game. The accepted target behavior is in the [NYON Galaxy Workshop V1 design](docs/superpowers/specs/2026-09-02-nyon-v2-design.md), and implementation is organized by the [NYON V2 master plan](docs/superpowers/plans/2026-09-02-nyon-v2.md) plus its core, client, and qualification child plans.

Current status is intentionally separated by evidence layer:

- Implemented and locally verified: reconstruction of the exact reviewed RulesV1 source tree and the breaking NYON package, crate, binary, browser, product, and primary-storage identity.
- In progress: the pure Workshop authority and built-in validated content pack.
- Planned: recorded live galaxy creation, deterministic industry and logistics, immutable branch history, crash-safe native and browser saves, Workshop tools and semantic controls, distinct WebGPU and WebGL2 artifacts, and the full live desktop/browser qualification matrix.

The repository relocation and exact-baseline reconstruction are completed historical operations and must not be repeated. The prior Intergalactic Warfare design and plan remain RulesV1 references, not current execution authority. Legacy Intergalactic Warfare scenario and preference data remains read-only fallback input: a present NYON slot always wins, and Workshop work must not introduce copy-on-read migration, import markers, import reports, or legacy-data rewriting.

## Run natively

The repository pins `nightly-2026-09-01` in `rust-toolchain.toml`.

```bash
cargo run --release
```

The shared native source is intended for macOS, Windows, and Linux. Current live startup evidence is macOS Metal; compilation or CI configuration alone is not runtime acceptance on another platform.

## Build and run in a browser

The repository script is the supported web build path. It idempotently installs the pinned `wasm32-unknown-unknown` standard library and a project-local `wasm-bindgen-cli 0.2.127`, then writes ignored output under `dist/`.

```bash
./tools/build-web.sh
python3 -m http.server 8000
```

Open `http://127.0.0.1:8000/web/` in a browser. A browser release is a **pair** of artifacts, not one: `build-web.sh` runs both backend builds and emits `dist/webgpu` and `dist/webgl`.

`web/loader.js` chooses between them before graphics initialization and **does fall back to WebGL2**, by two separate paths: when the WebGPU preflight fails, and when WebGPU initialization itself throws after a successful preflight. `?backend=webgl2` forces the WebGL2 artifact directly. A successful target compile or bundle does not by itself prove browser startup, rendering, input, or storage access.

## Controls

- Click selects a world; right-click launches from the selected world to the target.
- Touch uses three deliberate steps: tap a Union source, tap a destination to preview, then tap LAUNCH. Dragging never launches.
- Space or the on-screen LAUNCH control launches the current valid command-tray preview. The tray shows source, destination, proposed strength, and a typed rejection when the command is unavailable.
- 1, 2, and 3 select atmosphere, hydrosphere, and topology; Q decreases and E increases the selected field.
- P toggles pause and resume; `[` and `]` select 0x, 1x, 2x, or 4x game speed. Every simulation update remains one canonical 60 Hz step, and speed changes discard residual wall time.
- Drag empty space or use the middle mouse button to orbit. Wheel or two-finger pinch zooms, and C resets the camera.
- S or SETTINGS opens local settings. HELP / H / F1 opens the player guide, R restarts the active validated scenario, and Escape clears selection.
- F4 or the on-screen SCENARIO control opens the detached editor.

The editor supports pointer/touch, wheel scrolling, Tab and Shift-Tab focus, arrows, Page Up/Down, Enter/Space activation, Escape, text input, IME commits, Backspace, and Delete. REVERT, FACTORY DEFAULTS, REGENERATE FROM SEED, LOAD, SAVE, CANCEL, and APPLY AND RESTART operate on a detached draft and require confirmation where data would be discarded or replaced. The canvas is keyboard-operable but does not provide screen-reader semantics.

First-run guidance pauses without accumulating catch-up time and walks through selection, launch preview and confirmation, field tuning, advisory status, and the detached editor. SKIP resumes the same match; Settings > RESET GUIDANCE restarts the tutorial. Settings provides 85%, 100%, 115%, and 130% UI scale, reduced motion, high contrast, and Auto/Low/High graphics quality. Interactive command-deck controls retain a minimum 44-by-44 logical-pixel target and visible keyboard focus.

## Scenario persistence

Scenario JSON is versioned, limited to 65,536 UTF-8 bytes, rejects unknown fields, uses exactly 16 uppercase hexadecimal seed digits, and encodes every `u64` as a canonical decimal string. Loading does not apply automatically; failed load, save, validation, or apply preserves the active campaign.

- macOS: `~/Library/Application Support/NYON/scenario-v1.json`
- Windows: `%APPDATA%\NYON\scenario-v1.json`
- Linux: `$XDG_DATA_HOME/nyon/scenario-v1.json`, falling back to `~/.local/share/nyon/scenario-v1.json`
- Browser: origin-scoped `localStorage` key `nyon.scenario.v1`

When the NYON scenario slot is absent, loading may consult the corresponding legacy Intergalactic Warfare path or `intergalactic-warfare.scenario.v1` browser key as a read-only fallback. A present NYON slot always wins, including when its payload is invalid. Saving writes only to the NYON slot and never changes legacy storage.

Browser storage can be denied or unavailable and is not confidential. Store failures are recoverable editor errors.

## Preference persistence

UI scale, motion, contrast, graphics quality, and onboarding completion use a separate versioned record limited to 16 KiB. Malformed, oversized, unsupported-version, denied, or quota-failed preference data falls back to defaults with one recoverable message and cannot read, overwrite, or apply scenario data.

- Native: `preferences-v1.json` beside the platform-specific scenario slot above
- Browser: origin-scoped `localStorage` key `nyon.preferences.v1`

Preference loading follows the same read-only legacy fallback rule using the old `preferences-v1.json` location or `intergalactic-warfare.preferences.v1` browser key. Preference saving writes only to NYON storage.

Reduced motion removes camera easing, parallax, pulsing halos, and nonessential particles while preserving static selection, ownership, field, fleet, and hazard cues. High contrast keeps faction hues and adds persistent non-color patterns. These are presentation preferences only; they never enter scenario JSON, campaign truth, command ordering, or the canonical state digest.

## Neural advisory

The advisory is a fixed, transparent 12-to-4-to-1 scoring model, not a trained service. CPU scores are authoritative for presentation. A compatible GPU may compute the same seven scores, but results publish as WEBGPU only after metadata freshness and parity checks. Any unsupported compute path or mismatch retains CPU output and marks CPU FALLBACK. Advisory output cannot construct or enqueue gameplay commands, and no performance or acceleration claim is made.

## Verification

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo build --release
./tools/build-web.sh
cargo check --target wasm32-unknown-unknown --lib
```

`--workspace` is mandatory and is not a stylistic preference. `Cargo.toml` sets
`default-members = ["."]`, so a bare `cargo test --all-targets` scopes to the root
package, silently skips every `crates/nyon-workshop-core` integration suite, and
still prints a green result. `AGENTS.md` is canonical for the gate; this block
mirrors it. Note also that `crates/nyon-workshop-core/fuzz` is a separate workspace
that no command here reaches.

`cargo test --test advisory_gpu -- --nocapture` is adapter-backed. A printed `SKIP:` is a passing test but is not GPU parity evidence. Headless tests, native builds, wasm compilation, bundle generation, provider CI, live native startup, live browser behavior, and manual visual acceptance are separate evidence layers.

The CI workflow defines native macOS/Windows/Linux gates and a wasm bundle job. Its presence does not establish that a provider has run it successfully.
