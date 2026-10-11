# Rust retirement: source-bound dependency inventory

Date: 2026-10-11. Status: Current source inventory; replacements unqualified.
Scope: tracked/source build and tooling entrypoints, excluding target caches and generated distributions. This is a replacement checklist, not deletion authorization or proof of a Rust-free full game.

| Current path | Rust dependency / behavior | Required replacement evidence before retirement |
| --- | --- | --- |
| Cargo.toml, src/, crates/nyon-workshop-core/ | Rust native game and Workshop/Living authority; naga30.0.1 and wgpu30.0.1; browser wasm-bindgen0.2.127 | Qualified expanded Zig authority, durable sessions, all native/browser hosts and retained required capabilities |
| .github/workflows/ci.yml | Native hosted/self-hosted Cargo fmt/clippy/test/release; separate fuzz and wasm jobs | Exact-pinned Zig native/wasm/tool gates and real platform acceptance; preserve fork trust/runner policy when separately authorized to alter CI |
| tools/build-web.sh, build-web-webgpu.sh, build-web-webgl.sh | Two Cargo wasm backends; rustup target install and Cargo-installed wasm-bindgen CLI | Project-owned browser JS/API glue, Zig Wasm outputs, distinct WebGPU/WebGL2 paths; live fallback/persistence/offline/device-loss tests |
| tools/check-workshop.sh:168 | Static artifact/freshness checks then Rust web/renderer/shader tests | New artifact freshness/contracts plus actual Zig/web runtime tests; compilation alone cannot replace browser qualification |
| tools/ui-atlas/Cargo.toml; tools/ui-atlas/src/main.rs; tools/build-ui-atlas.rs | Separate Rust workspace using fontdue, resvg, serde/json, sha2; wrapper includes shared generator source | Project-owned Zig atlas generator and approved raster/system API; versioned metrics/pixels/hashes and glyph/accessibility coverage; regenerated assets qualify before replacement |
| tools/validate-vertex-layout/Cargo.toml; src/main.rs | Separate Rust syn/quote/regex parser/validator | Retire only after new Zig renderer/shader ABI checks qualify. Current CI actually invokes tools/validate-vertex-layout.py, which reads Rust/WGSL declarations without running Rust; it still targets the old renderer contract |
| Cargo.toml naga; tests/shaders.rs; assets/shaders/ | Rust-backed shader validation in old test pipeline | Owned Zig shader tooling, independent malformed/valid compiler fixtures and qualified Metal/DX12/Vulkan/WebGPU/WebGL2 pipelines; approved platform spec provides design direction |
| tools/benchmark-workshop.sh | Cargo builds plus rustc metadata; old Workshop scenario driver | New full-game named-hardware capacity/performance harness with actual simulation/history/cleanup work and declared memory/timing limits |
| tools/cross-build-fingerprint.py | Calls Cargo RulesV1 facade and Workshop web suites | Independent new identity/wire/hash fixtures and cross-platform Zig replay/determinism checks; do not recycle old goldens under new identities |
| zig/tools/check-classic-parity.py | Optional historical differential oracle builds Rust example zig_classic_oracle | Keep explicitly optional migration evidence until no longer useful; exclude from final required verification and remove dependency path at qualified retirement |
| crates/nyon-workshop-core/fuzz/ | Independent Rust fuzz workspace and lockfile | New malformed command/archive/history coverage and appropriate fuzz execution; old CI compile/lint is not fuzz-run evidence |
| zig/tools/check.sh | PATH blockers for cargo/rustc/rustup, exact Zig pin, Debug/ReleaseSafe/release/CLI/wasm/Classic/surface checks | Current scoped Rust-blocked gate, already qualified; extend with new implemented systems rather than falsely presenting it as complete-game acceptance |

## Asset and pipeline constraints

The existing atlas generator pins Inter4.1 and Lucide commit assets and rasterizes a1024x1024R8 atlas with64pixel cells and15icon entries. Asset URLs/hashes are source observations, not a fresh external/download verification; this audit fetched no assets. Do not edit generated atlas files manually, acquire production dependencies, or silently change accepted asset versions.

A Cargo manifest's presence proves source dependency, not that every entry is on the required pipeline. CI/shell invocation establishes the distinction above. Likewise the Python vertex validator does not require rustc to parse Rust text, but final tooling must validate the new Zig/graphics ABI rather than depend on obsolete Rust declarations.

The approved platform design is docs/superpowers/specs/2026-10-09-nyon-zig-platform-bindings-design.md. Its implementation-plan approval remains separate. The newer expanded-game roadmap and no-old-data rule govern final delivery; historical Classic fallback/import rules must not be copied into new saves.

## Verification of this inventory

Read the listed manifests, generator/wrapper source, shell/Python entrypoints and workflow. Ran bounded rg searches excluding nested target output after an initial noisy cache-inclusive search. No Cargo/rustc/rustup, atlas generation, download, shader compilation, runtime test, production dependency, CI mutation, profile write, source retirement, commit or deployment occurred. Existing source and cached tools are preserved.

Next action: use this matrix to sequence tooling/host replacements after their reviewed contracts; retired source must be backed by replacement capability and pipeline evidence, not just deletion producing a clean search.
