# macOS SDK prerequisite probe

Date: 2026-10-11. Status: **Current prerequisite evidence; game host incomplete**.

## Verified results

The isolated [Zig probe](../../zig/tools/probe-macos-sdk.zig) uses the exact
`0.18.0-dev.146+35accc06e` compiler. Both Debug and ReleaseSafe compilation and
execution exited 0. `zig fmt --check tools/probe-macos-sdk.zig` exited 0.

Build from `zig/`:

```sh
zig build-exe tools/probe-macos-sdk.zig -lc -lobjc -framework AppKit -framework Metal -femit-bin=/tmp/nyon-macos-sdk-probe
/tmp/nyon-macos-sdk-probe
```

Repeat with `-O ReleaseSafe` and a separate output filename. The probe verifies
registered NSApplication/NSWindow classes and obtains an actual default Metal
device. The SDK declares that device return retained; one deferred Objective-C
`release` balances ownership. Missing classes/device produce explicit errors.
Independent review found no blocking ABI, lifetime, ownership or error-handling
defect; the reviewer did not independently rerun the reported commands.

Host: Apple M5, 24 GiB memory; macOS 27.2 build 26B5101f;
Xcode 27.2 build 27B5019j. `xcrun` located the
active macOS SDK, Clang and installed Metal compiler. ReleaseSafe `otool -L`
reported AppKit, Metal, libSystem and libobjc system dependencies. No production
dependency was acquired. The probe neither creates a window nor reads profiles.

## Limits

This proves installed framework linkage and basic device availability. It does
not prove window/event ABI, a Metal pipeline or frame, input, accessibility,
shader compilation, device-loss recovery, storage or a playable game. The Metal
compiler was located but not executed. This standalone tool is not yet part of
the required Zig gate; the explicit commands above are its verification seam.
No Windows/Linux/browser evidence, commit, publication or source retirement is
claimed. Coordinated-galaxy implementation remains owned by its active session.

## Scoped repository gate

`cd zig && ./tools/check.sh` exited 0 in this session. Its log recorded 610/610
Debug tests and 610/610 ReleaseSafe tests, release compilation, four planet CLI
stories/default/eight invalid inputs, the freestanding Wasm probe, 57 Classic
store process checks with zero skips, and two seam vectors/four invalid inputs.
The local diagnostic log is `/tmp/nyon-0821-scoped-gate.log`; it is disposable,
not a durable artifact or a final delivery receipt. Other sessions were actively
editing galaxy sources during this run, so this is an observed scoped gate, not
qualification of a frozen integrated game revision. The standalone SDK probe
was built/run separately as recorded above.
