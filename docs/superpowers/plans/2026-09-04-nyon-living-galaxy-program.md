# NYON Living Galaxy Program Execution Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to execute this program. Use the linked child plans as the exact task briefs and maintain one implementation agent plus independent task/final reviewers at each shared-state boundary.

**Goal:** Safely turn the approved Living Galaxy design and its four implementation plans into one recoverable, reviewed, qualified NYON product without losing the existing uncommitted Workshop V1 implementation.

**Architecture:** Establish the current Workshop V1 tree as a reviewed and gated baseline, then execute four child plans through an explicit dependency graph. Authority schemas land before civilization rules; the measured UI/renderer foundation lands before Living presentation; persistence and coordinator contracts land before client lifecycle/transfer. Every slice uses RED/GREEN/refactor, exact-path commits, independent review, and evidence appropriate to the claimed layer.

**Tech Stack:** Rust 2024/nightly-2026-09-01, winit, wgpu, AccessKit, wasm-bindgen/web-sys, deterministic `nyon-workshop-core`, native filesystem persistence, IndexedDB, SDF UI, Playwright qualification, and Superpowers task ledgers.

**Child Plans:**

- `docs/superpowers/plans/2026-09-04-nyon-living-galaxy-authority.md`
- `docs/superpowers/plans/2026-09-04-nyon-living-galaxy-civilizations.md`
- `docs/superpowers/plans/2026-09-04-nyon-living-galaxy-foundation.md`
- `docs/superpowers/plans/2026-09-04-nyon-living-galaxy-experience.md`

## Program Invariants

- Preserve unrelated user work. Never reset, clean, stash, or broadly stage the canonical checkout.
- The current dirty Workshop V1 tree is an identified program artifact, not disposable noise. Review and gate it before committing; do not create Living work from `b043a15` alone because that omits Workshop V1.
- Keep Classic, WorkshopV1, LivingV2, Library coordinator, presentation, audio, and platform adapters as separate authorities/evidence layers.
- A child task may consume another plan's public contract but cannot redefine it. Resolve interface drift in the plan before implementation.
- Use one implementer at a time as required by subagent-driven development. Parallelize read-only review, verification, and research only when files/state do not overlap.
- Every implementer receives explicit path ownership and the warning that other work exists. The controller never repairs implementation directly; it returns findings to the owning implementer.
- Every task receives a requirements review, code-quality review, controller rerun of focused gates, exact-path commit, and ledger update.
- Do not claim live native/browser, accessibility, performance, durable storage, download persistence, cross-host parity, or provider CI from source/build tests.

## Dependency Graph

```text
W0  Workshop V1 baseline review, fresh gates, commit
 |
 +--> A1-A4  Living wire, catalog, state, commands
 |      |
 |      +--> C1-C12  civilization/economy/fleet/diplomacy/combat/fixtures
 |      |      |
 |      |      +--> A5-A6  boundary orchestration, history/replay/archive
 |      |
 |      +--> A7-A11  stores, coordinator, import routing
 |
 +--> F1-F7  measured UI, virtualization, editing, camera, renderer, SDF, qualification
        |
        +----------------------------------------------+
                                                       |
 A6 + A11 + C12 + F6 --------------------------------> E1-E12
                                                       |
 C13 + F7 + E12 --------------------------------------> E13-E14
```

`C13` can run after A6 because it exercises replay/archive. `F1-F4` can be scheduled while civilization behavior is under review, but implementation remains serialized. `E1` starts only after the authority types it imports are stable. `E5/E9/E10` require the foundation renderer and layout contracts. `E13/E14` are the only whole-product qualification closeout.

### Program Task 0: Preserve and establish the Workshop V1 baseline

**Files:** All paths currently listed by `git status --short`, excluding the new 2026-09-04 Living plan/review documents, are the candidate Workshop V1 baseline.

- [ ] Read `.superpowers/sdd/nyon-v2/progress.md` and the 2026-09-02 design/plan; freeze the candidate path inventory and current diff hash.
- [ ] Obtain independent review of tracked and untracked source, tests, tools, CI, docs, and the ledger. Repair every P0-P2 through its owning agent; record accepted P3 items.
- [ ] Run fresh combined-tree gates: workspace fmt/clippy/tests, root/core wasm checks and wasm clippy, fuzz-workspace fmt/locked clippy, `check-workshop.sh`, and `git diff --check`.
- [ ] Rebuild WebGPU/WebGL2 artifacts before browser claims; artifact existence/freshness is not runtime acceptance.
- [ ] Confirm no unexpected process/session is still writing candidate paths.
- [ ] Stage the frozen candidate allowlist only. Review `git diff --cached --name-status`, `--stat`, and `--check`; verify plan/review files and unrelated untracked paths are excluded.
- [ ] Commit with a conventional message derived from the final staged diff and record exact gates in the ledger. This commit creates the branchable recovery point; it does not complete live runtime qualification.

### Program Task 1: Commit the reconciled implementation plans

**Files:** The five child/program plan documents only.

- [ ] Require the Superpowers header, Goal/Architecture/Tech/Spec, exact tasks/files/interfaces/tests/commands, dependency order, and completion boundary in every child plan.
- [ ] Scan for unfinished markers, contradictory type names, overlapping ownership, fixed list truncation, implicit V1 conversion, browser disk-persistence claims, and presentation data in authority digests.
- [ ] Confirm `RenderScene` is foundation-owned; neutral coordinator IDs are authority-owned; civilization phase order is authority-owned; experience only consumes these contracts.
- [ ] Commit only the plan allowlist with `docs(nyon): plan living galaxy implementation`.

### Program Task 2: Execute authority schema tasks A1-A4

- [ ] Create an isolated worktree from the reviewed Workshop baseline using the required worktree procedure and a project-specific branch.
- [ ] Create the Superpowers task ledger from the authority plan.
- [ ] For each task A1-A4: fresh implementer, RED evidence, minimal GREEN, requirements review, code-quality review, controller gate, exact commit, ledger update.
- [ ] Do not begin civilization code until the frozen public types, operation variants, event ordinals, and vector registry are reviewed together.

### Program Task 3: Execute civilization tasks C1-C12 and authority A5-A6

- [ ] Reconcile the remaining blocked/rejection and event-payload spellings against A4 before the first civilization edit.
- [ ] Execute C1-C12 in plan order. Preserve the authority-owned phase order, state field order, IDs, queue/cursor semantics, and atomic candidate boundary.
- [ ] Execute A5 after the phase hooks exist; execute A6 after deterministic behavior and four strategic fixtures are frozen.
- [ ] Run insertion-order, cadence, fixture witness, replay, same-tick Undo/Redo, save/reload, archive, wasm, and real fuzz gates.
- [ ] Keep C13 pending until persistence/client integration if its endurance harness needs the final archive/store surface; otherwise run it immediately after A6.

### Program Task 4: Execute persistence/coordinator tasks A7-A11

- [ ] Implement and review Memory, native, and browser transaction models independently.
- [ ] Negative-test cancellation on both sides of the durable commit point and recovery after every injected fault.
- [ ] Implement the neutral global Continue coordinator without adapter recency fallback.
- [ ] Prove the import router is bounded and read-only and every V1 identity/golden remains unchanged.
- [ ] Record native filesystem model and IndexedDB host-model evidence separately from live platform durability.

### Program Task 5: Execute foundation tasks F1-F7

- [ ] Start from the live-observed defects: blank Workshop canvas, clipped creator fields, fixed collection truncation, and primitive/semantic visibility mismatch.
- [ ] Execute measured layout and virtualization before editing, camera, renderer, and SDF integration.
- [ ] Keep `RenderScene`, `DioramaFrame`, `WorkshopLayout`, `VisibleWindow`, `WorkshopViewState`, and `build_platform_ui_batch` as the shared handoff.
- [ ] Re-run Classic/Workshop compatibility after every renderer/input slice.
- [ ] Capture native and both browser-backend evidence at the required viewport/scale matrix; mark unavailable rows pending.

### Program Task 6: Execute experience tasks E1-E12

- [ ] Reconcile imports against the landed authority/foundation types before each task.
- [ ] Implement pacing/holds/session epoch, runtime/Continue, Library lifecycle, starters/experiments, navigator/inspector, creator forms, Chronicle, history comparison, semantic zoom, app/SDF integration, file transfer, and isolated audio in plan order.
- [ ] Every modal/dialog acquires a hold and cannot cause catch-up. Every import creates a new slot and replaces the active session only after durable slot commit and coordinator CAS.
- [ ] Browser export reports handoff only. Audio failure/mute and presentation preferences cannot affect ticks or digests.
- [ ] Re-run Classic and Workshop regressions after every shared app/platform/UI edit.

### Program Task 7: Execute E13-E14 and final program review

- [ ] Run the complete automated journey, both browser artifacts, compatibility suites, source gate, benchmarks, endurance, offline journeys, and exact save/export/import/reload continuation.
- [ ] Perform pointer-only, keyboard-only, and screen-reader paths on every available named runtime row.
- [ ] Record exact source revision, scoped dirty diff hash, artifact hash, catalog/seed/fixture, backend, OS/browser/GPU, digest, evidence paths, and actual outcomes.
- [ ] Update player documentation only from observed qualified behavior.
- [ ] Request one final program reviewer to inspect the full branch diff against all four specs and all open ledgers.
- [ ] Repair all P0-P2, rerun affected gates, merge the implementation branch into canonical `main`, remove its worktree/branch, and verify canonical `main` at the merged commit.
- [ ] Report separately: source gates, builds/artifacts, native runtime, browser rows, persistence, accessibility, performance/endurance, cross-host rows, and any pending release evidence.

## Completion Definition

The program is source-complete only when all child tasks and final reviews are committed on canonical `main` with their automated gates green. It is whole-product qualified only when the available live runtime, accessibility, durability, performance, offline, and manual visual rows carry exact evidence and unavailable required rows remain explicit. No terminal five-world victory is required in Living V2; Classic RulesV1 retains its original win/loss behavior unchanged.
