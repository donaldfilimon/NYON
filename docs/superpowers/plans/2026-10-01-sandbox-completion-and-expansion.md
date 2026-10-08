# Sandbox Completion and Expansion Implementation Plan

2026-10-08 verified prerequisite milestone: see the
[Living completion ledger](../../qualification/2026-10-08-living-completion-ledger.md).
Native and paired wasm gates are green for the recorded candidate; whole V2
completion, player qualification and V3 approval remain open. The intentionally
refreshed toolchain is `nightly-2026-10-08`; the original plan stack below is
historical context.

> **For agentic workers:** Use the native task-by-task execution workflow. Design tasks produce reviewed specifications before implementation tasks for new mechanics begin. Do not spawn agents, commit, push or publish without the user's authorization.

**Goal:** Complete and qualify the existing Living Galaxy sandbox, then deliver autonomous population/research, evolving ecology, negotiated diplomacy and objective-driven conflict with complete creator, history, persistence and player workflows.

**Architecture:** Deterministic authority owns all gameplay. Session polls bounded jobs and installs only complete, fresh results; presentation and audio remain read-only. Preserve V2 interpretation and introduce new mechanics through a separately versioned rules/catalog/archive contract.

**Tech Stack:** Rust edition 2024, nightly-2026-09-01, winit 0.30.12, wgpu/Naga 30.0.1, wasm-bindgen 0.2.127; native, WebGPU and WebGL2 adapters already in this repository.

**Spec:** Existing completion requirements are the approved September Living Galaxy design, rules, experience and qualification contracts under `docs/superpowers/specs/`, including the creator addendum and explicitly accepted October additions. Expansion direction and current source inventory are in `docs/SANDBOX-REVIEW-2026-10-01.md`. This is a program implementation plan: expansion specification tasks must precede their code tasks. It does not represent unreviewed numerical mechanics as approved rules.

## Global constraints

- Autonomous societies; creator interventions and editable policies; no direct-management faction or mandatory terminal victory.
- Cover completion, societies/research, ecology, diplomacy/conflict, player tools, experiments and sharing.
- Integer fixed ticks, deterministic phase order, shared-snapshot proposals and central reservations; no GPU/clock/UI/storage dependencies in authority.
- Preserve V1 and V2 identities, original vectors and archive interpretation. New mechanics use an isolated versioned domain; no implicit conversion or migration on read.
- Preserve existing dirty work in the canonical checkout. No generated-file hand edits or production dependency additions without authorization.
- Shared-budget poll limit remains 1..=1024 visited records for accepted V2 paths; counting a whole forecast/tick/command as one visit is insufficient.
- Public untrusted receipts/imports retain full validation. Performance shortcuts require authority-owned freshness/integrity evidence.
- Completion evidence separates source, tests, builds, live adapters, accessibility, hardware and performance.

## Review focus

1. Stale/cancelled work must not consume a newly admitted queue tail or overwrite a different resident session.
2. Capacity-sized nested state, history and retirement must not hide unbounded work inside preparation, commit or destruction.
3. Scarcity and simultaneous proposals must not duplicate resources, reserve a hull twice or leak hidden actor information.
4. Exact historical views and evidence must survive save/Continue/transfer after later edits and branch changes.
5. Compact/high-DPI editing, modal ownership and degraded rendering must preserve all information and reachable controls.

## Task 1: Establish the current qualification baseline

**Ownership:** verification and the review/evidence documents; no incidental source cleanup.
**Consumes:** current shared checkout and approved LG-01–27 acceptance contract.
**Produces:** dated command outcomes, source/artifact identities, reconciled gaps and a platform evidence matrix.

- [ ] Finish the already started full workspace test run; record actual terminal status, counts and failures. Do not restart a live job merely because a wait timed out.
- [ ] Run native, separate fuzz and wasm/both-backend gates exactly as documented in AGENTS.md; also run CI vertex-layout, original/additive vector and cross-build verifiers.
- [ ] Associate every outcome with the tested source and resulting artifact hashes. Preserve historical checkpoints instead of relabeling them current.
- [ ] Reconcile each LG-01–27 row to source, tests and live evidence. Classify missing evidence separately from a confirmed implementation defect.

**Acceptance:** complete evidence inventory, no stale success promoted to current. Missing platform hosts remain unverified unless a concrete attempted prerequisite establishes Blocked.

## Task 2: Complete bounded execution and history

**Ownership:** core living simulation/step/production/rules/archive jobs; Session transport and runtime installation.
**Consumes:** existing staged executor through diplomacy, standalone dispatch, private capture/commit fencing and frozen reference behavior.
**Produces:** one complete executor used by synchronous compatibility stepping and resident polling; bounded replay/comparison/retirement.

- [ ] Add parity cases before replacing each remaining synchronous phase: intents/forecast, dispatch, validation, all receipt sealing, history publication and complex creator cascades.
- [ ] Poll the 200-boundary forecast's children under the same remaining visit budget. Preserve actor-private information, exact event order and reservation semantics.
- [ ] Integrate the existing dispatch cursor rather than creating a second dispatch algorithm.
- [ ] Prepare all fallible state validation, digest/receipt work and journal growth before a freshness-checked publication. Return retired ownership and reclaim it through bounded work.
- [ ] Replace Session's synchronous resident step with one pending job. Preserve Step, 10 Hz pacing, holds, requested/effective speed, fault display, accepted queue and stale-session fencing.
- [ ] Bound restore preparation, archive decode/graph scheduling, branch replay, comparison output and cleanup; cancellation must preserve the active state.
- [ ] Compare complete state, ordinary/decision/fact receipts, queue, IDs, allocator marks and history against the retained reference at budgets 1, 64 and 1024.

**Acceptance:** no explicit synchronous fallback on the accepted resident path; cancellation/staleness/fault tests and existing vectors pass. Poll-budget correctness and measured timing are separate gates.

## Task 3: Finish the accepted V2 game and experiments

**Ownership:** current rules, creator tools, Chronicle/Notebook, navigation/inspectors and content fixtures.
**Consumes:** existing V2 contracts and completed bounded execution.
**Produces:** accepted autonomous economy, diplomacy/conflict and seven evidence-backed experiment workflows.

- [ ] Close legal candidate enumeration, demand/output classification, full-slot replacement and scarcity fixture gaps; retain repaired scout/assembled-fleet/route-adjustment behavior.
- [ ] Exercise simultaneous diplomacy/combat, capture/retreat/credits/refunds and dormant recovery; display recorded reasons and intelligence timestamps.
- [ ] Qualify all 46 creator tools across paused/running boundaries, capacity/reservations, dependency cascades, atomic rejection and exact undo/redo/reload.
- [ ] Complete Corridor's controlled intervention workflow and war-plus-alternative comparison with exact linked bookmarks/receipts; do not certify causality from unrelated observations.
- [ ] Complete portable reproduction from an authenticated exported/imported artifact pair and subsequent matching simulation results. A receipt alone cannot prove file transfer.
- [ ] Preserve all seven experiment evidence links through save, fresh Continue and portable import; reject stale/wrong-branch/wrong-party/mismatched evidence.
- [ ] Run accepted Commerce, Frontier Friction, Interrupted Corridor and First Expansion witness deadlines through ordinary authority and visible controls.

**Acceptance:** every experiment has a valid journey and negative proof cases; fixture outcomes are produced by simulation, with no injected narrative success.

## Task 4: Qualify the complete player, creator and storage journey

**Ownership:** presentation/UI/platform adapters, Living store/transfer and neutral Library coordination.
**Consumes:** completed current-game flows and exact artifacts from Task 1/updated gates.
**Produces:** complete platform journeys, accessible layouts and tested recovery.

- [ ] Inspect every entity kind and first/last/deepest filtered hierarchy row; retain all branches and use virtualization/scrolling instead of truncated lists.
- [ ] Exercise conventional caret/selection/paste, invalid intermediate fields, owner-tagged ordered input, cancel/focus restoration and resize during forms/modals/history.
- [ ] Verify declared layout sizes/scales, 44 logical-pixel controls, full glyph/readout containment, camera recovery and logical/physical coordinate isolation.
- [ ] Run pointer-only, keyboard-only and screen-reader journeys; preserve information under contrast, reduced-motion, mute and Low rendering.
- [ ] Test actual native/IndexedDB quota, corruption, interrupted generations, CAS, cancellation racing durable commit, pack deletion/re-registration and capacity reclamation.
- [ ] Run exact save/quit/reopen/Continue, missing/mismatched catalog import into a new slot, archive/catalog/notebook export, next-1000-tick parity and legacy preservation.
- [ ] Exercise graphics initialization/fallback/device/surface recovery and no-audio-device cleanup without losing the resident session.
- [ ] Run named-device frame/input/authority measurements, mixed 36000/100000-tick endurance, bounded memory and network-disabled full journeys.

**Acceptance:** meet the approved platform/layout/performance matrix. Targets: authority p95 <=5 ms, native/WebGPU frame p95 <=16.7 ms, WebGL2 Low <=33.3 ms and ordinary input response <=100 ms. Unavailable hosts do not become inferred passes.

## Task 5: Specify the integrated expansion and compatibility contract

**Ownership:** focused design specifications, independent canonical vectors and reviewed catalog/rules definitions.
**Consumes:** confirmed product direction and current architecture; implementation proceeds only after specification review.
**Produces:** one shared compatibility contract and separate society, ecology, diplomacy/conflict and experience specifications with no unresolved normative mechanics.

- [ ] Define the new rules domain, types, wire/hash formulas, catalog schema, capacity limits, archive routing and explicit unsupported-version errors. Existing V2 remains loadable as V2.
- [ ] Specify population cohorts, food/life support, carrying capacity, growth/decline, workforce specialization, research costs/unlocks, clocks and autonomous scoring.
- [ ] Specify habitability/stress, renewable production, finite depletion, warnings/disasters, seeded schedules and adaptation/migration interactions.
- [ ] Specify offer/accept/refuse/expire semantics, access/resource obligations, alliances/intelligence sharing, war objectives, peace and local-resource logistics.
- [ ] Specify phase ordering and conservation across needs, ecology, industry, research, diplomacy, reservations and military supply. Bound every new scan and capacity.
- [ ] Define creator operations and exact cascade previews, visible/semantic inspectors, decision reasons, layers and experiment witnesses for every new mechanic.
- [ ] Review the concrete written specifications, derive independent vectors and then write subsystem implementation plans with exact APIs, schemas and regression fixtures.

**Acceptance:** formulas, caps, phase order, failure cases and compatibility are decision complete. This task produces rules rather than deferring design choices to implementers.

## Task 6: Implement all expansion mechanics as complete vertical slices

**Ownership:** isolated new-version authority plus Session/UI/storage integration described by Task 5 specifications.
**Consumes:** reviewed shared interfaces and rules; produces working autonomous mechanics with creator/history/storage proof.

- [ ] Society slice: population needs/growth/recovery, real local supply, specialization and finite research; include autonomous intent, creator forms, inspectors, receipts and durable replay.
- [ ] Ecology slice: habitability/stress, renewal/depletion, stellar activity/disasters and adaptation; integrate needs/research/routes and preserve deterministic seeded schedules.
- [ ] Diplomacy/conflict slice: negotiated terms/alliances, obligations, objectives, supply disruption/relief and varied fleet plans; retain limited intelligence and central reservations.
- [ ] For each slice, write failing deterministic/invalid/capacity/fault tests, implement the minimum complete behavior, run focused gates and review the full slice before continuing.
- [ ] Test conservation, simultaneous scarcity, stale intelligence, removed dependencies, creator overrides, mixed poll budgets and uninterrupted versus saved/replayed outcomes.

**Acceptance:** each mechanic is available through ordinary player/creator flows and survives exact undo/redo/branch/save/reopen/transfer. No disconnected authority-only feature counts as delivered.

## Task 7: Integrate observation tools, content and sharing; qualify the expansion

**Ownership:** read-only projection, inspector/semantics, creator previews, Notebook, content and portable formats.
**Consumes:** all new mechanics and verified authority interfaces.
**Produces:** coherent expanded sandbox and complete release evidence.

- [ ] Add society intelligence views, need/production explanations, research and environmental timelines, treaty obligations, war objectives and fleet readiness/fallback reasons.
- [ ] Add population/economy/diplomacy/logistics/intelligence/ecology layers while preserving individual inspection and degraded-quality legibility.
- [ ] Add contextual before/after creator previews and read-only exact branch differences at matching ticks.
- [ ] Create validated scenarios exposing scarcity, specialization, research, ecological adaptation, negotiated access, alliance obligations and objective-driven war without scripts injecting outcomes.
- [ ] Package exact rules/catalog/genesis/history/Notebook evidence for portable reproduction; reject unknown versions, mismatched content and tampered evidence atomically.
- [ ] Repeat full gates and native/browser/accessibility/recovery/offline/endurance/performance acceptance for the expanded artifacts. Update manual and ledgers from executed behavior.

**Acceptance:** original completion goals and every new specified mechanic have current evidence; all unique work remains in the canonical checkout. Publication is a separate requested action.

## Execution and current evidence

Recommended execution: native, sequential ownership, one independently testable deliverable at a time. No branch/worktree is needed for this documentation or sequential implementation. Do not create commits merely because a generic skill template lists them.

Current review evidence: 35 root focused tests and 18 staged-step tests passed (53 total). Fresh workspace fmt, strict all-targets/all-features Clippy and original vector verification passed on the subsequent review turn. Full workspace tests were started and must be recorded only after terminal completion. Earlier performance pilots exceeded acceptance; they do not establish the current optimized artifact's timing.

The existing-game completion and expansion are both required by this program. Passing only one subsystem, a headless gate or a single-platform smoke does not finish it.
