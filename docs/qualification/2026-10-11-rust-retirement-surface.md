# Rust retirement surface — 2026-10-11

Status: **Partial.** Donald explicitly authorized complete Rust removal at
08:23 EDT. Source retirement follows equivalent Zig replacement evidence;
no remaining Rust file was removed by this audit.

Current filesystem inventory through `rg --files` found **242 Rust source files**:
108 under `src`, 56 under `crates`, 69 under `tests`, five under `examples`, and
four under `tools`. It also found ten Cargo/toolchain manifests and lockfiles.
The inventory respects repository ignore rules; it does not measure ignored
build caches or establish that every indirect Rust invocation has been found.
Exact enumerated bytes and SHA256 values are retained in the scoped execution
ledger's `rust-retirement-inventory.json`.

| Surface | Current evidence | Required retirement evidence |
| --- | --- | --- |
| Pure gameplay and authority | Existing Zig migration gate reached both 628-test suites successfully; coordinated-galaxy work is still progressing independently. | Complete approved gameplay, global atomicity, history/replay, robustness and exact versioned contracts; preserve independent wire witnesses. |
| Native/browser hosts and retained UI | No native or browser journey qualified by the seeded metadata work. | AppKit/Metal, Win32/DX12, X11/Vulkan, WebGPU/WebGL2 and input/audio/accessibility acceptance under new Zig runtime. |
| Durable sessions | New contracts and separately owned implementation are in progress; no quit/reopen Continue evidence claimed here. | New namespaces, durable conflicts/crash/cancellation/import/export/branch/Continue qualification without reading old profiles. |
| Atlas generator | Existing Cargo workspace uses fontdue/resvg; new owned-Zig V2 design passed planning review; implementation remains outstanding. | Qualified Zig generator, metrics/raster fixtures, atomic publication/check mode, cross-host output and actual visual acceptance. |
| Shader and vertex tooling | Rust tooling remains in the inventory. | Owned Zig compilation/validation, independent vertex packing/shader fixtures and actual backend pipelines. |
| Examples, tests, build/package graph | Rust references remain. Existing Zig gate's shared source-stability guard failed after the completed suites. | Owner-stable full Zig gate, replacement independent fixtures, required packaging/browser/native journeys and audit with Rust unavailable. |

The independently reviewed seeded metadata slice passes 48 aggregate and 10 CLI
tests in each of Debug and ReleaseSafe. It qualifies metadata generation,
owned decode and copied policy/config queries; it does not establish production
replacement for the Rust host, city/combat game, storage or tooling graph.

Before retirement, rehash the inventory against the live dirty checkout, preserve
all unique/uncommitted work, identify each exact replacement and remove obsolete
source/configuration only within its transferred ownership. Re-run the complete
build/generation/verification/runtime/package graph with Rust unavailable.
Rust retirement is authorized; acceptance evidence remains required. Do not
substitute file deletion, compilation or metadata tests for game delivery.
