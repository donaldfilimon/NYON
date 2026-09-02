# NYON

NYON is a deterministic, single-player seven-world strategy game written in Rust with winit and wgpu. The campaign simulation uses integer fixed-tick state. Rendering, the detached scenario editor, and the optional WebGPU neural advisory do not control simulation commands, AI, or outcomes.

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

Open `http://127.0.0.1:8000/web/` in a browser with WebGPU enabled. The browser path requests `BROWSER_WEBGPU` explicitly and has no WebGL fallback. A successful target compile or bundle does not by itself prove browser startup, rendering, input, or storage access.

## Controls

- Click selects a world; right-click launches from the selected world to the target.
- Touch uses three deliberate steps: tap a Union source, tap a destination to preview, then tap LAUNCH. Dragging never launches.
- Space or the on-screen LAUNCH control launches the current valid command-tray preview. The tray shows source, destination, proposed strength, and a typed rejection when the command is unavailable.
- 1, 2, and 3 select atmosphere, hydrosphere, and topology; Q decreases and E increases the selected field.
- P toggles pause and resume; `[` and `]` select 0x, 1x, 2x, or 4x game speed. Every simulation update remains one canonical 60 Hz step, and speed changes discard residual wall time.
- Drag empty space or use the middle mouse button to orbit. Wheel or two-finger pinch zooms, and C resets the camera.
- S or SETTINGS opens local settings. H toggles help, R restarts the active validated scenario, and Escape clears selection.
- F4 or the on-screen SCENARIO control opens the detached editor.

The editor supports pointer/touch, wheel scrolling, Tab and Shift-Tab focus, arrows, Page Up/Down, Enter/Space activation, Escape, text input, IME commits, Backspace, and Delete. REVERT, FACTORY DEFAULTS, REGENERATE FROM SEED, LOAD, SAVE, CANCEL, and APPLY AND RESTART operate on a detached draft and require confirmation where data would be discarded or replaced. The canvas is keyboard-operable but does not provide screen-reader semantics.

First-run guidance pauses without accumulating catch-up time and walks through selection, launch preview and confirmation, field tuning, advisory status, and the detached editor. It can be skipped, restarted from Help, or reset in Settings. Settings provides 85%, 100%, 115%, and 130% UI scale, reduced motion, high contrast, and Auto/Low/High graphics quality. Interactive command-deck controls retain a minimum 44-by-44 logical-pixel target and visible keyboard focus.

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
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --release
./tools/build-web.sh
cargo check --target wasm32-unknown-unknown --lib
```

`cargo test --test advisory_gpu -- --nocapture` is adapter-backed. A printed `SKIP:` is a passing test but is not GPU parity evidence. Headless tests, native builds, wasm compilation, bundle generation, provider CI, live native startup, live browser behavior, and manual visual acceptance are separate evidence layers.

The CI workflow defines native macOS/Windows/Linux gates and a wasm bundle job. Its presence does not establish that a provider has run it successfully.
