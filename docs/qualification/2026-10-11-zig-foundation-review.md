# Expanded Zig rewrite foundation review

Reviewed the current working-tree foundation slice against the approved Zig-only direction, including `zig/.zigversion`, `zig/src/surface.zig`, `zig/src/play_role.zig`, their `root.zig` and `main.zig` wiring, `zig/tools/check.sh`, `zig/tools/check-surface-cli.py`, `zig/README.md`, and `docs/qualification/2026-10-11-zig-rewrite.md`. Also inspected the repository CI definition for the Rust pipeline boundary. This review does not treat these pure modules as a completed game.

## Verdict

- **Spec compliance: Partial, appropriate for this foundation slice.** The exact `0.18.0-dev.146+35accc06e` pin is enforced by `.zigversion`, `build.zig`, and `check.sh`. The cube topology has distinct face IDs, reversible adjacency, validated coordinates, and `u64` IDs. The role admission function rejects creator impersonation, a ruler using another civilization, and autonomous submissions. The local required Zig gate no longer calls the historical Rust differential oracle. Final repository CI still requires Rust (finding below).
- **Code quality: Acceptable for continued implementation.** No correctness defect was found in the reviewed topology or role policy for valid typed inputs. These modules are not yet connected to new gameplay authority, and the qualification record says so. There are no `unwrap`, cloning, or lock usages in these new foundations.

## Findings

### Important — Required repository CI still depends on Rust

- **File:line:** `.github/workflows/ci.yml:33` (also the Cargo commands at lines 41, 47, and 50, with analogous hosted/other-platform/wasm jobs).
- **Description:** The local `zig/tools/check.sh` gate is Zig-only, but the repository's required CI jobs still install Rust and run Cargo. The approved *final* no-Rust verification pipeline is therefore not implemented. This is a migration gap, not a defect in the scoped local Zig script.
- **Suggestion:** When the replacement authority and platform gates are ready, update required CI jobs to install the pinned Zig compiler and run the appropriate Zig/native/browser checks; remove Rust from the final required pipeline. Keep historical Rust comparison optional.
- **Status:** Open; final-pipeline work remains outside the current foundation slice.

### Minor — Independent seam fixtures cover one source face

- **File:line:** `zig/tools/check-surface-cli.py:14` (second witness at line 18).
- **Description:** Both independent CLI orientation witnesses start on positive Z. The Zig unit test checks reciprocal neighbors on all faces at small sides, and this review's focused max-side corner probe also passed, but reciprocal checks alone do not specify the intended orientation of every seam.
- **Suggestion:** Add independently derived expected IDs for representative seams from the other faces when the topology contract is frozen for map/content consumers.
- **Status:** Open coverage improvement; no observed topology failure.

## Evidence and limits

- The supplied gate log `/tmp/nyon-zig-20261011-final-gate.log` reports 479/479 Debug tests passed, including `play_role` and both `surface` tests, then the ReleaseSafe build and 57 Classic store CLI checks plus two seam vectors/four invalid CLI inputs. The reported overall gate exit was 0; this review did not rerun the full gate.
- The ReleaseSafe section in that log prints `failed command: .../test` without a test failure summary. A focused `zig build test -Doptimize=ReleaseSafe --summary all` rerun exited **0** and explicitly reported `3/3 steps succeeded; 479/479 tests passed` and `run test 479 pass`. It also printed the same `failed command` diagnostic. Thus the phrase is an unexplained Zig runner diagnostic, not evidence that the suite failed; the successful build summary and exit code support the qualification record's ReleaseSafe pass claim. The focused probe log is `/tmp/nyon-foundation-release-probe.log`.
- A read-only CLI probe at side 65535 checked all 24 face corners, 96 neighbor/reverse pairs, and valid target ID ranges; it exited **0**. The maximum valid coordinate is 65534, so the `cell.x + 1` and `cell.y + 1` seam tests do not overflow `u16`. The existing source tests exhaust sides 1, 2, 3, and 8; they are not exhaustive over all 65535 sides.
- The role policy is a pure admission helper. Its public struct can be constructed directly, but `admit` rechecks missing ruler civilization before allowing ruler provenance; autonomous provenance always fails. It does not establish enforcement at a future command dispatcher until it is wired there.
- No live graphical host, browser, persistence migration, city/combat gameplay, accessibility, performance, or cross-platform behavior was verified by this review. Existing user data was not touched. No source, index, branch, dependency, or worktree changes were made for this review.
