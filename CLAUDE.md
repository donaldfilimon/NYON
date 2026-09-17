# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`AGENTS.md` is the canonical repository guidance and wins on any conflict. It owns
the gate commands, the authority/storage rules, and the frozen GPU and renderer
contracts. Read it first; this file is a routing map for the parts it does not
carry, and deliberately does not restate it.

## Gates

Run the four native commands, the fuzz pair, and the wasm sequence exactly as
`AGENTS.md` lists them. Two things it emphasizes and this file repeats only as a
pointer: `--workspace` is mandatory because `default-members = ["."]` scopes bare
cargo invocations to the root package and skips `crates/nyon-workshop-core`
entirely, and the fuzz directory is its own workspace that no root command reaches.

Focused runs during development:

```sh
cargo test --lib <filter>                                   # unit tests in src/
cargo test --test workshop_ui_sdf                           # one root integration binary
cargo test --test campaign -- <filter> --nocapture          # one case, with output
cargo test -p nyon-workshop-core --test two_system_forge    # core authority crate
cargo run --release                                         # native app
```

`tools/` holds the scripted paths: `build-web.sh` (runs both backend builds),
`check-workshop.sh` (artifact shape, freshness, static contracts, three Rust
suites), `benchmark-workshop.sh`. `.claude/`, `.remember/` and `.superpowers/` at
the root are session tooling, not project files.

## Layout

`src/lib.rs` declares twelve modules under `forbid(unsafe_code)`:

- `game` is Classic truth. `model` and `simulation` hold integer fixed-tick state
  and take no dependency on winit, wgpu, storage, the editor, advisory, clocks or
  presentation. `view` is the read side.
- `crates/nyon-workshop-core` is the separate pure authority/history/archive crate
  behind Galaxy Workshop (`model`, `simulation`, `command`, `history`, `archive`,
  `pack`, `ids`), plus `living`, a second rule set deliberately partitioned from
  all of those (next section). It must never depend back on `nyon`; a facade test
  pins that.
- `engine` is the wgpu layer: `gpu`, `backend`, `resources`, `shader`, `render`,
  `render_frame`, `scene_renderer`, `ui_renderer`, `primitives`, `quality`,
  `input`, `time`.
- `ui` is the SDF platform UI. `platform.rs` is the entry with children
  `platform/shell.rs` and `platform/workshop.rs`; alongside it sit
  `platform_sdf`, `platform_projection`, `platform_inspector`, and the split
  native/web backends `platform_native.rs` / `platform_web.rs`. Workshop screens
  are `workshop.rs` (the immutable, render-neutral UI model that returns typed
  intents and never mutates authority) with private builders under
  `workshop/{creator_defaults,history,outliner,removal,semantics}.rs`, plus the
  siblings `workshop_inspector`, `workshop_layout`, `workshop_view` and
  `creator`, `guide`, `library`, `accessibility`, `virtual_list`,
  `start_marker`. **Four files are named for the library and they are two
  different layers:** `ui/library.rs` is the Library screen's UI model, intents
  and focus order, covered by `tests/workshop_ui_library.rs`;
  `app/client_runtime/library.rs` is the open algorithm below it (next bullet),
  covered by `tests/workshop_library.rs`. Neither test reaches the other layer.
- `workshop` is the client side of the Workshop. `session.rs` paces it;
  `store.rs` is the facade holding the `WorkshopStore` trait, the request/result
  and error taxonomy, `SlotId`/`SlotName`/`SaveGeneration` and the job table,
  over three adapters: `store/memory.rs` (unconditional), `store/native.rs`
  (`cfg(not(wasm32))`), and `store/web.rs`. See the wasm split under *Traps*.
- `app` is the runtime: `core` (and `core/session.rs`), `client_runtime` (and
  `client_runtime/library.rs`), `input_router`, `durable_exit`, `onboarding`,
  `settings`. **`client_runtime/library.rs` holds `WorkshopLibraryClient`, the
  exact-catalog open algorithm extracted from `poll_continue_bootstrap` in
  `75cf146`.** The split is deliberate and load-bearing: the client owns the
  five phases and returns events, while `client_runtime.rs` keeps *screen
  policy* — the client never enters recovery, never picks a screen and never
  installs a session. Startup Continue runs on it, so there is one
  implementation rather than two. `app/modal_lifecycle_tests.rs` is an in-tree
  module declared `#[cfg(all(test, not(target_arch = "wasm32")))]` at
  `app.rs:1732`, not a root integration binary: only `cargo test --lib` reaches
  it and `--test <name>` will not find it. It is **also a fixture provider** —
  `app/input_router.rs` tests import `capacity_app` and `open` from it — so
  deleting or renaming those helpers breaks tests in another module.
- `platform` splits the entry points into `native.rs` and `web.rs`; `main.rs` is
  native only, and `src/lib.rs` is also the cdylib the wasm build exports.
- `scenario` (`codec`, `store`) and `preferences` (`store`) own versioned,
  size-capped persistence with the read-only legacy fallback rule.
- `presentation` (`camera`, `interaction`, `picking`, `scene`, `ui`, `workshop`),
  `editor` (`layout`, `render`), `advisory` (`gpu`), and `classic` are the
  remaining view, editor and advisory surfaces. None of them enqueue commands.

### Modal projection

Three answers used to be derived three ways, and `WorkshopUiModel` can hold the
creator form and the removal confirmation at once (`src/app.rs` clears neither
when opening the other), so they could disagree about which dialog was live.
Since `6ef114b` every modal projection routes through `ui/platform_projection.rs`
and reads the same `modal_dialog(tree)`: `modal_presentation` for what is drawn,
`modal_action_ids` for the focus trap's Tab order, `modal_identity` for the
change-detection key. Add a projection and it must take the dialog from there.
The pin is `every_modal_projection_describes_the_same_dialog`, and
`modal_action_ids_covers_a_dialog_type_no_model_field_names` pins the payoff:
because the action list comes from the semantic tree rather than named model
fields, a new dialog type gets a focus trap without touching
`platform/workshop.rs`. No user path into the both-open state has been
demonstrated — treat it as a latent inconsistency now impossible by
construction, not a fixed user-visible bug.

Each modal module owns its own action identifiers and focus order rather than
`src/app.rs` holding either: `creator_defaults.rs` exports `CREATOR_*_ACTION`
with `creator_modal_order`, `removal.rs` exports `REMOVAL_*_ACTION` with
`removal_modal_order`. A new control belongs in that module, not in the app.

Interactive controls are laid out to a 44 logical-pixel minimum on both axes,
and `tests/workshop_ui_layout.rs` asserts `>= 44.0` on control rects across the
Workshop screens — most recently the modal paging controls raised in `21ab3c3`
(`modal_paging_controls_meet_the_44px_minimum_when_the_modal_paginates`).
**The two UI surfaces spell it differently and only one has a symbol:**
`presentation/ui.rs` defines `MIN_CONTROL_EXTENT = 44.0` and clamps against it,
while the SDF platform UI under `ui/` writes bare `44.0` literals
(`platform/workshop.rs`, `platform/shell.rs`, `platform_projection.rs`,
`workshop_view.rs`). Grep the literal, not the constant, when working in `ui/`,
and do not assume changing `MIN_CONTROL_EXTENT` moves the Workshop screens.

## Living Galaxy V2

`crates/nyon-workshop-core/src/living/` (`mod`, `ids`, `wire`, `catalog`,
`model`, `genesis`, `command`, `receipt`, `simulation`: **nine files as of
2026-09-17**, `simulation` being authority Task 5's first slice, `04c678f`) is
the Living Galaxy V2 authority island, landed 2026-09-06 in `1283e55` and
`4f3d28e`. The authority plan's four tasks all landed on 2026-09-08: the catalog
(`71e5b6f`), the frozen state schema (`50c50f9`), genesis (`f569437`), and
commands/receipts/events with the entity-kind and phase registries (`2f6561a`,
`bbc3da6`, `ab1f081`). Its test suites are `living_wire`, `living_catalog`,
`living_model`, `living_genesis`, `living_command`, `living_receipt`,
`living_queue`.
As of 2026-09-17 nothing under `src/` consumes any of it
(`grep -rn living src/` is empty), and the crate root re-exports the V1 names but
not the V2 ones, so callers path through `living::`. Its binding
contract is `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md`; its
evidence is `crates/nyon-workshop-core/tests/living_wire.rs` read against
`tests/fixtures/living-v2/vectors.json`. Two rules that suite enforces
mechanically, and one it cannot:

- **A V1 identity and a V2 identity never appear in the same source file.** The
  guard scans every file the crate compiles for the WorkshopV1 vocabulary
  (`RevisionId`, `EntityId`, `BranchId`, `CatalogHash`, `StateDigest`,
  `WorkshopTick`, `BatchLocalId`, `WORKSHOP_RULES_VERSION`, `WORKSHOP_TICK_HZ`)
  alongside any `Living`/`LIVING_` name. There is no conversion in either
  direction by design, even where serialized widths match. Being a source scan it
  cannot catch a conversion whose V1 operand type is never spelled, or one
  written downstream; the test's own doc comment states that boundary.
- **Adding a module to this crate is a two-place edit.** `CRATE_SOURCES` in
  `living_wire.rs` `include_str!`s every file (**17 as of 2026-09-17**),
  and a companion test resolves every `mod` declaration against that list,
  asserting a literal declaration count (**16**). A new `pub mod` fails the suite
  on both counts until the array and the number are updated together, and the
  child label must carry the `living/` prefix or path resolution fails. **Both
  numbers moved four times on 2026-09-08 — measure them, do not quote this
  line.**
- **`vectors.json` is not a golden file, and no test can tell you that.** Every
  expected digest was derived independently from section 10 of the rules spec,
  never from this crate's output. **The tool that does it is now checked in:
  `tools/living-v2-vectors.py`.** Its `verify` mode re-derives every stored digest
  from the spec formulas, and the discipline that makes it evidence is running
  that control on the *unmodified* corpus first — Task 4 reproduced every stored
  digest before changing five of them, so the transcription was validated against
  frozen values rather than trusted. Use it, and never regenerate a vector from
  the implementation. The suite only compares against the file, so
  regenerating it from a changed implementation passes while destroying the
  evidence. A digest that moves means the spec moved or the code is wrong.

## Build topology

Features: `default = ["native-backends", "webgpu-backend"]`. `webgl-backend` is
never part of a default build. `crate-type = ["rlib", "cdylib"]`, so the same lib
is both the native library and the wasm module.

A browser release is a pair of artifacts, not one: `build-web-webgpu.sh` and
`build-web-webgl.sh` each disable default features, build `--lib` into their own
target directory, and emit `dist/webgpu` and `dist/webgl`. `web/loader.js` picks
between them before graphics initialization, and `?backend=webgl2` forces the
WebGL2 artifact; `web/` is served beside the artifact rather than compiled into
it. Everything under `assets/` (shaders, the UI atlas, and the two catalog
packs `workshop/core-pack-v1.json` and `living/core-pack-v2.json`) does reach the
wasm, through `include_str!` and `include_bytes!`, which is why `check-workshop.sh`
lists the whole directory as a freshness input.

`tools/ui-atlas/` is an independent Cargo project with its own lockfile and its
own ignored target dir. It runs `tools/build-ui-atlas.rs` to rasterize Inter plus
the Lucide icon set into the committed `assets/ui/atlas.r8`, `atlas-metrics.json`
and `hashes.json`. `tests/ui_assets.rs` re-hashes those committed files and
validates the metrics, so regenerating the atlas without updating the manifest
fails the root gate.

## Tests

Root `tests/` maps to subsystems: `rules_v1_facade` (the frozen RulesV1 oracle,
seeds and digest in `AGENTS.md`), `campaign`, `scenario`, `scenario_editor`,
`presentation`, `renderer_contracts`, `shaders`, `ui_assets`, `advisory`,
`advisory_gpu` (SKIP-tolerant, not parity evidence), `app`, `identity`,
`preferences`, `usability`, `player_guide`, `browser_contract`, `workshop_web`,
`workshop_store_web`, `workshop_library` (added 2026-09-08 with the library
client), and the `workshop_*` family covering session, store, recovery, client,
presentation, accessibility, bare `workshop_ui` and the five `workshop_ui_*`
UI suites (`sdf`, `layout`, `inspector`, `creator`, `library`). Shared fixtures
are in `tests/common/`.

`crates/nyon-workshop-core` has its own `tests/`: `archive`, `creator`,
`history`, `pack`, `simulation`, `two_system_forge`, and the six Living V2
suites `living_wire`, `living_catalog`, `living_model`, `living_genesis`,
`living_command`, `living_receipt`, with `common/` and `fixtures/living-v2/`.
The suite list in the comment above the core-crate Clippy step in
`.github/workflows/ci.yml` names only the first six and predates every Living
suite; do not read it as complete. `--workspace`
runs them regardless, so the stale comment costs documentation, not coverage.

## Docs

`docs/superpowers/specs/2026-09-02-nyon-v2-design.md` is the accepted Workshop
behavior; `docs/superpowers/plans/2026-09-02-nyon-v2.md` is the master plan, with
`reviews/` recording acceptance. Plans state targets, not proof: read manifests,
source and tests for what is actually implemented.

The 2026-09-04 Living Galaxy set is a second document family beside it:
`plans/2026-09-04-nyon-living-galaxy-program.md` is its master, with foundation,
authority, civilizations and experience children, and
`specs/2026-09-04-nyon-living-galaxy-rules.md` is the contract `living` is
measured against. `reviews/` mixes accepted records with unapplied proposals, so
read a review's `Status:` line before treating it as authority.

`docs/PLAYER-MANUAL.md` and `README.md` describe shipped controls, storage slots
and current limitations. **Both `README.md` staleness items recorded here were
fixed on 2026-09-08**: its Verification block now carries the mandatory
`--workspace` with the reason stated, and its browser section now describes the
real two-artifact story and both WebGL2 fallback paths in `web/loader.js` (failed
preflight, and WebGPU initialization throwing after a successful preflight).
`AGENTS.md` remains canonical for the gate and the README says so; prefer it on
any conflict.

## Traps

- Tests that read project files resolve `env!("CARGO_MANIFEST_DIR")` at compile
  time and Cargo does not fingerprint it, so after the checkout is moved or
  renamed a stale binary reads the old absolute path and fails with
  `No such file or directory`. Run `cargo clean -p nyon` before treating that as
  a source defect.
- `check-workshop.sh` compares `dist/` mtimes against build inputs. Rebuilding
  out of order, or touching a source after a build, produces a freshness failure
  that is not a code problem. Rebuild with `./tools/build-web.sh`, then re-run it.
- `advisory_gpu` printing `SKIP:` is a pass on a machine without a usable adapter.
  Do not report it as GPU coverage.
- **A green native gate is zero evidence for the browser store.**
  `src/workshop/store/web.rs` compiles everywhere and holds the schema constants
  plus `IndexedDbTransactionModel`, a pure host-testable oracle for the
  transaction boundary (mutations become visible only after it commits). The
  real adapter is `store/web/wasm.rs`, ~1,500 lines behind `cfg(wasm32)` that
  `cargo clippy --all-targets --all-features` never sees. Only the wasm gate in
  `AGENTS.md` compiles or lints it, so run that sequence after touching it, and
  do not read a passing `workshop_store_web` suite as coverage of the real
  IndexedDB path. The same asymmetry applies to `store/native.rs` under wasm.
- **A store job has exactly two lanes, and dropping a job ID used to wedge one
  for the store's lifetime.** `WorkshopStore::abandon` (added 2026-09-08 in
  `df2457c`) frees a lane and forgets the job; it is implemented once on the
  shared `JobTable` and delegated by all four adapters plus
  `RuntimeWorkshopStore`. Before it existed, a caller that dropped a job without
  polling starved `CommitSlot` and `PromoteRecoveredSlot`, so the resident
  Workshop could no longer save or discharge a recovery obligation.
  **⚠️ `abandon` abandons the OUTCOME, not the WORK, and the distinction is the
  whole risk.** Memory and the browser model have already executed by the time
  `start` returns; native's thread and a live IndexedDB transaction run to
  completion. Nothing is cancelled and nothing is rolled back. The argument that
  this is safe rather than corrupting is that every head-dependent mutation
  compare-and-swaps, so the next commit, promotion or selection receives
  `StaleGeneration` instead of writing over a generation nobody observed.
  **That argument was reviewed and found SOUND** (`5e066ce`,
  `docs/superpowers/reviews/2026-09-08-workshop-task4-library-client-review.md`):
  the reviewer enumerated the whole request vocabulary and found no
  head-dependent mutation that skips its compare-and-swap in any adapter —
  `memory.rs:145/205/277`; `native.rs:250/308/409`, all under the exclusive
  `store-v1.lock` taken at `native.rs:190` before the manifest read and held
  across the entire read-modify-write; `wasm.rs:400/488/690`, each inside one
  readwrite transaction. **The safety property is therefore a consequence of
  universal CAS, not of anything `abandon` itself does — so if you ever add a
  head-dependent mutation, it must compare-and-swap or this becomes a
  corruption path.** Note what remains unverified by execution rather than by
  reading: browser transaction serialization is inferred from the spec and the
  scopes read, never run.
- **A fast "Finished" from the wasm target may be a cached green.** The wasm
  commands are the only thing that compiles `store/web/wasm.rs` at all, so a
  cached pass is indistinguishable from real coverage. Force the question:
  append a temporary `compile_error!` to that file, confirm
  `cargo check --target wasm32-unknown-unknown --lib` exits non-zero *with that
  message*, then remove it. That is how `df2457c`'s wasm evidence was
  established, and a fast pass on that target should not be believed otherwise.
- **⚠️ A restore that preserves mtime can make Cargo skip the rebuild, and the dangerous
  direction is a FALSE GREEN.** Found 2026-09-08 while mutation-testing. `sed -i.bak`
  *renames* the original file — keeping its old mtime — and writes a new one in its
  place. Moving the `.bak` back therefore leaves the source **older** than the compiled
  artifact, so Cargo's mtime fingerprint decides nothing changed and reuses the stale
  binary. Observed as a false **red**: restored source, `git status` clean, `grep`
  confirming the mutation gone, and the mutant's failures still reported.
  **The symmetric case is the one that matters.** Any restore-style edit — `sed -i.bak`
  + `mv`, `git stash pop`, `cp` from an older copy, `git checkout` of an older blob —
  can leave a *mutation* uncompiled, so a mutation that appears to **survive** may
  simply never have been built. That silently converts "this test does not catch the
  defect" into a wrong conclusion, which is precisely backwards for a technique used to
  prove a test discriminates. Mutation evidence is only as good as the rebuild.
  **Mitigation: `touch` the file after any restore, or verify the rebuild happened**
  (a genuinely-rebuilt run is not instant). Writing the file fresh — as `cp src dst` or
  a script that opens and writes — sets a current mtime and is safe. Same family as the
  `env!("CARGO_MANIFEST_DIR")` trap above: the build system's cache disagreeing with the
  source you are looking at.
- **⚠️ AN EXIT CODE READ FROM ANYTHING BUT THE COMMAND ITSELF IS NOT EVIDENCE. This fired
  FOUR TIMES on 2026-09-08 in this repository alone**, twice on a controller's own runs,
  and it manufactures *green*, which is the direction that ships defects. The gates here
  are slow enough that everyone reaches for a wrapper, so this is a repo-local hazard
  rather than a general one. Four distinct shapes, all observed:
  1. **`cmd | tail`** reports tail's status, not `cmd`'s, and the window hides the rest.
  2. **A trailing `echo`** in a compound command: `cargo test > log 2>&1; echo "EXIT: $?"`
     is correct, but `{ …; } 2>&1` around a group, or any `echo` after the command whose
     own status you then read, gives you the `echo`'s.
  3. **A background wrapper's status.** A backgrounded gate reported **exit 0 while
     `clippy` had exited 101**, and separately while `fmt` had exited 1. Both were caught
     only by reading per-command codes out of the log.
  4. **Worst shape: a truncated log with no exit line at all.** A backgrounded baseline
     reported exit 0 while its log stopped at 559 lines, carried **no** exit line, and
     contained **339 of 638 tests**. Nothing in it says "incomplete" — it reads as a clean
     pass to anyone skimming, and the test count only looks wrong if you already know the
     baseline.
  **The rule: redirect each command to its own log, echo that command's own `$?` into
  that log, and confirm the count against the log's own `test result:` lines.** Never
  conclude green from a wrapper's status, and treat a suspiciously fast or suspiciously
  short log as unproven rather than passing.
- **⚠️ `git checkout <rev> -- <path>` WRITES THE INDEX, so the obvious undo restores the
  wrong thing.** Found 2026-09-09 while re-verifying a mutation. Checking a path out of
  another revision stages it as well as writing the worktree; a later
  `git checkout -- <path>` then restores **from the index**, which now holds the reverted
  content. The file looks restored, `git status` can look clean, and **the suite goes
  green while testing the wrong tree** — the same false-green direction as the stale
  artifact and the wrapper exit code. `git restore --staged --worktree <path>` is the
  correct undo *for that specific case*. Relevant here because reverting one file to an
  older revision is exactly how you prove a test could not have caught a defect before a
  commit.
  **⚠️ CORRECTED 2026-09-09 02:5x — the advice above is incomplete and the incomplete
  form caused a real contamination.** `git restore --staged --worktree <path>` restores
  to **HEAD**, so running it to undo a mutation while your own fixes to that file are
  **uncommitted silently deletes them**. Observed: four source fixes vanished, and the
  next mutation run killed **five** tests instead of two — three of the kills were the
  deleted fixes rather than the mutation. It was caught only because the failing test
  *names* did not match what the mutation should have broken.
  **The general rule, of which the `checkout`-index case is one instance: restore-to-HEAD
  is a correct mutation-undo only when HEAD is the baseline you want.** When you have
  uncommitted work in the file you are mutating, either commit first and mutate against
  the committed tree, or back the file up outside git (`/bin/cp -f`) and restore from
  that. **Then check that the tests which failed are the ones the mutation should have
  broken** — a count alone will not tell you the run was contaminated.
