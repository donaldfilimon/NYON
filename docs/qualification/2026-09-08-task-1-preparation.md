# Workshop V1 qualification, Task 1: preparation and naming decisions

Date: 2026-09-08

**Status: PREPARATION ONLY. THIS IS NOT EVIDENCE.** No qualification gate has been
run for this document and no artifact has been hashed. It resolves the two naming
and scoping questions that
`docs/superpowers/plans/2026-09-02-nyon-galaxy-workshop-qualification.md` leaves to
its executor, so that the real Task 1 run does not have to stop and decide them
mid-flight. The evidence artifact the plan calls for is
`docs/qualification/2026-09-02-galaxy-workshop-evidence.md`, and it **does not exist
yet**. Do not cite this file in place of it.

Qualification status conferred by this document: **none**.

## Why this is separate from the evidence report

Task 1 Step 1 freezes a candidate commit. `main` is moving today — the Library
prerequisite series is landing task by task — so a provenance capture taken now
would be stale before the gates it introduces had finished. The decisions below are
commit-independent: they are about which suites exist and which program owns them,
and neither answer changes when a commit lands.

So this file resolves what can be settled early, and the evidence report is written
in one pass against a quiet tree.

## Decision 1 — the `source-contract` suite the plan names does not exist

Task 1 Step 3 lists sixteen suites to run individually, one of them "source-contract".
There is no `tests/source_contract.rs`. The plan's own stale-items section flags this
and says an executor must pick one and say which.

**The slot expands to three suites, not one, and naming a single suite would misdescribe
the tree.** Three suites in this repository are source-*text* contracts — they read
project files as strings and assert on substrings:

| Suite | Static tests | What it pins |
|---|---:|---|
| `tests/identity.rs` | 3 | The breaking `nyon` identity across `Cargo.toml`, all three build scripts and `web/loader.js` |
| `tests/browser_contract.rs` | 3 | Browser GPU startup shape across `src/app.rs` and `src/engine/gpu.rs` |
| `tests/workshop_store_web.rs` | 8 | 17 required and 6 forbidden substrings across both browser store files |

**`tests/renderer_contracts.rs` is deliberately excluded from this slot** despite the
name being the closest match. It is a structural contract, not a source-text one: it
asserts with `offset_of`/`size_of` and by building real platform frames. It belongs in
the evidence report under its own heading, and folding it into "source-contract" would
blur the one distinction that makes these suites interesting — that a source-text
assertion survives a refactor that changes behavior, and a structural one does not.

**A caveat the evidence report must carry rather than bury.** A source-text contract
reads like a lint and is not one: it passes if the substring is present, whatever the
code around it does. `workshop_store_web.rs` in particular asserts against
`src/workshop/store/web/wasm.rs`, **a file that executes nowhere in this repository** —
there is no `wasm-bindgen-test` and no browser harness. So these eight tests are
evidence that the browser source says the right things, and are not evidence that the
browser store works.

## Decision 2 — the report is scoped to Workshop V1, and 23% of the suite is not

`cargo test --workspace` now sweeps in the Living Galaxy V2 authority island, which is
a **different program** with its own plans, its own spec and its own vector corpus. Its
tests must not be counted toward Workshop V1 qualification.

Static `#[test]` counts by owner, from the tree:

| Owner | Integration tests | Suites |
|---|---:|---:|
| Workshop V1, engine, UI, app | 337 | 35 |
| **Living Galaxy V2** (`living_catalog` 24, `living_model` 50, `living_wire` 25) | **99** | 3 |
| Total integration | 436 | 38 |

**99 of 436 integration tests — 23% — belong to Living Galaxy V2.** A whole-workspace
count quoted as a Workshop V1 figure overstates it by roughly a quarter.

Two further counting traps, both of which have already produced a wrong number in this
program today:

- The workspace total (549 across 42 binaries at `50c50f9`) exceeds the 436 above
  because `--all-targets` also runs `--lib` unit tests inside `src/` and the core crate.
  The 436 is integration tests only. **Neither number is the other.**
- `tools/check-workshop.sh` runs its **own** `cargo test` over three suites. Summing
  `test result:` lines across a full eleven-command gate log therefore double-counts
  those three. Scope any baseline comparison to the workspace command alone.

The counts in this table are **static greps for `#[test]`**, not run results. They are
an inventory, not evidence, and the evidence report must replace them with recorded
counts from an actual run.

## What remains for the real Task 1 run

- [ ] Step 1, provenance, against a quiet tree: `git rev-parse HEAD`, `git status --short --branch`, `git diff --check`, `rustc --version --verbose`, `cargo metadata --no-deps`. Untracked paths listed separately and excluded from the release diff.
- [ ] Step 2, the five source gates, each exit status read **from the command itself, never through a pipe or a trailing `echo`** — both have manufactured false greens in this repository.
- [ ] Step 3, the sixteen subsets run individually with real counts, using Decision 1's expansion and Decision 2's scoping.
- [ ] Step 4, release build, both web backends, `check-workshop.sh` **after** `build-web.sh`, then artifact hashes. `dist/` is gitignored, so Step 4 hashes untracked outputs — say so in the report.
- [ ] Label the result `artifact-qualified` and nothing stronger.

**The ceiling, stated up front so it is not discovered at the end.** Nine of the
fourteen live-matrix rows need Windows 11 or Ubuntu hosts that do not exist on this
machine, and the plan forbids substituting compilation for live evidence.
Runtime-qualified and release-qualified are unreachable here **by design, not by
omission**. Artifact-qualified with nine rows labelled pending is the honest terminal
state, and Task 6 Step 4 explicitly permits committing it.

Never established by any gate above, and to be stated rather than implied: native
startup, browser startup, GPU parity, and manual visual acceptance. `advisory_gpu`
printing `SKIP:` is a pass on a machine without a usable adapter, not GPU coverage.
