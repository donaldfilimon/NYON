# AGENTS.md

Canonical repository guidance. Accepted Workshop behavior is in
`docs/superpowers/specs/2026-09-02-nyon-v2-design.md`; the master plan is
`docs/superpowers/plans/2026-09-02-nyon-v2.md`. Plans describe targets, not proof
of implementation; use manifests/source/tests for current behavior. Intergalactic
Warfare docs are historical RulesV1 references. Do not repeat relocation,
reconstruction, identity restoration or superseded mega-platform work.

## Gates

Use **nightly-2026-09-01**, edition 2024 (`rust-toolchain.toml`, `Cargo.toml`).
The native CI gate in `.github/workflows/ci.yml` is:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo build --release
```

- `--workspace` is mandatory: `default-members = ["."]` omits Workshop-core
  integration suites and their Clippy targets from bare root invocations.
- Focus root integration: `cargo test --test campaign` (also `scenario`,
  `scenario_editor`, `shaders`, `advisory`); unit: `cargo test --lib <filter>`.
  Focus core: `cargo test -p nyon-workshop-core --test two_system_forge`.
- `cargo test --test advisory_gpu -- --nocapture` passes with `SKIP:` when an
  adapter or compute support is absent. That is not GPU-parity evidence.
- Fuzz is a **separate workspace**, excluded even from root fmt. Its CI checks
  compile/lint only, not fuzz execution; keep its independent lockfile enforced:

```sh
cargo fmt --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all --check
cargo clippy --locked --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all-targets -- -D warnings
```

The wasm CI path is separate; native all-features cannot lint wasm-only code:

```sh
./tools/build-web.sh
cargo check --target wasm32-unknown-unknown --lib
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings
./tools/check-workshop.sh
```

- `build-web.sh` is an alias for **both** backend build scripts. Each disables
  default features and builds `--lib` into its own target directory, producing
  separate `dist/webgpu` and `dist/webgl` artifacts. Do not substitute a default
  feature wasm build. Scripts may install the wasm target and local
  `wasm-bindgen-cli 0.2.127`; `dist/` and `target/tools/` remain ignored.
- `check-workshop.sh` checks artifact shape, difference and input freshness,
  static backend contracts, optional Node loader syntax, and three Rust suites.
  It needs both built artifacts; tests/examples/benches and the separate fuzz
  workspace are excluded from freshness inputs. It is not a docs-only check.
- Compilation, bundle/static checks, fuzz compilation, live native/browser
  behavior, accessibility and manual visual acceptance are separate evidence.

## Authority And Storage

- `game::model`/`game::simulation` own Classic truth: integer fixed ticks, no
  winit/wgpu/editor/storage/advisory/clock/presentation dependencies. Only
  `Simulation` controls gameplay; editor and advisory never enqueue `GameCommand`.
- `nyon-workshop-core` is a separate pure authority/history/archive crate. Never
  depend back on `nyon`, wgpu, winit, Tokio or browser APIs; the facade test pins
  this boundary. Keep SHA-2 assembly target-scoped away from MSVC.
- Scenario edits stay in possibly-invalid `ScenarioDraft`; validate temporary
  scenario/simulation state before swapping. Failed load/save/apply preserves
  the active campaign. JSON is capped at 65,536 UTF-8 bytes before parsing, with
  16 uppercase hex seed digits and canonical decimal-string `u64` values.
- Legacy scenario/preferences are read-only fallback only when the NYON slot
  is absent. A present or errored NYON slot wins; no copy-on-read migration,
  import markers/reports or legacy rewriting (`scenario/store.rs`).
- Advisory CPU scores are the reference. GPU results are presentation-only;
  publish only after freshness/parity checks, otherwise disable that device
  epoch's GPU advisory and retain CPU output. Neither affects canonical digests.

## GPU And Frozen Contracts

- Native `main` hosts Tokio while winit retains the main thread. Prepare window,
  instance and surface there; only adapter/device acquisition moves to Tokio.
  Return through a mailbox plus generation-only `AppEvent`, then configure the
  surface and renderer on winit. `pollster` is test-only.
- Wasm uses `spawn_local` and a fresh `Rc<RefCell<Option<Result<...>>>>` mailbox
  per generation. Stale completions must not install resources; do not require
  `GpuContext: Send` to share the native initialization path.
- Preserve `forbid(unsafe_code)` and the oracles in `tests/rules_v1_facade.rs`:
  seven dense worlds, seed `0x4947_5731_2026_0902`, base fleet speed 23, digest
  `0x67D9_6E98_3D6C_9330`. Generator draw order and fixed-tick phase order are
  frozen; changes require versioned design, not replacement goldens.
- `Vertex` is 36 bytes: shape low 8 bits are primitive kind, upper 24 ring-width
  ratio. Change batching/layout/tests/`assets/shaders/primitives.wgsl` together;
  validate WGSL before pipelines and keep advisory host/shader packing aligned.
- Layout, hit-testing and renderer globals use logical pixels; only surface
  configuration uses physical pixels. Retina dimensions are not UI geometry.
