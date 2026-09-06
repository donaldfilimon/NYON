# Repository Instructions

## Sources Of Truth

- This repository is executing `docs/superpowers/plans/2026-09-02-nyon-v2.md`; the accepted behavior is in `docs/superpowers/specs/2026-09-02-nyon-v2-design.md`. The Intergalactic Warfare documents are historical RulesV1 references only.
- The plan describes the target tree, not necessarily the current tree. Trust `Cargo.toml`, `cargo metadata`, source, and tests for what exists now.
- Current task state and evidence boundaries are recorded in `.superpowers/sdd/nyon-v2/progress.md` and the current Galaxy Workshop child plans. Do not rerun relocation, reconstruction, identity restoration, legacy-data copying, or any superseded mega-platform sequence.

## Toolchain And Gates

- Use the repository-pinned `nightly-2026-09-01` toolchain from `rust-toolchain.toml`; do not replace it with moving `nightly` or stable.
- Native startup uses target-scoped Tokio. `main` hosts the multi-thread runtime while winit retains the process main thread; `pollster` is development-only for adapter-backed tests.
- Full CPU gate: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, then `cargo test --workspace --all-targets`.
- `--workspace` is mandatory, not stylistic. `Cargo.toml` sets `default-members = ["."]`, so a bare `cargo test --all-targets` silently runs **294** tests instead of **340** and never executes the `crates/nyon-workshop-core` integration binaries `archive`, `creator`, `history`, `pack`, `simulation`, and `two_system_forge` — the deterministic-authority suites. A bare `cargo clippy --all-targets` does lint that crate's library, because it is a workspace member built as a path dependency, but it does **not** lint its test targets. Both holes were negative-checked on 2026-09-04 by planting a `clippy::len_zero` violation in the crate's library and again in `tests/archive.rs`. The child plans under `docs/superpowers/plans/` already specify the `--workspace` form; this line agrees with them.
- `crates/nyon-workshop-core/fuzz` declares its own `[workspace]`, so **no root gate reaches it**: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features`, and `cargo test --workspace --all-targets` all skip it entirely. Gate it separately with `cargo fmt --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all --check` and `cargo clippy --locked --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all-targets -- -D warnings`. Negative-checked on 2026-09-04 by planting a `clippy::len_zero` violation in `fuzz_targets/pack_decode.rs`: the root gate returned **exit 0** and the fuzz invocation returned **exit 101**; the probe file was restored byte-for-byte and its mtime with it. `--locked` is deliberate — that crate's `Cargo.lock` had already drifted out of date against `sha2`'s `asm` feature and was silently rewritten by the first `cargo check` run against it. This gate is compile-and-lint only; it does not run the fuzzer and is not fuzzing evidence.
- Web compile/bundle gate on a clean machine: `./tools/build-web.sh`, then `cargo check --target wasm32-unknown-unknown --lib`. The script alone installs the pinned target and project-local `wasm-bindgen-cli`; generated `dist/` and `target/tools/` content stays ignored.
- Also run `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`. The host gate never compiles `cfg(target_arch = "wasm32")` code, so browser-only modules such as `src/ui/platform_web.rs` are otherwise unlinted; this caught a real `clippy::let_unit_value` on 2026-09-04.
- `./tools/check-workshop.sh` now fails when either artifact is older than any input compiled or embedded into it (`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `src`, `crates`, `assets`, and the two backend build scripts; test, example, and bench targets are pruned because both artifacts are `--lib`, and `crates/nyon-workshop-core/fuzz` is pruned because it declares its own `[workspace]` and is therefore not part of this build at all). Before 2026-09-04 it verified only that the two artifacts existed and differed, so it returned green against a `dist/` that predated a half-applied source edit. It greps with `grep`, not `rg`, so it runs on a stock hosted image with no extra install step; the flags used (`-F -q`, `-r -n -E -i`) are POSIX, and the three recursive-match cases were re-checked against `/usr/bin/grep` (BSD grep 2.6.0-FreeBSD), which is what a `#!/usr/bin/env bash` script resolves to on this machine even though the interactive shell's `grep` is ugrep. A green result is still artifact evidence only and never establishes browser startup, rendering, fallback, accessibility, or performance.
- Focus integration suites with `cargo test --test campaign`, `cargo test --test shaders`, `cargo test --test scenario`, `cargo test --test scenario_editor`, or `cargo test --test advisory`. Focus module unit tests with `cargo test --lib <module-path-or-test-name>`.
- `cargo test --test advisory_gpu` needs a live WebGPU compute adapter. It prints `SKIP:` and passes when no adapter or compute support exists; that is not GPU-parity evidence.
- Run `cargo build --release` only when the current manifest has a binary target. A successful library build does not prove window, surface, browser, or gameplay startup.

## Authority Boundaries

- `game::model` and `game::simulation` are deterministic campaign truth. Keep them independent of winit, wgpu, editor, storage, advisory, wall-clock time, floating-point state, and presentation values.
- `Simulation` is the sole gameplay authority. Editor drafts and advisory/GPU output must never construct or enqueue `GameCommand`, affect AI/outcomes, or enter the canonical state digest.
- Scenario edits stay in possibly-invalid `ScenarioDraft`; obtain `ScenarioV1` only through validation. Apply/restart must construct temporary validated scenario and simulation state before swapping; load/save/apply failures preserve the active campaign.
- Advisory CPU scores are always the reference. GPU results are presentation-only and publish only after metadata freshness and parity checks; failures disable GPU advisory for that device epoch and retain CPU output.
- On native resume, create the window, wgpu instance, and surface on the winit thread; move only prepared adapter/device acquisition to Tokio. Return results through the standard-library mailbox plus generation-only `AppEvent`, then configure the surface and create renderer resources on the winit thread. Wasm remains a separate `spawn_local` same-thread path.
- On wasm resume, use a fresh `Rc<RefCell<Option<Result<GpuContext, GpuError>>>>` mailbox per generation and send only `AppEvent::GpuInitFinished { generation }`; replacing the mailbox invalidates detached stale futures without requiring `GpuContext: Send`.

## Frozen Contracts

- Preserve `#![forbid(unsafe_code)]`, seven dense worlds, default seed `0x4947_5731_2026_0902`, RulesV1 base fleet speed `23`, generator draw order, fixed-tick phase order, and default state digest `0x67D9_6E98_3D6C_9330`. Do not update goldens to conceal a behavior change; rule or generator changes require an explicit versioned design change.
- Preserve the 36-byte `Vertex` ABI. `shape` uses its low 8 bits for primitive kind and upper 24 bits for the quantized ring-width ratio; host batching, `Vertex::LAYOUT`, tests, and `assets/shaders/primitives.wgsl` must change together.
- Validate shipped WGSL through `engine::shader::validate_wgsl` before pipeline creation. Keep advisory host buffer sizes and `assets/shaders/advisory.wgsl` packing synchronized.
- Layout, hit testing, and renderer globals use logical pixels; only surface configuration uses physical pixels. Do not feed Retina/device-pixel dimensions into logical geometry.
- Scenario JSON is untrusted and capped at 65,536 UTF-8 bytes before parsing. Its seed is exactly 16 uppercase hex digits and every `u64` is a canonical decimal string, not a JSON number.

## Evidence Boundaries

- Keep headless tests, native compile, live native GPU startup, adapter-backed advisory parity, wasm compile/bundle, browser runtime, provider CI, and manual visual acceptance as separate claims.
- Do not claim browser support, native gameplay startup, cross-platform runtime behavior, accessibility semantics, or GPU acceleration from planned files, compilation, headless tests, or an adapter skip.
