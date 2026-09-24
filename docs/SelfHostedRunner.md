# Self-hosted macOS runner

Three CI jobs in `.github/workflows/ci.yml` run on a macOS arm64 runner registered to this repository:

| Job id | Check name |
|--------|------------|
| `native` | `Native (macos-latest)` |
| `fuzz` | `Fuzz target compile and lint` |
| `wasm` | `WASM compile and bundle` |

GitHub-hosted jobs can't start while the account's Actions billing is locked (they fail in about two seconds with no runner assigned), but self-hosted jobs still run. The check names above are unchanged, so branch protection that requires them keeps working.

## Registration

| Field | Value |
|-------|-------|
| Labels | `self-hosted`, `macOS`, `ARM64`, `nyon` |
| Register at | [Settings → Actions → Runners → New self-hosted runner](https://github.com/donaldfilimon/NYON/settings/actions/runners/new?arch=arm64) (choose macOS, ARM64) |

`nyon` is a custom label; add it when `./config.sh` asks for additional labels (or pass `--labels nyon`). `.github/actionlint.yaml` declares it so actionlint accepts it.

A runner is registered to one repository. If the same Mac already runs a runner for another repository (for example `abi` or `gama`), install a second runner in its own directory, such as `~/actions-runner-nyon`, register it here with the `nyon` label, then run `./svc.sh install && ./svc.sh start` from that directory. The two runners share the account's `~/.rustup` and `~/.cargo`, which is fine: the toolchain is pinned.

Until a runner with these labels is online, the three jobs wait in the queue.

## Host requirements

- **Xcode Command Line Tools** (`xcode-select --install`). They supply the C/C++ linker and Apple clang (the fuzz job's `libfuzzer-sys` build compiles libFuzzer with it), `git`, and `/usr/bin/python3`. The Python tools (`validate-vertex-layout.py`, `living-v2-vectors.py`, `cross-build-fingerprint.py`) use only the standard library and run on the Command Line Tools' Python 3.9.
- **Homebrew** on the runner service's `PATH`, owned by the runner account. `actions-rust-lang/setup-rust-toolchain` runs `brew install bash` on every macOS job, because macOS ships bash 3.2. Install it once ahead of time (`brew install bash`) and consider `HOMEBREW_NO_AUTO_UPDATE=1` in the runner's `.env` so that step stays fast. The runner reads `PATH` from its `.path` file, written by `./config.sh`; if Homebrew was added to `PATH` later, update that file and restart the service.
- **rustup**. The toolchain action installs it per user (no `sudo`) if it is missing, then installs the pinned `nightly-2026-09-01` with `rustfmt` and `clippy`. `tools/build-web-*.sh` add the `wasm32-unknown-unknown` target and build `wasm-bindgen-cli 0.2.127` into `target/tools/` inside the checkout.
- **Node.js** is optional. `tools/check-workshop.sh` runs `node --check web/loader.js` only when `node` is on `PATH`.
- **Disk**. Each job cleans the checkout, so `target/` and `dist/` are rebuilt from the Actions cache or from scratch. Keep 20 GB or so free.
- **GPU**. `tests/advisory_gpu.rs` prints `SKIP:` without an adapter. On a Mac with Metal it finds one and runs the GPU-versus-CPU comparison, which hosted runners usually skip.

Do not set `RUSTFLAGS` or `CARGO_TARGET_DIR` in the runner account's environment. The toolchain action sets `RUSTFLAGS=-D warnings` only when it is unset, and the web build scripts expect `target/` inside the checkout.

## Security

This repository is public. The self-hosted jobs run only when `github.repository` is `donaldfilimon/NYON` and the event is a `push` or a pull request whose head branch is in this repository (the condition also admits `workflow_dispatch`, which the workflow does not currently declare). Pull requests from forks never reach the Mac: they run the same steps on GitHub-hosted runners through `native-hosted`, `fuzz-hosted` and `wasm-hosted`, which carry ` (GitHub-hosted, fork PRs)` in their names. Those fork-PR jobs also wait on the billing lock.

The workflow has no `pull_request_target`, `issue_comment` or `workflow_run` triggers, and none may be added that reach the self-hosted jobs. The workflow token is `contents: read`. The self-hosted checkouts use `persist-credentials: false`, so the token is not left in `.git/config` on the host. The self-hosted toolchain steps set `cache-bin: false`, so the runner account's `~/.cargo/bin` is never saved to or restored from the Actions cache.

Where you can, run the runner under a dedicated macOS user rather than your daily account. Keep no production secrets on the host.

## Still GitHub-hosted

- `Native (windows-latest)` and `Native (ubuntu-latest)` (job `native-other-os`): they need Windows and Linux hosts. The Mac can't substitute for them, and dropping them would lose the only Windows (MSVC) and Linux compile and test coverage.
- The three fork-PR jobs listed above: untrusted code must not run on the Mac.

These stay blocked until the billing lock is cleared.
