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
| macOS picker interaction | **unproven, procedure ready** | A real save panel cannot be driven headless. The spike now has both picker modes (see *Manual picker run*); each needs one run on the owner's screen. |
| Event-loop fit | **partly proven by source reading** | With `set_parent`, rfd 0.17.2 calls `beginSheetModalForWindow_completionHandler` (`backend/macos/file_dialog/panel_ffi.rs`), and the completion is delivered on the main run loop. Without a parent it calls `runModal`, which spins its own modal loop. So `pollster::block_on` is only safe on the no-parent path; blocking inside `resumed` on the sheet path would starve the loop the completion needs. The spike's `--sheet` mode polls the future from `about_to_wait` with a no-op waker under `ControlFlow::Poll`, which is the shape `TransferAdapter`'s start/poll/abandon already has (no `Send` bound). The real adapter should wake the loop from the future instead of busy polling. |
| Durable write sequence | **proven on macOS** | The spike's `--durable-write` writes a same-directory temp file (`create_new`), calls `sync_all`, renames over the destination, and syncs the parent directory. A fresh write and a replace both succeeded, leaving no temp file behind. Rust std maps `sync_all` to `fcntl(F_FULLFSYNC)` on Apple platforms (`library/std/src/sys/fs/unix.rs`), so the `DurablySaved` label is honest here. |
| License | **proven for the macOS graph** | Every crate in the macOS graph is permissive (MIT, Apache-2.0, BSD-2/3, Zlib, ISC, Unlicense-or-MIT, Unicode-3.0 combinations). One crate reports no license field, which is the spike crate itself. `cargo deny` and `cargo about` are not installed, so this is a `cargo metadata` reading, not a policy run. |
| Dependency cost (macOS) | **measured** | New crates over winit alone: `rfd`, `objc2` 0.6, `objc2-app-kit` 0.3, `objc2-foundation` 0.3, `objc2-core-foundation` 0.3, `block2` 0.6, `dispatch2` 0.3, `log`, `pollster`. winit 0.30 uses the older `objc2-app-kit` 0.2, so two objc2 generations would ship side by side. |
| Windows | **unproven** | Resolves to `windows-sys` 0.61.2, beside winit's 0.52.0: a duplicate `windows-sys`. Needs a Windows 11 host. |
| Linux, xdg portal, Wayland/X11 | **unproven** | Default features pull `wayland-backend`/`wayland-client`/`wayland-protocols`, `pollster` and `percent-encoding`. No new duplicate versions (the `rustix`/`linux-raw-sys` pair already comes from winit's X11 path). Portal behavior needs an Ubuntu host. |
| Packaging | **unproven** | NYON has no release packaging path in this tree. The only precedent is the 2026-09-04 acceptance review, which ad-hoc signed a temporary `.app`. See *macOS packaging checklist*. |

## Manual picker run (owner, macOS, about five minutes)

The scratch crate lives in `/private/tmp`, which a reboot clears. The *Spike source*
appendix below recreates it.

1. `cd /private/tmp/nyon-rfd-spike && cargo build > build.log 2>&1; echo "EXIT: $?"`.
   Expect `EXIT: 0`.
2. **Modal path.** Run `target/debug/nyon-rfd-spike` with no arguments. A blank window
   appears, and a free-standing save panel opens with `archive.json` filled in and a
   "NYON archive" format filter.
   - Save into any scratch folder. **Pass:** the terminal prints
     `picked: Some("<folder>/archive.json")` and the process exits by itself.
   - Run it again and press Cancel. **Pass:** it prints `picked: None` and exits.
   - No file is written in either case, because the spike only reports the path.
3. **Sheet path.** Run `target/debug/nyon-rfd-spike --sheet`. The save panel now slides
   down attached to the window.
   - **Pass:** Save and Cancel print the same two lines and exit.
   - **Fail:** a panel that never appears, a beachball, or no output after choosing.
     Any of these means the polling shape is wrong for the real adapter.
4. **Durable write.** Run `target/debug/nyon-rfd-spike --durable-write <folder>/archive.json`
   twice. **Pass:** `durably saved: <path>` both times, and no `.tmp` file is left in
   `<folder>`.
5. **Record the result** in this file's table: the date, the macOS build
   (`sw_vers -buildVersion`), and pass or fail for each of steps 2 and 3. Also run
   `codesign -dv target/debug/nyon-rfd-spike` and note it. A debug build is linker-signed
   ad hoc, so the table must not claim more than that. Do not paste the chosen path if it
   is under your home directory.

## macOS packaging checklist

Nothing below has been exercised. Each item is what the packaging row needs before it
can say **proven**.

- **Unsandboxed, Developer ID build.** An `NSSavePanel` needs no entitlement. Sign with
  hardened runtime (`codesign --options runtime`), notarize, staple, and repeat steps 2
  and 3 from the stapled `.app`. The spike run above does not stand in for this.
- **Sandboxed build** (only if NYON ever ships sandboxed). The app needs
  `com.apple.security.files.user-selected.read-write`. Per Apple's App Sandbox
  documentation, the panel then runs out of process through Powerbox. Two consequences:
  - Access to the chosen URL lasts for the session only, unless NYON stores a
    security-scoped bookmark. §8 does not ask for that, so none is planned.
  - The durable-write sequence creates a temporary file next to the chosen destination.
    The user-selected grant covers the chosen file, not its directory, so a sandboxed
    build may be refused that sibling temp file. This must be measured before a
    sandboxed build can claim `durably saved`. If it is refused, the honest label is
    `written`.
- **Quarantine.** A downloaded `.app` carries `com.apple.quarantine`. Gatekeeper must
  accept the notarized build on a clean account before the picker is exercised from it.
- **Mixed objc2 generations.** A release build links both objc2 0.5-era crates (from
  winit) and 0.6 crates (from rfd). Confirm the release binary launches and shows the
  sheet, and record its size delta against a build without rfd.
- **Licenses.** Run `cargo deny check licenses` or `cargo about` on the release graph
  once either tool is installed. The table's reading comes from `cargo metadata` and is
  not a policy run.

## Unproven on other hosts (no macOS run can close these)

- **Windows 11.** Steps 2 to 4 on the `IFileSaveDialog` path. Confirm the duplicate
  `windows-sys` builds and check the binary size cost. `sync_all` there is
  `FlushFileBuffers`, and the parent directory cannot be synced, so the label
  question must be decided from Windows semantics.
- **Linux.** Steps 2 to 4 under GNOME/Wayland through `xdg-desktop-portal`, then on X11
  and on a desktop without a portal. That last run decides the `gtk3` fallback question.
  A portal returns a document-portal path in sandboxed installs, which needs the same
  sibling-temp-file check as the macOS sandbox item.
- **Packaging** on both: the release artifact format NYON will ship, which does not exist
  yet.

## Open questions for the decision

1. Accept two objc2 generations on macOS, or wait for a winit release on objc2 0.6.
2. Accept the `windows-sys` duplicate on Windows.
3. Keep the default `xdg-portal` + `wayland` features, or add `gtk3` as a fallback for
   desktops without a portal.

Reproduce: `cargo build` in a crate with the two dependencies above, then
`target/debug/nyon-rfd-spike --durable-write <dir>/archive.json`; the picker modes are under *Manual picker run*.

## Spike source

This crate is throwaway and stays outside the NYON tree. Recreate it with `cargo new nyon-rfd-spike` in `/private/tmp`, then replace these two files.

`Cargo.toml`:

```toml
[package]
name = "nyon-rfd-spike"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
rfd = "=0.17.2"
winit = "=0.30.12"
pollster = "0.4"

[workspace]
```

`src/main.rs`:

```rust
//! Spike: rfd 0.17.2 beside NYON's pinned winit 0.30.12.
//! `--durable-write <path>` exercises the §8 durable-replace sequence without UI.
//! With no arguments it opens a winit window and an async save dialog on it.
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

fn durable_replace(dest: &Path, bytes: &[u8]) -> std::io::Result<&'static str> {
    let dir = dest.parent().unwrap_or(Path::new("."));
    let tmp = dir.join(format!(
        ".{}.tmp",
        dest.file_name().unwrap().to_string_lossy()
    ));
    let mut f = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?; // on macOS, std maps this to fcntl(F_FULLFSYNC)
    drop(f);
    std::fs::rename(&tmp, dest)?;
    File::open(dir)?.sync_all()?;
    Ok("durably saved")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--durable-write") {
        let dest = Path::new(&args[2]);
        match durable_replace(dest, b"{\"spike\":true}\n") {
            Ok(label) => println!("{label}: {}", dest.display()),
            Err(e) => {
                println!("failed: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    // Two picker modes. Default: no parent, so rfd calls `runModal`, which spins
    // its own modal loop and is safe to block on. `--sheet`: parent set, so rfd
    // calls `beginSheetModalForWindow`, whose completion runs on the main run
    // loop; blocking inside `resumed` would starve it, so the future is polled
    // from `about_to_wait` instead, the shape NYON's adapter needs.
    let sheet = args.get(1).map(String::as_str) == Some("--sheet");
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll, Waker};
    use winit::application::ApplicationHandler;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::window::{Window, WindowId};
    type Pick = Pin<Box<dyn Future<Output = Option<rfd::FileHandle>>>>;
    struct App {
        sheet: bool,
        window: Option<Window>,
        pending: Option<Pick>,
    }
    fn dialog() -> rfd::AsyncFileDialog {
        rfd::AsyncFileDialog::new()
            .add_filter("NYON archive", &["json"])
            .set_file_name("archive.json")
    }
    impl ApplicationHandler for App {
        fn resumed(&mut self, el: &ActiveEventLoop) {
            let window = el.create_window(Window::default_attributes()).unwrap();
            if self.sheet {
                self.pending = Some(Box::pin(dialog().set_parent(&window).save_file()));
                el.set_control_flow(ControlFlow::Poll);
            } else {
                let picked = pollster::block_on(dialog().save_file());
                println!("picked: {:?}", picked.map(|h| h.path().to_path_buf()));
                el.exit();
            }
            self.window = Some(window);
        }
        fn about_to_wait(&mut self, el: &ActiveEventLoop) {
            if let Some(fut) = self.pending.as_mut() {
                let mut cx = Context::from_waker(Waker::noop());
                if let Poll::Ready(picked) = fut.as_mut().poll(&mut cx) {
                    self.pending = None;
                    println!("picked: {:?}", picked.map(|h| h.path().to_path_buf()));
                    el.exit();
                }
            }
        }
        fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, e: WindowEvent) {
            if matches!(e, WindowEvent::CloseRequested) {
                el.exit();
            }
        }
    }
    let el = EventLoop::new().unwrap();
    el.run_app(&mut App { sheet, window: None, pending: None }).unwrap();
}
```
