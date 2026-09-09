# NYON Galaxy Workshop V1 — qualification evidence record

Date: 2026-09-08
Plan: `docs/superpowers/plans/2026-09-02-nyon-galaxy-workshop-qualification.md`
Preparation and naming decisions: `docs/qualification/2026-09-08-task-1-preparation.md`

## Status conferred by this record

**ARTIFACT-QUALIFIED. Nothing stronger, and the gap is by design rather than by omission.**

This record discharges **Task 1 only**. It establishes source-gate and artifact
evidence for one candidate commit on one host. It is **not** runtime evidence, and
it confers no runtime, accessibility, recovery, offline, performance or
cross-platform status.

Stated plainly because the plan requires the ceiling be named rather than
discovered: **nine of the fourteen live-matrix rows need Windows 11 or Ubuntu
hosts that do not exist on this machine**, and the plan forbids substituting
compilation for live evidence. Runtime-qualified and release-qualified are
therefore **unreachable from this session by design**. Task 6 Step 4 explicitly
permits committing an artifact-qualified checkpoint with unavailable rows labelled
pending, which is what this is.

---

## Step 1 — Candidate provenance

| Field | Value |
|---|---|
| Commit | `60db100a53eb5b813a740bef47abd9c5eec57607` (`60db100`) |
| Branch | `main` |
| Working tree | **clean** — `git status --short --branch` reported no modified, staged or untracked paths |
| Whitespace | `git diff --check` exit **0** |
| Toolchain pin | `nightly-2026-09-01` (`rust-toolchain.toml`, components `clippy`, `rustfmt`, profile `minimal`) |
| rustc | `1.100.0-nightly (0dfb098f3 2026-08-31)`, commit-hash `0dfb098f3aeecbe38c2566ca090193280e7349e7` |
| Host | `aarch64-apple-darwin` |
| LLVM | `23.1.0` |

The plan asks that unrelated untracked paths be listed separately and excluded from
the release diff. **There were none**: the tree was clean at freeze time, which is
why this run waited for a quiet tree rather than proceeding earlier in the session.

## Step 2 — The five source gates

Every exit code was read **from the command itself**, written to its own log file,
never through a pipe or a trailing `echo`. That distinction is not pedantry here: a
backgrounded gate earlier this same session reported exit 0 while `clippy` had
exited 101, because the wrapper's status was read instead of the command's.

| Gate | Exit |
|---|---:|
| `cargo fmt --all --check` | **0** |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **0** |
| `cargo test --workspace --all-targets` | **0** |
| `cargo check -p nyon-workshop-core --target wasm32-unknown-unknown` | **0** |
| `git diff --check` | **0** |

**Workspace totals: 566 passed, 0 failed, 43 binaries.**

`--workspace` is load-bearing and not stylistic: `default-members = ["."]` scopes a
bare invocation to the root package and silently skips every `nyon-workshop-core`
suite while printing green.

## Step 3 — Named subsets, each run individually

Counts are from **isolated invocations**, one suite per `cargo test` call. An
earlier attempt to derive them by parsing the combined workspace log produced
misaligned pairings — it credited `workshop_library` with 23 tests when its own run
reports 7 — and was discarded rather than published. Per-suite counts from a
combined log are not trustworthy without care that this record does not claim.

The plan's Step 3 names a `source-contract` suite that does not exist. Per the
preparation document it expands to **three** source-*text* contracts, marked ※ below.
`renderer_contracts` is deliberately excluded from that slot: it is a *structural*
contract (`offset_of`/`size_of`, real frame building), a different kind of evidence.

| Suite | Exit | Tests |
|---|---:|---:|
| `rules_v1_facade` | 0 | 3 |
| `campaign` | 0 | 8 |
| `scenario` | 0 | 20 |
| `shaders` | 0 | 9 |
| `identity` ※ | 0 | 3 |
| `browser_contract` ※ | 0 | 3 |
| `workshop_store_web` ※ | 0 | 9 |
| `workshop_session` | 0 | 14 |
| `workshop_store` | 0 | 23 |
| `workshop_recovery` | 0 | 7 |
| `workshop_client` | 0 | 23 |
| `workshop_library` | 0 | 7 |
| `workshop_presentation` | 0 | 5 |
| `workshop_accessibility` | 0 | 11 |
| `workshop_web` | 0 | 11 |
| core `pack` | 0 | 9 |
| core `creator` | 0 | 11 |
| core `simulation` | 0 | 8 |
| core `two_system_forge` | 0 | 1 |
| core `archive` | 0 | 3 |
| core `history` | 0 | 6 |

### Scope correction — 100 of these tests are not Workshop V1

`cargo test --workspace` now sweeps in the Living Galaxy V2 authority island, which
is a **different program** with its own plans, spec and vector corpus. Measured by
direct invocation:

| Owner | Tests | Suites |
|---|---:|---:|
| Workspace total | 566 | 43 |
| — of which library unit tests (`--lib`) | 112 | — |
| — of which integration tests | 454 | 40 |
| **Living Galaxy V2** (`living_catalog` 24, `living_model` 51, `living_wire` 25) | **100** | 3 |
| Integration tests in the Workshop V1 candidate's scope | 354 | 37 |

**Quoting 566 as a Workshop V1 figure overstates it.** The 354 is itself the whole
product's integration surface — engine, UI, app, advisory and scenario suites
included — not a Workshop-only count.

## Step 4 — Artifacts

Run in the mandated order, with `check-workshop.sh` **after** both web builds. Its
freshness guard compares `dist/` mtimes against build inputs, so running it first
produces a failure that is not a code problem.

| Command | Exit |
|---|---:|
| `cargo build --release` | **0** |
| `./tools/build-web-webgpu.sh` | **0** |
| `./tools/build-web-webgl.sh` | **0** |
| `./tools/check-workshop.sh` | **0** |

A browser release is a **pair** of artifacts, not one. `web/loader.js` selects
between them before graphics initialization and falls back to the WebGL2 artifact
when the WebGPU preflight fails or when WebGPU initialization throws.

| Artifact | SHA-256 | Bytes |
|---|---|---:|
| `dist/webgpu/nyon.js` | `0bec7ddfb1f0ae8d8a7027219d04079fb3597b231cd4302c17ccdcb19f1469ea` | — |
| `dist/webgpu/nyon_bg.wasm` | `31ab2c5491af4873d82548eb567c9a0f15d9315cb1c37af863cf57a62b517ae8` | 5,516,964 |
| `dist/webgl/nyon.js` | `d7917ad93a7b4e27058464f7ae34f5fe0f5498eda6cfc2a61973e76366a9fa94` | — |
| `dist/webgl/nyon_bg.wasm` | `f393527590ecb574de7afbebf8b7859c67396763b90f286bbaa87e47e71d1e8e` | 7,436,699 |

**`dist/` is gitignored**, so these hashes cover untracked build outputs. They
identify the artifacts this run produced; they are not recoverable from the
repository and do not survive a rebuild unless the build is reproducible, which
this record does not claim to have tested.

---

## What this record does NOT establish

Stated rather than implied, because every one of these is a plausible misreading of
a page full of zeros.

- **Native startup.** Nothing here launched the binary.
- **Browser startup, rendering, input or storage.** A successful target compile and
  a generated bundle are not evidence that the page runs. Neither artifact above was
  loaded in any browser.
- **GPU parity.** `advisory_gpu` printing `SKIP:` is a pass on a machine without a
  usable adapter, not GPU coverage.
- **The IndexedDB adapter.** `src/workshop/store/web/wasm.rs` executes **nowhere in
  this repository** — there is no `wasm-bindgen-test` harness. It has compile and
  clippy evidence on the wasm target and nothing more. `workshop_store_web` passing
  is a source-text contract plus a host-side transaction model; it is not browser
  coverage, and its name invites exactly that misreading.
- **Manual visual acceptance, accessibility inspection, fault injection, offline
  behaviour, recorded p95 timing** — all Tasks 2 through 6, none run.
- **Two-System Forge steps 16–18 through the product.** Baseline Finding 2 is still
  open: `RequestExport`, `RequestImport` and `RequestLoad` are declared and
  constructed nowhere in `src/`. Task 3 remains blocked, and the plan forbids closing
  that here — a product gap returns to the owning child plan. The Library route
  design is that return, and its five prerequisites all landed on 2026-09-08, but the
  screen itself is not built.

## Reproducing this record

```sh
git checkout 60db100
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
git diff --check
cargo build --release
./tools/build-web-webgpu.sh
./tools/build-web-webgl.sh
./tools/check-workshop.sh          # after the web builds, never before
shasum -a 256 dist/webgpu/nyon.js dist/webgpu/nyon_bg.wasm \
              dist/webgl/nyon.js  dist/webgl/nyon_bg.wasm
```

Read each exit code from its own command. Do not read one through a pipe, a
trailing `echo`, or a background wrapper's status.
