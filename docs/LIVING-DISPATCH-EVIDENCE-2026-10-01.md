# Living dispatch integration evidence — 2026-10-01

Status: **Current automated checkpoint; Partial program completion**.

The staged boundary job composes owned dispatch on non-intent boundaries under
its existing shared 1–1024 visited-record budget. A private completed-phase marker
prevents dispatch from running again during transitional sealing. Every 50th
boundary preserves intent-before-dispatch order through the existing explicitly
synchronous completion path. Resident transport, full-boundary budgeting,
validation/sealing, complex creator operations, history and retirement remain
incomplete. This checkpoint does not establish the 5 ms authority target.

The new regression failed before the change (exit 101), then passed after it.
The focused staged suite passed 20 tests, including budgets 1/64/1024, full
state/receipt parity, cancellation, NotReady and stale queue-tail rejection.
The full workspace passed **1,353 tests across 101 suites**, zero failed/ignored.
The earlier 1,351-test checkpoint remains historical in the review document.

All commands below exited 0 against the same code checkpoint:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-targets`
- `cargo build --release`
- Separate fuzz workspace formatting and `cargo clippy --locked --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all-targets -- -D warnings`
- `./tools/build-web.sh` (both WebGPU and WebGL2 artifacts)
- Root and Workshop-core wasm target checks; strict root wasm Clippy
- `./tools/check-workshop.sh`
- Vertex-layout validation, original V2 vector verification and all 17 additive vector modes
- `python3 tools/cross-build-fingerprint.py --skip-tests`

The cross-build command checks paired artifacts and host contract fingerprints;
it does not compare canonical bytes extracted from two live browser executions.
Fuzz lint is compile/lint evidence, not fuzz execution. Native build is not native
runtime acceptance. GPU advisory parity, accessibility journeys, quotas,
corruption/reclamation, performance and endurance remain unverified here.

Source/test/asset/tool identity: 506 files; manifest SHA256
`a6021d1b41a27d2de6d9fe776b1fff3afaed72ff74e7877d48dfe1d219bdf3f9`.
Docs are excluded. Existing unrelated dirty work was retained; no commit made.

## Artifact identity

| Artifact | Bytes | SHA256 |
| --- | ---: | --- |
| `target/release/nyon` | 16,074,544 | `4692feb30869e6cb6e8da4c7b208eb49e7b06cd063bfefa3695eb585bbfb5bca` |
| `dist/webgpu/nyon_bg.wasm` | 10,999,130 | `eb215633a114f06cab8f8b973215fc7480e9b7fe4dc4394430936dde142322ce` |
| `dist/webgpu/nyon.js` | 117,809 | `15b0ff8b7ab3feaea4b714feb6d874af03316d19065b7f0c77e0c8b62e004f64` |
| `dist/webgl/nyon_bg.wasm` | 12,919,039 | `447537a2436aa3764d0143cc59c6502b5c6b2c9d350c1b9b95e7fa1e0437c293` |
| `dist/webgl/nyon.js` | 115,744 | `180cb16e47eb584edc5ecbd402c35cbf15b26731e5028b36afd354742d517173` |

## Browser journey

On this macOS host in the Codex in-app browser, the rebuilt page reported
`WEBGPU initialized successfully`. Product menu → Living Galaxy → dismiss guide
opened paused tick 0. Step advanced to paused tick 1. Save reported
`Galaxy and Continue selection saved`. Reload reported
`Opened exact saved Living galaxy and notebook` at paused tick 1, with the same
visible inventory (energy 0, ore 100, alloy 120) and six fleets.

Navigation to `?backend=webgl2` reported `WEBGL2 LOW initialized successfully`
and reopened that saved tick 1 with the same visible inventory and fleet count.
This is an ordinary real browser save/reopen journey. It is not proof of exact
archive/receipt byte parity, next-1,000-tick reproduction, screen-reader output
or all failure paths. A later screenshot capture failed because the tab became
unavailable; its replacement timed out. No screenshot artifact is claimed.

The native release executable was launched, but computer-use could not bind its
bare executable as an app; no native UI acceptance is claimed for that attempt.

## Logs

Terminal logs were captured in `/tmp/nyon-dispatch-gate-*.log`; refreshed lint,
fuzz, vector and focused regression logs use `/tmp/nyon-dispatch-*` or
`/tmp/nyon-implementation-*`. These paths are session diagnostics, not permanent
reproduction inputs. The commands and artifact identities above are the durable
checkpoint. Proposed expansion specs do not establish implemented mechanics.
