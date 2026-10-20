# Rust stable compilation qualification — 2026-10-19

Status: Current compilation evidence; runtime/feature/platform acceptance remains Unverified.

Compiler observed: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, native `aarch64-apple-darwin`. Both native and `wasm32-unknown-unknown` standard libraries were installed. Existing root/core/atlas Rust floors and browser scripts select1.99.1. No dependency edits were necessary after these checks.

## Retained workspace checks

The following were executed during the planning inspection on2026-10-19 and each returned exit0:

```sh
cargo +1.99.1 check --locked --workspace --all-targets --all-features
cargo +1.99.1 check --locked --target wasm32-unknown-unknown --lib --no-default-features --features webgpu-backend
cargo +1.99.1 check --locked --target wasm32-unknown-unknown --lib --no-default-features --features webgl-backend
cargo +1.99.1 check --locked --manifest-path tools/ui-atlas/Cargo.toml --all-targets
cargo +1.99.1 check --locked --manifest-path tools/validate-vertex-layout/Cargo.toml --all-targets
cargo +1.99.1 check --locked --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all-targets
```

Native workspace check:33.04s; WebGPU:47.39s; WebGL:64s (including build-directory wait); atlas:10.47s; vertex validator:8.92s; fuzz compilation:20.50s. These are observed Cargo elapsed times, not performance measurements.

Vertex validator warned about unused `Ident` and `TypePath` imports. This is a successful compilation with warnings, not a strict lint result. Its output directory contains tracked artifacts: the command updated `tools/validate-vertex-layout/target/.rustc_info.json` and created untracked debug output. Existing artifacts remain preserved. Future separate-tool checks use ignored root output directories.

## Policy and limits

No tests, fuzz execution, Clippy, formatting gates, CI, browser wrappers, runtime launches, asset generation or deployment were run. `cargo check --all-targets` typechecks test targets without executing them. It does not establish release linking, asset/WGSL validity, browser bundling, IndexedDB behavior or gameplay parity.

CI still pins `nightly-2026-10-08`; CI changes are deferred. The retained Rust upgrade compiles; the broader feature ports and full-game acceptance remain Partial.

## Recovery changes

Cube topology and player-role admission have initial Rust ports under `crates/nyon-workshop-core/src/recovery/`, with independent source witnesses in `tests/recovery_foundations.rs`. Every new module is included in the existing core source/declaration inventory. Behavioral tests remain unexecuted by instruction. New game/UI design discovery is in progress at Donald's request; no new UI mechanics are approved by a questionnaire answer alone.

Post-port native compilation: `cargo +1.99.1 check --locked --workspace --all-targets --all-features` returned exit0 after the topology/admission ports and core source inventory updates. New regression targets were typechecked, not executed. Independent task review is pending.
