# Native dialog spike, macOS half (addendum §8)

Date: 2026-09-17 02:3x EDT. Host: macOS 27.2 (26B5086k), Apple silicon, rustc
1.100.0-nightly (2026-08-31). Scratch crate outside this tree
(`/private/tmp/nyon-rfd-spike`), so NYON's lockfile and target were not touched.

**Verdict: `rfd` 0.17.2 is a viable candidate on macOS. The dependency decision stays
open** until the Windows, Linux and packaging rows below are proven on those hosts;
§8 forbids pinning before then.

## Candidate

`rfd` 0.17.2 from crates.io, license MIT, repository
<https://github.com/PolyMeilex/rfd>. Default features are `xdg-portal` and
`wayland`; `gtk3` is optional. It exposes `AsyncFileDialog` with `set_parent`
(raw-window-handle), filters, and a suggested file name.

## Findings by §8 criterion

| Criterion | Status | Evidence |
|---|---|---|
| macOS build beside NYON's pins | **proven** | `rfd = "=0.17.2"` with `winit = "=0.30.12"` builds (debug binary 4.1 MB). Both use `raw-window-handle` 0.6.2, so there is one version, not two. |
| macOS picker interaction | **unproven** | A real save panel cannot be driven headless and was not opened on the owner's screen. Needs one manual run of the spike binary with no arguments. |
| Event-loop fit | **partly proven by design reading** | The dialog future is created on the winit event-loop thread with `set_parent(&window)`. The spike's `pollster::block_on` inside `resumed` is only a smoke path: on macOS a modal panel needs the running event loop, so the real adapter must poll the future from the loop. That matches `TransferAdapter`'s start/poll/abandon and its lack of a `Send` bound. |
| Durable write sequence | **proven on macOS** | The spike's `--durable-write` writes a same-directory temp file (`create_new`), calls `sync_all`, renames over the destination, and syncs the parent directory. A fresh write and a replace both succeeded, leaving no temp file behind. Rust std maps `sync_all` to `fcntl(F_FULLFSYNC)` on Apple platforms (`library/std/src/sys/fs/unix.rs`), so the `DurablySaved` label is honest here. |
| License | **proven for the macOS graph** | Every crate in the macOS graph is permissive (MIT, Apache-2.0, BSD-2/3, Zlib, ISC, Unlicense-or-MIT, Unicode-3.0 combinations). One crate reports no license field, which is the spike crate itself. `cargo deny` and `cargo about` are not installed, so this is a `cargo metadata` reading, not a policy run. |
| Dependency cost (macOS) | **measured** | New crates over winit alone: `rfd`, `objc2` 0.6, `objc2-app-kit` 0.3, `objc2-foundation` 0.3, `objc2-core-foundation` 0.3, `block2` 0.6, `dispatch2` 0.3, `log`, `pollster`. winit 0.30 uses the older `objc2-app-kit` 0.2, so two objc2 generations would ship side by side. |
| Windows | **unproven** | Resolves to `windows-sys` 0.61.2, beside winit's 0.52.0: a duplicate `windows-sys`. Needs a Windows 11 host. |
| Linux, xdg portal, Wayland/X11 | **unproven** | Default features pull `wayland-backend`/`wayland-client`/`wayland-protocols`, `pollster` and `percent-encoding`. No new duplicate versions (the `rustix`/`linux-raw-sys` pair already comes from winit's X11 path). Portal behavior needs an Ubuntu host. |
| Packaging | **unproven** | Not exercised; needs the release packaging path on each OS. |

## Open questions for the decision

1. Accept two objc2 generations on macOS, or wait for a winit release on objc2 0.6.
2. Accept the `windows-sys` duplicate on Windows.
3. Keep the default `xdg-portal` + `wayland` features, or add `gtk3` as a fallback for
   desktops without a portal.

Reproduce: `cargo build` in a crate with the two dependencies above, then
`target/debug/nyon-rfd-spike --durable-write <dir>/archive.json`.
