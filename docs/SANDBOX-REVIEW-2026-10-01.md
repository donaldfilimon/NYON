# Sandbox review and expansion roadmap

Date: 2026-10-01, America/New_York. Status: **Partial review; Proposed expansion**.

This is a source-grounded review and prioritization artifact, not an approved
rules specification or an implementation plan. Existing dirty work is retained.
No commit, publication, dependency installation or gameplay change is included.

## Product direction confirmed in this conversation

Cover the whole sandbox: complete the accepted Living Galaxy experience, deepen
simulation, and improve the player/creator experience. Design new mechanics now
across living societies, diplomacy/conflict and galaxy ecology. Societies remain
autonomous; the creator intervenes and edits policies rather than playing a
direct-management faction. Preserve the continuous, nonterminal sandbox.

The accepted September contracts remain the completion baseline. Expansion
choices below are recommendations awaiting design review, not accepted numerical
rules. New stateful mechanics require separately versioned authority, catalog,
wire, archive and replay contracts; existing V2 files retain their meaning.

## Current evidence and corrections to older ledgers

Inspected canonical main checkout, manifests/toolchain, CI, authority model,
rules, staged executor, Session, creator/navigation/Chronicle interfaces,
Notebook verification, storage/transfer boundaries and qualification contracts.
This is not an exhaustive line-by-line security or correctness audit. Earlier
verification records are historical evidence tied to their stated checkpoint.

- **Current source:** V2 supports energy/ore/alloy, local inventories, industry,
  finite deposits, construction, freight, scouting/settlement, limited actor
  intelligence, bilateral agreements, war/truce, combat and dormant recovery.
- **Current source:** 46 creator tools; searchable entity hierarchy; camera
  navigation; Chronicle categories, actor/entity following and pause categories;
  history branches, comparisons, Notebook, save/Continue and portable transfer.
  These should not be proposed as absent features.
- **Partial:** staged `LivingStepJobV2::poll` reaches diplomacy. Intents onward,
  final validation/sealing and some creator operations still require explicitly
  synchronous completion. Resident `SandboxSession::step_once` calls
  `authority.step()`; full resident budgeting is not established.
- **Current source repair:** funded scout selection uses the funded witness;
  assembled attack uses one eligible reachable fleet; useful existing routes can
  be adjusted. Older rules-audit findings are not proof these defects persist.
- **Current focused automated evidence:** linked Trade and Supply observations,
  Corridor observation pairs, creator recovery and Chronicle restoration have
  working regression paths. Corridor text explicitly avoids claiming a complete
  controlled experiment. War-alternative recording and portable-reproduction
  completion remain separate gaps in the inspected Session recording path.
- **Historical diagnostic:** capacity authority quiet-start pilots recorded
  p95/max 42.759–77.730 ms against the accepted 5 ms target. Ten samples,
  incomplete host quiescence and later source changes prevent current timing
  acceptance. Reservation lookup optimization exists now; remeasure it rather
  than reimplementing the earlier proposed optimization.

Fresh review command:

```sh
cargo test --test living_trade_completion --test living_supply_ui \
  --test living_corridor_journey --test living_recovery_evidence \
  --test living_chronicle --test living_creator_qualification
```

Actual exit 0: **35 passed, 0 failed, 0 ignored, six suites**. Counts: Trade 7,
Supply 1, Corridor 2, recovery 6, Chronicle 10, creator qualification 9.
Log: `/tmp/nyon-sandbox-roadmap-review-tests.log`. The creator sweep exercises all
46 forms through valid preview/apply/undo/redo/reload and rejection checks; nine
tests are not nine operations. Host storage evidence is not live GUI/IndexedDB.
Full native/fuzz/wasm gates, fresh release/bundles and runtime matrix were not
run by this review at this point.

Fresh `cargo test -p nyon-workshop-core --lib staged_step_` also exited 0:
**18 passed, 0 failed, 0 ignored, 136 filtered**. Log:
`/tmp/nyon-sandbox-roadmap-step-tests.log`. Covers staged parity, queue/cancel/stale
publication, authentic faults and diplomacy cadence. Combined focused review:
**53 tests passed**. This does not establish fully bounded resident stepping.

## Completion inventory

All 27 accepted qualification IDs remain applicable. Completion requires their
specified evidence, not merely an implemented function or an older green gate.

| Workstream | IDs | Remaining work and acceptance evidence |
| --- | --- | --- |
| Diorama and workspace | LG-01–06 | Qualify compositing, atlas-measured labels, all entity inspectors, hierarchy reachability/virtualization, conventional editing, fit/navigation/removal recovery. Run declared dimensions/scales, resize during editing/modal/history work, first/last/deepest rows, zero overlap and 44 logical-pixel targets. |
| Authority, economics, societies | LG-07–09, LG-12, LG-27 | Finish whole-boundary executor and resident integration; complete candidate/demand/replacement fixtures; test render-cadence independence, actor-private intelligence, shared reservations, zero/one/many societies, insertion-order parity, tail admission and atomic faults. Preserve exact original V2 vectors. |
| Diplomacy and conflict | LG-10–11 | Exercise bilateral proposal/expiry reasons, assembled fleet eligibility, simultaneous multi-faction rounds, occupation/retreat, credits/refunds, itineraries, capture and last-colony recovery; inspect actual results through UI. |
| Chronicle and experiments | LG-13–14, LG-21 | Qualify recorded reasons and historical event restoration; complete controlled Corridor, war plus alternative and portable reproduction. Preserve exact branch/tick/receipt links through save/reopen/transfer; test more than four retained branches and matching-tick comparisons without causal overclaim. |
| Durable lifecycle and portability | LG-15–16, LG-25–26 | Actual adapter quota/corruption/interruption/CAS/cancel-race journeys; missing/mismatched catalog; pack generation/reclamation; coherent archive/catalog/notebook handoffs; isolated new-slot import; exact Continue and next-1000-tick parity. Preserve active session on every failure. |
| Compatibility | LG-17 | Classic and WorkshopV1 byte/digest oracles plus live legacy open journey; strict kind routing, unchanged slot precedence and no copy-on-read rewriting. |
| Accessibility and graphics | LG-18–19, LG-22 | Pointer/keyboard/screen-reader equivalent journeys, focus ownership, contrast/reduced-motion/mute, native surface/device recovery, browser fallback, actual audible/no-device cleanup and presentation digest isolation. |
| Performance and delivery | LG-20, LG-23 | Named-device authority/frame/input measurements, bounded replay/import/history memory, mixed 36000/100000-tick runs, full offline journey, current manual and evidence ledger. Commits/publication require a separate user request. |

Full journey and platform matrix are defined in
`docs/superpowers/specs/2026-09-04-nyon-living-galaxy-qualification.md`:
macOS Metal and Safari/Chrome; Windows DX12 and Edge/Chrome; Ubuntu Vulkan/GL and
Firefox; natural browser backend and forced WebGL2. Missing runtime evidence is
**unverified**, not automatically Blocked. Layouts: 723x802 at 100/115/130%,
1280x720, 1440x900 and 1920x1080 at 100/130%, with high-DPI changes.

## Proposed expansion design

Recommended approach: one versioned expansion contract, delivered as dependent
vertical slices. Each mechanic includes autonomous decisions, creator preview,
inspector, receipts, history, persistence, content and tests. Designing all three
systems now prevents population, warfare and ecology from inventing incompatible
resource or clock models. Shipping each slice requires its complete journey.

### Living societies

- Colony population with explicit needs, carrying capacity, growth/decline and
  recovery. Aggregate integer cohorts; avoid individual-person simulation.
- Habitation and life-support/food production with local storage and transport.
  New resources and recipes must be explicit catalog entries; no hidden global
  stockpile or fabricated supply.
- Specialization through workforce allocation and infrastructure choices; make
  opportunity costs visible in industry demand/output and construction decisions.
- A finite research graph funded by real costs. Technology unlocks reviewed
  capabilities; research intent, progress and completion have recorded reasons.
- Autonomous policies choose sustainable growth, recovery, specialization and
  research from actor-visible information. Creator edits are provenance-labeled
  overrides and can be undone/replayed like other interventions.

### Diplomacy and conflict

- Bilateral negotiated offers with explicit terms, expiry, acceptance/refusal and
  recorded reasons. Add transit/access, resource commitments and alliances only
  with defined obligations and admission/capacity rules.
- War objectives tied to actual interests: defend a colony, secure access or
  occupy a named objective. Define completion, abandonment and peace terms; no
  mandatory total conquest or forced sandbox ending.
- Logistics consumes explicit local resources and transport capacity. Blockade,
  supply disruption and relief must interact with the same freight/economy rules.
- Fleet roles and autonomous plans expose target, route, readiness, known risk,
  supply and fallback. Preserve shared-snapshot decisions and central reservation
  so actors cannot spend the same stock or hull twice.
- Alliances cannot reveal omniscient state implicitly. Any intelligence sharing
  needs explicit source/timestamp/permission semantics and stale-data tests.

### Galaxy ecology

- World habitability, environmental stress and renewable production capacity
  change on deterministic scheduled boundaries. Finite mineral depletion remains
  distinct from renewable replenishment; neither duplicates resources silently.
- Stellar activity and disasters affect identified worlds/lanes and expose
  warning, interval, effects and recovery. Use seeded, versioned schedules;
  rendering/audio/wall-clock randomness never changes outcomes.
- Societies adapt through infrastructure, specialization, research, routes and
  migration/settlement choices. Environmental resilience competes with expansion
  and warfare for real resources.
- Creator can preview/schedule/modify environmental interventions and see the
  affected entities. Removal/restoration must include complete dependent state.

### Player and creator experience across all mechanics

- One inspector model feeds visible rows and semantics. Add demand/shortfall,
  population trajectory, research, treaty obligations, war objectives and ecology
  timelines alongside current economy/history facts.
- Separate creator omniscience from each society's observed information; show
  freshness, decision candidates and rejection reasons without implying the actor
  knew hidden quantities.
- Add economy, population, diplomacy, logistics, intelligence and ecology layers
  only as read-only projections. Preserve readability through aggregation while
  every underlying entity remains individually inspectable.
- Contextual creator forms show before/after values, costs charged to autonomous
  actors, free creator overrides, dependencies, cascades and application tick.
- Experiments compare exact branches at matching ticks. A structured difference
  can support a local intervention comparison; it is not a universal causal law.
- Portable bundles carry exact rules version, catalog, genesis, history and
  Notebook evidence. Imported files remain untrusted and bounded.

## Dependency order and interfaces to specify next

1. **Completion foundation:** finish shared-budget executor, forecasts,
   dispatch, validation/sealing, retirement and resident transport; qualify
   current proof/storage/UI seams. Profile exact current artifacts and retain the
   synchronous reference until complete state/all-receipt parity is established.
2. **Expansion compatibility:** specify the next rules domain, isolated types,
   catalog schema, capacity limits, canonical byte formulas, clocks, phase order,
   receipts and archive routing. V2 remains readable in V2. No implicit conversion;
   any conversion must be explicit, separately approved and independently tested.
3. **Society/economy substrate:** population, needs, production, specialization
   and research, with creator tools and complete proof/storage/UI journeys.
4. **Ecology/adaptation:** connect habitability and deterministic environmental
   schedules to that substrate; qualify depletion, renewal and recovery.
5. **Diplomacy/logistics/conflict:** terms, obligations, objectives and supply use
   the established needs/resource/environment interfaces. Specification is drafted
   alongside the other mechanics; execution follows the interfaces it consumes.
6. **Integrated content and acceptance:** validated scenarios demonstrating each
   interaction, controlled experiments, portable reproduction and the full
   platform/accessibility/offline/endurance matrix.

Detailed specs must settle formulas, rates, policies, catalog definitions,
resource conservation, intelligence visibility, negotiation/objective semantics,
capacity rejection and replay compatibility before writing implementation plans.
This roadmap intentionally does not manufacture those rules as accepted values.

For every slice: deterministic fixtures, invalid/capacity atomic rejection,
different polling budgets, stale/cancelled jobs, native/wasm byte/digest parity,
save/reopen/transfer and visible/semantic equivalence. Run native fmt/strict
workspace Clippy/tests/release, independent fuzz fmt/locked Clippy, paired browser
builds/wasm checks/Clippy/Workshop checks and CI's vertex-layout/vector/cross-build
verifiers as applicable. Live native/browser, actual IndexedDB, screen reader,
adapter parity and performance remain distinct acceptance layers.
