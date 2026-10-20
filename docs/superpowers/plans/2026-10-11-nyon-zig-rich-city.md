# NYON Zig Rich City Implementation Plan

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Canceled on 2026-10-19:** Donald canceled further Zig rewrite work. Unfinished Zig tasks and continuation instructions in this document are superseded; retain its historical evidence and existing source. See [the current implementation decision](../../../REWRITE-DECISION-2026-10-19.md).
<!-- END 2026-10-19 rewrite decision -->

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the specified paid city economy, trusted playable capital and observable creator/ruler/combined journeys inside one reviewed city/combat authority transaction.

**Architecture:** City rules operate on the composed authority's unpublished sparse candidate through bounded cursors. The coordinator owns the clock, policy, shared work allowance, history and sole descriptor publication; city and tactical producers exchange typed projections, effects and grants inside that candidate. Initial full-scale economy and existing v0 identities retain their independent codecs and behavior.

**Tech Stack:** Zig/std exactly `0.18.0-dev.146+35accc06e`; integer arithmetic; explicit byte codecs; preallocated sparse storage; no new production dependency or required Rust tool.

**Spec:** [R: rich-city design](../specs/2026-10-11-nyon-zig-rich-city-design.md), SHA256 `c9443864b336bf415b6dc38537118c14438343f9d4ebb0a820025adb5b1a4709`.

**Status:** Proposed city-side implementation plan. Controller approval permits planning after the [narrow fix2 review](../../qualification/2026-10-11-city-contract-reconciliation-fix2-review.md); it does not authorize product implementation. Tasks 0 and 1 are hard prerequisites. No implementation, executable tests, layout witness, complete-story oracle, performance or durability acceptance is claimed by this document.

## Global Constraints

- Compiler: exactly `0.18.0-dev.146+35accc06e`.
- Proposed semantic identity: `nyon.rich-city.v1`.
- Proposed composed rules identity: `nyon.city-combat-galaxy.v1`.
- Proposed composed storage profile: `nyon.city-combat-storage.v1`.
- Proposed scenario identity: `nyon.capital-start.v1`.
- One published tick is one hour; completed tick0 is genesis. Horizon1,000,000.
- Stocks, cargo, costs and incoming reservations use u32 units; population u16, health debt u8 in0..100, needs/growth streaks u16.
- Entity references are `{planet:u32,local:u64}` plus typed kind; cells are planet-qualified.
- Local IDs begin1, increase only for accepted creations, never reuse, and cannot wrap.
- 4,096 cities globally,64/world;16,384 buildings/projects globally,256/world;4,096 local shipments and64/world;36,864 claims globally,576/world and actual surface cell count.
- Each city has at most256 buildings subject to stricter shared world/global caps, population0..512, and eight commodity ledgers.
- Commodity tags: food0,water1,timber2,stone3,ore4,alloy5,tools6,medicine7.
- Per-city stock capacity is512 for each commodity; each completed warehouse adds512, maximum4,096 per commodity after seven effective warehouses.
- Training quotas are2,048 jobs globally,32/world,4/city. Live+reserve units+training reservations<=4,096 global and256/world.
- No allocation during poll. Budget0 changes nothing. Caller allowance0..4,096 is shared by all phases, sorting, graph work, copy/hash, validation, cancellation and retirement.
- At most one bounded record operation, one edge/neighbor, one index movement, one<=256byte copy/zero or one<=64byte hash block per unit.
- All candidate state remains invisible until invariants, receipts and descriptor are complete. Publication consumes one unit and emits/hashes no new bytes.
- No changes to frozen `planet-economy.v0`, `coordinated-galaxy.v0`, `seeded-galaxy.v0`, `nyon.fullscale-galaxy.v1` codec or old profiles. No migration or access to existing user saves.
- One writer owns shared authority modules per task. Fresh independent specification and code review follow every task; findings are fixed before the next writer starts. No commits, pushes, instruction edits, dependencies or deployments are authorized by this plan.

## Review Focus

- Repair and cancellation in the same hour must consult current candidate capacity, including retained over-capacity goods/reservations; Tasks 8,10,11 cover this.
- Interrupted graph/index/route work must preserve free-once page ownership and all cargo/crew until atomic publication; Tasks 4,10,12,13,17 cover this.
- Equal local IDs on different worlds/kinds must never alias targets or training grant producers; Tasks 2,11,12,15 cover this.
- Canonically different but behaviorally similar genesis bytes must be rejected, including passive enabled/priority defaults; Tasks 1–3 cover this.
- A four-person satellite and a clinic-growth city must have an actually feasible workforce, food supply and medicine schedule through the full journey; Tasks 1,6,7,9,16 cover this.

---

## Evidence, scope and ownership

Read the raw [reconciliation review](../../qualification/2026-10-11-city-contract-reconciliation-review.md), [fix1](../../qualification/2026-10-11-city-contract-reconciliation-fix1-review.md), [fix2](../../qualification/2026-10-11-city-contract-reconciliation-fix2-review.md), and [delegated authority](../../qualification/2026-10-11-delegated-game-design-authority.md). Preserve their original statuses. Fix2 approves two city-side corrections only.

Planning-time E is [expanded city annex](../specs/2026-10-11-nyon-zig-expanded-city-economy-design.md), SHA256 `2647a9e43022e37ce0343362684617c295aaa210d48dd0be7ae1d819c6ae829a`; T is [tactical](../specs/2026-10-11-nyon-zig-tactical-ground-combat-design.md), SHA256 `41916c11f31c50b62e012fa162530c4a9629501e49e2f7e3e24f60c72195e08a`. Both still pin intermediate R `32c9e391e9f8936b8124a984b8c83f588aa51017ea8e987a0e551bff362c6668`. They require owner repinning and fresh integrated review; this plan cannot silently update their authority.

The [initial full-scale plan](2026-10-11-nyon-zig-fullscale-authority.md) and [design](../specs/2026-10-11-nyon-zig-fullscale-authority-design.md) are prerequisites, not existing service APIs. Initial design planning pin is `a8bc0d41767722b3215e2e6c334d06d5cf5cacc3107f28f83b3b4219ba6ce2f5`, [initial codec](../specs/2026-10-11-nyon-zig-fullscale-authority-codec.md) pin `c116655859eae999ee1529180521254a575570ba471099fc0ca2e00c8778b6b9`. Its own compatibility and compiled-layout gates remain binding. Read the latest [durable-session design](../specs/2026-10-11-nyon-zig-expanded-durable-sessions-design.md) and independently owned bridge drafts; select one exact composed bridge in Task 0. A 32MiB logical transaction segmented at1MiB is capacity arithmetic, not provider/retention acceptance.

Current source has `zig/src/planet_economy/`, `coordinated_galaxy/`, `seeded_galaxy/`, `surface.zig`, `play_role.zig` and an incomplete `expanded_session/` surface. The new modules below are Proposed. Reuse verified pure adjacency and role rules through explicit conversion; do not call an old island job, cast an old command union, duplicate 512 reservations, or regard unfinished session code as the full bridge. Recheck live owners before every task; do not revert their edits.

Scope includes the city side of tactical projection/effects/training, full shared-authority integration, CLI acceptance and city-specific session qualification. Tactical rounds, hostility production, provider implementation and native/browser graphics have separate owners and plans. Their reviewed implementations are dependency gates for the relevant integrated test, not mock implementations that qualify production. Independent fixture hostility is clearly labeled fixture-only.

## Proposed file boundaries

All paths below are repository-relative. `RC` in task file lists expands exactly to `zig/src/rich_city/`; `CC` to `zig/src/city_combat/`; `FX` to `zig/fixtures/rich_city/`. These directory aliases are used only in this document.

| Files | Responsibility / owner |
| --- | --- |
| `RC/types.zig`, `records.zig`, `recipes.zig` | New strong city IDs, exact tags/values, canonical records, fixed numeric tables; city writer. |
| `RC/codec.zig`, `receipts.zig` | Incremental city family0/kinds2..6/event payloads, accepted effect mapping; city writer. |
| `RC/genesis.zig`, `topology.zig`, `infrastructure.zig` | Trusted capital and metered footprint/network derivation; city writer. |
| `RC/commands.zig`, `construction.zig`, `expeditions.zig` | Ordered command preflight, costs, refunds, founding/resettlement; city writer. |
| `RC/operations.zig`, `needs.zig`, `freight.zig` | Hourly work, power, production, health/population and shipment custody; city writer. |
| `RC/combat_bridge.zig`, `training.zig` | Frozen projection/effect validation, occupation, paid crew/reserve conversion; city writer. |
| `RC/view.zig`, `invariants.zig`, `root.zig` | Copied city queries, city conservation checks, narrow exports; city writer. |
| `CC/bridge.zig`, `types.zig`, `job.zig`, `genesis.zig`, `codec.zig`, `history.zig`, `replay.zig`, `session_bridge.zig`, `root.zig` | New composed adapter, global phases/codecs/publication/history; one named shared-authority writer, city task writer only when expressly holding that ownership. |
| `zig/src/fullscale_authority/` | Separately planned storage/provider engine; reuse only after its acceptance. Any extension requires explicit ownership, new-profile registration and separate review; initial codec remains frozen. |
| `zig/src/rich_city_tests.zig`, `RC/test_support.zig`, `RC/*_tests.zig` | Isolated city and composed regression tests. |
| `zig/tools/rich-city-reference.zig`, `rich-city-layout-witness.zig`, `rich-city-qualify.zig`, `check-rich-city-cli.zig` | Independently written oracle, standalone layouts, workload qualification and CLI harness. |
| `FX/scalars/`, `FX/capital/`, `FX/journeys/`, `FX/custody/`, `FX/malformed/`, `FX/manifest.json` | Exact independent bytes, schedules, expected tables/hashes, schema and input pins. |
| `zig/src/rich_city_cli.zig`, `rich_city_wasm.zig`, `zig/src/main.zig`, `zig/src/root.zig`, `zig/build.zig`, `zig/tools/check.sh` | Later explicit build/CLI/compile probes, owned serially; no host graphics acceptance inferred. |
| `docs/qualification/2026-10-11-rich-city-*.md` | Exact new contract acceptance, witness, task reviews, journey and capacity/performance reports. Never overwrite existing reports. |

## Interface contract proposed for Task 0 ratification

These signatures are design input, not implemented APIs. Task 0 must adopt them or revise this plan consistently before dispatch. `CC/bridge.zig` must bind them to the accepted storage/provider/session contracts; a parallel fake transaction implementation is forbidden. All internal pointers refer to the one preallocated candidate/control block; public callers receive value tokens only.

```zig
// RC/types.zig: distinct enums prevent local/cross-world/kind aliasing.
pub const PlanetId = enum(u32) { _ };
pub const CivilizationId = enum(u32) { _ };
pub const Tick = enum(u64) { _ };
pub const Revision = enum(u64) { _ };
pub const CityId = struct { planet: PlanetId, local: u64 };
pub const BuildingId = struct { planet: PlanetId, local: u64 };
pub const CellId = struct { planet: PlanetId, local: u64 };
pub const ShipmentId = struct { planet: PlanetId, local: u64 };
pub const TrainingId = struct { planet: PlanetId, local: u64 };
pub const UnitId = enum(u64) { _ }; // realm zero; tactical ownership
pub const ObjectiveId = enum(u64) { _ };
pub const Goods = [8]u32;
pub const Digest = [32]u8;
pub const StructureKind = enum(u8) { hall = 0, building = 1 };
pub const StructureRef = union(StructureKind) { hall: CityId, building: BuildingId };
pub const Step = enum { pending, done };
pub const WorkBudget = struct { remaining: u16 }; // one instance, shared by all owners
pub const Failure = error{ InvalidWire, UnsupportedIdentity, InvalidBudget,
    InvalidPolicy, InvalidManifest, UnknownPlanet, InvalidSequence, InvalidCount,
    StaleJob, StaleView, Busy, NotReady, ArithmeticOverflow, InvariantViolation,
    IdExhausted, HistoryBackpressure, ProviderFailure, Faulted, Shutdown, OutOfMemory };
```

`Commodity`, `BuildingKind`, `UnitClass`, `Scenario`, `Reject`, `Phase`, `BuildingStatus`, `StallReason`, `TrainingOutcome`, `PopulationCause` use R's exact explicit tags. `Policy`/`Provenance` explicitly convert the existing `play_role` types after checking values. `Command` is a new union containing the eighteen payloads in R's tags0..17 table in precisely that field order. `CityRecord`, `BuildingRecord`, `ClaimRecord`, `ShipmentRecord`, `TrainingRecord` reproduce R's kinds2..6 field tables. Fixed arrays plus explicit lengths represent footprints<=2/path<=65; no heap slice escapes. Tactical kinds7..9 and family1 remain owned by T and require joint freeze.

```zig
// Public CC/root.zig values; backing storage is independently owned.
pub const JobToken = struct { owner: u64, generation: u64, id: u64 };
pub const ViewToken = struct { owner: u64, generation: u64,
    descriptor: u64, lease: u64, lease_generation: u64 };
pub const RoutedEnvelope = struct { planet: PlanetId, body: []const u8 };
pub const BeginTick = struct { expected: Revision, target: Tick,
    policy: Policy, commands: []const RoutedEnvelope };
pub const GenesisRequest = struct { manifest_wire: []const u8,
    manifest_digest: Digest, scenario: Scenario, policy: Policy };
pub const Progress = union(enum) { pending: void, prepared: void,
    published: Revision, canceled: void, failed: Failure };
pub const CancelResult = enum { requested, already_published, stale, faulted };
pub fn beginGenesis(self: *Authority, request: GenesisRequest) Failure!JobToken;
pub fn beginTick(self: *Authority, request: BeginTick) Failure!JobToken;
pub fn poll(self: *Authority, job: JobToken, allowance: u16) Failure!Progress;
pub fn cancel(self: *Authority, job: JobToken) CancelResult;
pub fn acquireView(self: *Authority) Failure!ViewToken;
pub fn releaseView(self: *Authority, view: ViewToken) Failure!void;
```

`Authority` is the new composition's opaque, preallocated control object over the accepted shared storage engine; allocation/open/provider/shutdown signatures and ownership must be supplied by that engine's approved contract in Task 0. These city facade tokens need explicit checked adapters to its provider/view identities; differing token layouts cannot be bitcast. `begin*` admits bounded references and records a copy cursor; caller buffers remain borrowed until the admitted request-copy phase finishes, signaled by the accepted bridge. The selected bridge must expose that signal and its lifetime rules before implementation. It must not hide an unmetered 1,024-command deep copy inside admission.

`Engine` below is `CC/bridge.zig`'s internal candidate handle: candidate root, expected/target revision, trusted policy, input/emit cursors and the witnessed shared scratch. It exposes only the approved storage's metered lookup/write/delete/index/page operations; its physical layout and exact storage methods are Task 0/1 acceptance inputs. City functions never publish independently.

```zig
// Every stateful city module exports this exact stepping convention.
pub fn stepGenesis(e: *Engine, budget: *WorkBudget) Failure!Step;
pub fn stepCommands(e: *Engine, budget: *WorkBudget) Failure!Step;
pub fn stepProjection(e: *Engine, budget: *WorkBudget) Failure!Step;
pub fn stepEffects(e: *Engine, budget: *WorkBudget) Failure!Step;
pub fn stepFreight(e: *Engine, budget: *WorkBudget) Failure!Step;
pub fn stepInfrastructure(e: *Engine, budget: *WorkBudget) Failure!Step;
pub fn stepOperations(e: *Engine, budget: *WorkBudget) Failure!Step;
pub fn stepNeeds(e: *Engine, budget: *WorkBudget) Failure!Step;
pub fn stepInvariants(e: *Engine, budget: *WorkBudget) Failure!Step;
```

Producer/consumer matching: commands mutate candidate before projection; tactical reads copied projection, emits a validated effect batch; city consumes it at13 and emits exactly one outcome per received effect; tactical pairs outcomes by ObjectiveId inside the candidate. Freight consumes current endpoint ownership at14. Infrastructure produces worker/power/capacity snapshots at15. Operations consumes snapshots at16 and emits building/training/grant receipts. Needs consumes remaining ledgers and care at17. Invariants/codec/history prepare all bytes at18; only coordinator publishes at20.

```zig
// RC/combat_bridge.zig, concrete types accepted jointly with T.
pub const ProjectionRow = struct { target: StructureRef, owner: CivilizationId,
    cells: [2]CellId, cell_count: u8, integrity: u16,
    passable: bool, cover: bool, capturable: bool };
pub const Damage = struct { target: StructureRef, amount: u16 };
pub const Occupy = struct { hall: CityId, objective: ObjectiveId,
    expected_owner: CivilizationId, new_owner: CivilizationId };
pub const Effect = struct { ordinal: u16, effect_id: u64,
    value: union(enum) { damage: Damage, occupy: Occupy } };
pub const EffectBatch = struct { source: Revision, target: Tick,
    tactical_identity: []const u8, effects: []const Effect };
pub const EffectOutcome = struct { kind: enum(u8) { damage = 0, occupy = 1 },
    effect_id: u64, target: StructureRef,
    objective: ?ObjectiveId, previous_owner: CivilizationId,
    new_owner: CivilizationId, integrity_before: u16, integrity_after: u16,
    outcome: enum(u8) { applied = 0, blocked_destroyed_hall = 1 } };
pub const DeploymentGrant = struct { producer: TrainingId, reserved_unit: UnitId,
    owner: CivilizationId, world: PlanetId, hall: StructureRef,
    class: UnitClass, crew: u8, eligible_tick: Tick };
```

Effect ordinals1..N are distinct from receipt ordinals:u32; projected tables/effect arrays live in witnessed scratch and are frozen for their specified phase. The concrete adapter copies/validates them under allowance, never stores a borrowed public callback. The city is sole producer of `DeploymentGrant`; tactical consumes each qualified producer exactly once against the paired roster reservation. No host grant/effect/hostility injection API is exported.

Queries are copied, descriptor-bound and resumable; arbitrary sparse lookup work cannot be hidden in a synchronous getter:

```zig
pub const QueryToken = struct { view: ViewToken, id: u64, generation: u64 };
pub const Query = union(enum) { city: CityId, building: BuildingId,
    shipment: ShipmentId, training: TrainingId, quotas: PlanetId,
    receipt: u32, claims: struct { planet: PlanetId, after: ?u64 } };
pub const QueryValue = union(enum) { city: CityView, building: BuildingView,
    shipment: ShipmentView, training: TrainingView, quotas: QuotaView,
    receipt: Receipt, claims: ClaimPage, absent: void };
pub const QueryProgress = union(enum) { pending: void, ready: QueryValue };
pub fn beginQuery(self: *Authority, view: ViewToken, query: Query) Failure!QueryToken;
pub fn pollQuery(self: *Authority, token: QueryToken, allowance: u16) Failure!QueryProgress;
```

Task14 defines `CityView`, `BuildingView`, `ShipmentView`, `TrainingView`, `QuotaView`, `ClaimPage`; `Receipt` is the exact bounded composed event union frozen by Task0. Query admission/storage lanes, abort/release behavior and max concurrent query controls must be accounted in Task1; copied query results may outlive the lease, internal pointers may not.

## Execution and evidence conventions

Use repository root, exact compiler verified at execution:

```sh
RC_ZIG=/Users/donaldfilimon/.zvm/bin/zig
"$RC_ZIG" version
```

Require exactly `0.18.0-dev.146+35accc06e`. This path/version was read during planning; no test gate was run. All commands/tests below are future execution steps.

For each product task, add a compiling behavior test and observe its intended assertion fail before implementing it. Missing imports, compiler mismatch and stub syntax errors are setup failures, not behavioral RED. Then run its exact filtered Debug and ReleaseSafe commands, followed by the accumulated isolated suite. A fresh spec reviewer then a fresh code reviewer read the exact frozen diff and evidence; preserve their reports and fix findings before dispatching the next task. A test-only harness may build legal independent fixture states; final public journey cannot mutate candidate records directly.

`RC/test_support.zig` owns `pub fn checkCase(comptime name: []const u8) !void`: load the independently pinned `FX/manifest.json` case and its explicit mode (`codec`, `genesis`, `kernel`, or `public`). Codec mode invokes only field cursors; genesis mode drives the trusted builder; kernel mode initializes a legal independent fixture candidate over the accepted storage engine, invokes only the named completed city phase cursors, and compares exact records/events to the independent expected values. Kernel mode makes no public atomicity or tactical/provider acceptance claim. Public mode opens fresh approved storage and feeds exact public genesis/tick/query/cancel actions. Every mode compares typed scalars/bytes and its specified hashes, checks allocator/accounting closure, and destroys. Unknown case, missing expected bytes or an unavailable required action fails; the harness never generates expectations, returns early or skips an unfinished case. Tasks2–12 use only modes whose dependency is complete; Task13 adds public execution of the composed cases, Task15 fresh public replay, and Task16 the complete CLI journeys. `const std = @import("std"); const support = @import("test_support.zig");` is the common import header in each `RC/*_tests.zig`; production symbols in snippets are imported from the named owning module.

The code snippets below name concrete test cases and their required assertions; per-task prose specifies exact inputs/expected values. Tests may not pass merely because `checkCase` returns early. Freeze fixture hashes independently in Task1. For stateful cursor tests, test support wraps the accepted engine with a legal isolated candidate and exposes all mutation traces; it does not invent global atomicity.

## Task 0: Accept the exact composed contracts before any product source

**Files:** New `docs/qualification/2026-10-11-rich-city-contract-acceptance.md`; owner amendments to E/T, a new composed codec/bridge document and this plan only under explicit controller task assignments. Do not edit R, initial codec or other owners' documents as part of this plan's creation.

**Interfaces:** Consumes the exact R/E/T and initial-authority/session artifacts above. Produces an immutable pin matrix and controller acceptance of one complete profile/codec/phase/bridge contract; no executable city API is yet produced.

- [ ] Rehash every input; stop on drift. Have E/T owners adopt final R c944… and the corrected150-byte minted-cargo literal. Obtain fresh integrated review of the resulting E/T revisions, not only their old intermediate snapshot.
- [ ] Freeze all52 quotas; family0 tags0..17; tactical family1; kinds0..9; events0..12,32..36,64..67 where67 belongs to the ratified hostility contract; phases0..21 including four all-world tactical round barriers. Bind exact ownership for accepted/rejected outcomes and objective completion. No unspecified kind7/8/9 bytes or public hostility override is permitted.
- [ ] Select the production hostility producer with authenticated input/revision identity, replay bytes and missing/stale/malformed rejection behavior. A no-hostility fixture cannot satisfy this production dependency.
- [ ] Have authority/session owners accept city `G`, state/history domains, unchanged NYON-FS1 transport domains and NYON-ES1 session domains as distinct registries. Register262,144-byte genesis,65,536-byte initial receipts and12,746,752-byte tick reservation; retain32MiB logical transactions and1MiB transport segments only if their exact reviewed provider contract says so.
- [ ] Accept the proposed interfaces above and fill the composed bridge with concrete storage/provider allocation, bounded request-copy lifetime signal, query lane limits, lease ownership, source/replay-target ownership, retention/GC and shutdown signatures. Amend this plan's interface section to the accepted declarations and hash it. This step is an explicit missing prerequisite, not implementer discretion.
- [ ] Require the initial full-scale owner's Task0 compatibility and compiled controls/layout acceptance. Approve only the isolated Task1 witness/oracle work next. Product Tasks2–18 remain blocked until the city-specific witness and full journey artifacts pass review too.
- [ ] Record executable dependency acceptance separately from design acceptance: Task2 may use only reviewed pure value/codec interfaces after Task1; Tasks3–12 require the separately implemented and qualified sparse storage/candidate/budget engine; Task13 requires completed tactical rounds, production hostility and shared provider/publication engine; Task18 requires completed durable-session/provider implementation. Waiting dependencies never become a local fake backend or an unreviewed public API. Record one writer and reserved file list for every task, fresh reviewer identities, exact accepted pins and blocking conditions. Delegated controller decisions satisfy design approval; do not introduce an extra human confirmation loop. Do not interpret documentation approval as successful runtime qualification.

**Acceptance:** Independent review has no unresolved composition/schema/ownership blocker. Missing tactical payloads, changed pins, unaccepted session bindings or unspecified page ownership stop dispatch. No RED/GREEN product claim is applicable to this documentation gate.

## Task 1: Freeze independent layout, scalar and complete-story witnesses

**Files:** Create `zig/tools/rich-city-layout-witness.zig`, `zig/tools/rich-city-reference.zig`, `FX/manifest.json` and the five fixture subdirectories; create `docs/qualification/2026-10-11-rich-city-prerequisite-witness.md`. Witness-only authorization follows Task0; no `zig/src/` product edits.

**Interfaces:** Consumes accepted exact schemas/bridge layouts. Produces immutable typed case inputs, per-action/per-hour expected records/events, scalar wire files, manifest of SHA256s, complete public journey schedule, independently proven legal quota/workload fixtures and measured compiler layout. Oracle imports Zig/std only, never authority/game/production serializer modules.

- [ ] Implement independent cube-basis adjacency and fixed integer recipe arithmetic. Exhaust all116 seed-residue×side combinations and every directed seam/corner. Assert side4 residue0 hall29; roads `[25,28,30,21,24,26,44,95]`; farm17/east→18; Waterworks20,Forestry22,Quarry40,Solar27,Residence31. Directions use wire east0/west1/north2/south3; template tie order north/east/south/west.
- [ ] Independently encode all capital defaults, empty state, eighteen commands, all rejection tags and every city event including maximal strings/counts/optionals, plus tactical-owner supplied canonical vectors. Check R's849-byte building vector SHA256 `d4587faaf1900d6eeccb988b2d17d22946b00290b0aa948c5256e8e7ef40ee11`;36-byte recruit frame;150-byte transfer/mint/founding frames and their three R hashes. Mutate every passive enabled/priority byte and require changed genesis comparison.
- [ ] Prove actual host storage layouts and explicit encoders: city payload<=98, building<=53, training60, world60; each stored record payload<=256; shipment header plus at most3 owned route pages;52 quota fields; maximum theater framed record80/event128. Count83,216 rows and95,504 pages/root,35,567 remaining under131,071. Show every category and training's single roster-reservation page; never allocate a second one.
- [ ] Independently derive and freeze quota qualification inputs in `FX/custody/quota-workloads.json`, referenced by hash from `FX/manifest.json`. Separate three evidence classes: (a) independently proven legal per-quota extrema and their next-admission/one-over attempts with the exact prescribed first rejection; (b) reachable joint loaded-gameplay workloads with actual city/building/road/farm/claim/shipment/training/unit counts and full action schedules; (c) synthetic physical allocator/codec envelope stress, explicitly not a reachable gameplay state. Preserve every configured quota and the83,216-row/95,504-page conservative envelope. A quota boundary not demonstrated reachable cannot be advertised as a gameplay fixture: retain the configured-boundary predicate/storage test separately and report the legal extremum, proof and earlier admission guard without redefining the quota.
- [ ] For every legal workload, pin manifest/seed/side, exact qualified hall/building/footprint/road coordinates, owners, stocks/reservations, cargo routes/positions/departures, typed public command sequences and per-phase/per-tick counts plus oracle hashes. Prove terrain, exclusive claims, completed connection, route adjacency/uniqueness/endpoints, available inputs and all quota/custody invariants from the independent rules. Label the maximum admission/pre-freight candidate count separately from the published committed count: arrival retirement can lower cargo totals later in the same tick. A route claim or a headline count is not this proof. Do not require the impossible joint4096cities/16384buildings/4096cargo/36864claims tuple: full claims at that city/building count forces all buildings to be farms, uses every hall cardinal attachment slot and leaves no legal freight path. Choose no arbitrary replacement tuple or implementation-generated golden; independently review the actual legal extrema and joint workloads before accepting them for Tasks15,17,18.
- [ ] Compile scratch/control/index-transition/descriptor/query layouts against exact bridge declarations: city fits existing8MiB shared scratch, tactical has its separate8MiB; two building lists256 each and a6,144-cell queue/visited set are reused only after the world cursor completes. Include sort cursors, incremental decoder/hash state, projection/effects/grants and all cancellation ownership bits. Prove old+new roots, two detached transition slots and source+target total;197,197,824bytes/authority and394,395,648bytes/source+target are proposed arithmetic until this witness confirms them. Include provider/session buffers without double-counting shared or missing distinct owners.
- [ ] Derive the complete public journey before product implementation. Use a pinned seeded manifest with a founding world of terrain residue0/side4 (search using the independent seeded formula, record exact global seed and manifest bytes); bind every entity/cell to that exact PlanetId. Pin the capital coordinates above. Allocate new buildings by first free attached legal cell in ascending order and resolve all resulting coordinates into the checked-in action file; no runtime adaptive search remains in the final schedule.
- [ ] The journey must include: two untouched capital hours; paid Residence completion/next-hour activation; Foundry/Workshop with2 construction workers and deliberate industry priorities; paused construction;8 successful Apothecary hours yielding16 extra medicine; disable Apothecary, run Clinic for24 consecutive fully supplied hours and grow exactly1; paid infantry recruitment/completion/deployment plus a separate cancellation branch; paid road corridor, paid satellite founding, regular food imports and operational satellite Waterworks; a rejected overlapping seam placement, full reservation rejection and recoverable shortage. Select exact per-hour commands that maintain supply, and pin every hour's goods/population/debt/work/crew/cargo. Training/care phases require deliberate reduction of industry staffing. The four-person satellite needs sustained food imports and cannot run a four-worker farm.
- [ ] Freeze the finished finite command schedule, exact routes, scalar table and final oracle G/S/H hashes in `FX/journeys/ruler-capital.*`; parallel fixtures `creator-empty.*` and `combined-capital.*` exercise explicit bypass, zero salvage and paid ruler actions in combined policy. A separate seam branch uses qualified path `[70,71,4,5]`, with halls70/5 and roads71/4 explicitly authored; it is not falsely described as the capital's road route.
- [ ] Add custody, damaged warehouse/repair, occupation, freight-capture/cancel and all three paid training classes as independent cases. Include the exact R12 arithmetic anchors; derive all additional expectations without running game code. Each source/sink expectation names its unique receipt emitter.

```zig
// Standalone witness tests; implementation is independent of zig/src/.
test "RC01 capital geometry and explicit defaults" { try verifyAllCapitalTemplates(); }
test "RC01 immutable scalar hashes" { try verifyPublishedScalarAnchors(); }
test "RC01 complete journeys conserve goods people and power" { try verifyJourneyTables(); }
test "RC01 legal quota extrema and joint workload reachability" { try verifyQuotaWorkloads(); }
test "RC01 physical ownership and scratch bounds" { try verifyCompiledLayout(); }
```

`verifyAllCapitalTemplates() !void`, `verifyPublishedScalarAnchors() !void`, `verifyJourneyTables() !void`, `verifyQuotaWorkloads() !void` belong to `rich-city-reference.zig`; `verifyCompiledLayout() !void` belongs to `rich-city-layout-witness.zig` and uses independently declared accepted physical layouts. Start with a known wrong scalar/queue/one-byte-over-bound candidate and observe the targeted assertion fail; remove only that deliberate defect before GREEN.

Run each witness with `"$RC_ZIG" test zig/tools/rich-city-reference.zig -O Debug --test-filter 'RC01 '` and `-O ReleaseSafe`; then the corresponding two commands for `zig/tools/rich-city-layout-witness.zig`. Run `"$RC_ZIG" run zig/tools/rich-city-reference.zig -- --check zig/fixtures/rich_city/manifest.json` to compare, never rewrite, approved vectors.

**Stop:** A complete journey digest is intentionally not invented here. Its absence currently blocks product dispatch. Task1 must finish the exact schedule/table/hash artifacts and legal quota/joint-workload witnesses, independent review and controller acceptance, and append their pins to this plan before Task2. If fit fails, return to the owning design review; do not shrink quotas or loosen ownership assertions.

## Task 2: Implement typed city values and incremental codec

**Files:** Create `RC/types.zig`, `records.zig`, `recipes.zig`, `codec.zig`, `receipts.zig`, `codec_tests.zig`, `test_support.zig`, `root.zig`, `zig/src/rich_city_tests.zig`. Read accepted `CC/types.zig`/`bridge.zig`; only the assigned authority owner writes those files.

**Interfaces:** Consumes Task1 independent scalar files. Produces the strong types above, exact `Command`/record/event unions and `pub fn stepDecode(c: *DecodeCursor, input: []const u8, budget: *WorkBudget) Failure!Step`, `pub fn stepEncode(c: *EncodeCursor, output: []u8, budget: *WorkBudget) Failure!Step`. Cursors have explicit tag, field/byte/element offsets and completion/result ownership in witnessed controls; result is available only after `.done`. `EncodeCursor` holds a typed immutable value; `DecodeCursor` owns a bounded decoded value and consumed length. The composed owner supplies outer/global/tactical framing; city owns only its registered bodies.

- [ ] Add compiling tests that use the independent36-byte recruit literal, all18 command bodies, capital records and150-byte custody outcomes; first use a minimal decoder returning `InvalidWire` for the valid recruit and observe the failed success assertion.
- [ ] Define every scalar tag explicitly. Decode LE field-by-field, validate bool/optional/tag/count before element traversal, and reject trailing/truncated/overlong input. Unknown enum bytes fail structure; known legal enum values with gameplay constraints reach Task5 rejection. Never serialize a native struct.
- [ ] Implement cursors; each bounded256-byte copy and64-byte hash block consumes shared budget. Decode a65-cell route over repeated allowance1; budget0 cannot move offsets, write output or consume input. Validate typed outer world and record key order in the composed codec adapter.
- [ ] Implement accepted command mappings exactly: ship transfer debit only, minted cargo debit/credit zero linked to source=mint input; founding debit-minus-credit=cost; canceled training counterpart=burned global UnitId; zero-effect policies have zero vectors. Ensure scenario stocks and command inputs are not counted twice by later conservation checks.

```zig
test "RC02 exact city scalar round trips and malformed precedence" {
    try support.checkCase("codec-all-tags");
    try support.checkCase("codec-noncanonical-passive-defaults");
    try support.checkCase("codec-foreign-qualified-reference");
    try support.checkCase("codec-route65-budget1");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC02 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC02 '`. Require byte-for-byte equality to independent literals, not only roundtrip equality. Review the complete payload field order against R and accepted tactical framing.

## Task 3: Build receipted trusted capital genesis

**Files:** Create `RC/genesis.zig`, `genesis_tests.zig`; composed writer modifies `CC/genesis.zig`, `codec.zig` and `root.zig` only under task ownership.

**Interfaces:** Consumes `GenesisRequest`, Task2 records/encoder and immutable manifest; produces `stepGenesis(*Engine,*WorkBudget) Failure!Step` for `Authority.beginGenesis/poll`. No command provenance can invoke scenario endowment.

- [ ] Write tests against Task1's full genesis bytes and all116 templates. A compiling empty builder must fail the expected city/building/count/digest assertions for capital start.
- [ ] Use verified `surface.zig` adjacency through explicit checked ID/direction conversion for this generator; Task4 owns the later general city infrastructure cursor, so genesis does not depend on an unfinished Task4 API. Validate trusted role/present civilization and manifest digest, then stage one capital per present civilization, independent of selected UI civilization. Implement trial hall/BFS/placement with metered cursors, temporary claims and no ID consumption for discarded trials. Initialize every field explicitly using R defaults.
- [ ] Produce CityId1, BuildingIds1..14,16claims on each founding planet, next_city2/next_building15; other counters1; zero theater/global state as accepted composed codec requires. Emit one genesis, scenario_city, buildings byID and stocks bytag in exact order. Waived buildings have paid_cost=false and zero salvage; scenario_city stock assertions do not mint a second time.
- [ ] Recompute decoded genesis rather than accepting altered grants. Test every28 enabled/priority byte mutations, wrong initial_policy, city count, unknown scenario, forged command provenance and mismatched manifest. Rejection/cancellation at each candidate claim/event cursor preserves the resident session and old descriptor.

```zig
test "RC03 capital all civilizations and trusted defaults" {
    try support.checkCase("capital-all-civilizations");
    try support.checkCase("capital-side4-residue0");
    try support.checkCase("capital-cancel-every-cursor");
    try support.checkCase("empty-genesis-no-hidden-grants");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC03 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC03 '`. This task proves bounded new-state construction after the accepted storage engine; it does not claim durable replacement of a live saved session.

## Task 4: Meter cube footprints, road access and infrastructure

**Files:** Create `RC/topology.zig`, `infrastructure.zig`, `infrastructure_tests.zig`; modify only the relevant `RC/root.zig` exports and suite imports.

**Interfaces:** Produces `pub fn footprint(side: u16, kind: BuildingKind, anchor: CellId, facing: ?Direction) Failure!Footprint` with `Footprint {cells:[2]CellId,len:u8}`; topology/range failures remain distinct from gameplay terrain checks. Produces `stepInfrastructure(*Engine,*WorkBudget) Failure!Step`; consumed by commands for candidate connection checks and hourly operations. `Direction` is explicit east0/west1/north2/south3. Public command wrappers map checked invalid placement into R rejection precedence.

- [ ] Compare every directed seam/corner to the independent basis oracle. Assert side4 cell71 east→4; reverse by resolved adjacency, not assumed heading. A farm reserves exactly anchor+facing neighbor, fertile anchor; quarry requires rocky anchor. A naive same-face neighbor implementation must fail the seam assertion.
- [ ] Traverse hall/road components once/world with sorted-neighbor cell IDs and same-city ownership; charge lookups, queue entries, edges, copies and clears. Clear reusable scratch once/world. Completed network drives operations; reserved roads permit placement/connectivity validation only. A two-cell farm does not become a transit road.
- [ ] Include ruined hall as network anchor with base512 capacity but zero hall housing. Passive residences/warehouses stay effective while disabled; intact connected solar remains active during occupation. Count activated/intact/connected capacity, not simply building kind count; clamp housing512 and warehouse increments7.
- [ ] Exercise completed/reserved branching roads, overlapping two-cell farm, foreign city road, disconnected project, stale query lease, every cursor cancellation and maximum6144-cell world. Assert visit counters are bounded by claims/edges and never a fresh full surface scan per city.

```zig
test "RC04 seams and single world infrastructure traversal" {
    try support.checkCase("seams-all-directed");
    try support.checkCase("reserved-road-placement-not-operation");
    try support.checkCase("farm-footprint-not-road");
    try support.checkCase("ruined-hall-passive-capacity");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC04 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC04 '`. Review queue membership, scratch reuse timing and charged lookup/index work.

## Task 5: Admit roles, paid building, authoring and policy commands

**Files:** Create `RC/commands.zig`, `command_tests.zig`; modify `RC/receipts.zig`; composed writer connects command-family dispatch in `CC/job.zig` only while owning it.

**Interfaces:** Produces `stepCommands(*Engine,*WorkBudget) Failure!Step` for city tags0..3,8..10 initially; Tasks8–12 add their owned command handlers to this same dispatcher serially. Consumes exact `Command`, `Policy`, candidate capacity/footprints and shared quota/ID reservation API. Produces exactly one typed accepted/rejected outcome per city envelope; structural failure aborts the whole candidate.

- [ ] Add the creator/ruler/combined matrix: creator requires null actor+bypass=true; ruler requires matching present actor and ownership; combined ruler pays ordinary cost. Public autonomous/scenario provenance rejects InternalCommandOnly. Minimal accept-all code must fail forged-scenario and unpaid-ruler assertions.
- [ ] Implement structural precedence before gameplay: lifecycle/revision/target; trusted policy; counts/body lengths; original-ordinal decoding; unknown planet/per-world count; metered global sequence sort and zero/old/duplicate detection. Enforce1024 global/64world. Then apply R's ordered gameplay role/provenance/bypass/target/owner/range/topology/quota/ID/resource checks.
- [ ] Paid build atomically reserves all footprint claims, world/global/city quotas and next ID, charges listed goods once, records remaining=initial work, integrity0/activation0, enabled true/input priority/paid=true. Creator complete buildings use enabled true/priority1/paid=false/integrity1000/activation=target. No-benefit residence/warehouse checks include admitted projects.
- [ ] Implement priority0..3, allowed enable targets, city growth/construction toggles and explicit creator stock/population grants. A grant cannot exceed functional capacity/hard4096 or population512; above-housing population is allowed and observable. A rejected command consumes no resources/IDs; its sequence advances only if the tick publishes.

```zig
test "RC05 role matrix costs defaults and ordered rejection" {
    try support.checkCase("command-role-matrix");
    try support.checkCase("paid-residence-admission");
    try support.checkCase("author-building-zero-refundable");
    try support.checkCase("quota-failure-does-not-burn-id");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC05 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC05 '`. Check dual-invalid inputs identify the first prescribed rejection and structural failures never become a gameplay rejection receipt.

## Task 6: Allocate workforce, power and atomic recipe operations

**Files:** Create `RC/operations.zig`, `operations_tests.zig`; complete `RC/recipes.zig`; modify `RC/infrastructure.zig` snapshot output.

**Interfaces:** Produces `stepOperations(*Engine,*WorkBudget) Failure!Step`; infrastructure supplies the following scratch values and operations produces receipts/remaining budgets for needs:

```zig
pub const HourBudget = struct { available_workers: u16, assigned_workers: u16,
    power_produced: u16, domestic_power: u16, operation_power: u16,
    cared_people: u16, housing: u16, capacity: u32 };
pub fn availableWorkers(population: u16, debt: u8) u16;
pub const Recipe = struct { workers: u16, power: u16, inputs: Goods, outputs: Goods };
pub fn recipe(kind: BuildingKind) ?Recipe;
```

- [ ] Write the operations-only capital first-hour kernel test: before household needs its stock is `[224,288,102,102,26,24,16,8]`;12available/10assigned,24power/3domestic/6operations/15unused. An idle implementation must fail its source/recipe commodity assertions. Task7 adds needs and asserts complete hour1 `[200,264,102,102,26,24,16,8]`, hour2 `[208,272,108,108,28,24,16,8]`, population24/debt0/growth0; do not require an unfinished Task7 implementation to make Task6 GREEN.
- [ ] Encode13 building costs/work and each operating recipe exactly from R; source water40/timber6/stone6+ore2 receipts identify extraction. Farm consumes water8/power4→food32; foundry ore4/power4→alloy2; workshop timber2/alloy2/power4→tools2; apothecary food4/water2/power2→medicine2; clinic medicine1/water2/power2→care32. Spaceport emits awaiting_strategic_rules and consumes zero.
- [ ] Snapshot `floor(floor(P/2)*(100-debt)/100)` after commands/freight/effects; reserve ceil(P/8) domestic power first. Stable metered tier sort is waterworks,farms,clinics,construction,training,industry,spaceports; within tier priority thenBuildingId (training ascendingTrainingId). Construction/training handlers are installed by Tasks8/12; until then pure operation fixtures contain no such work.
- [ ] Preflight enabled→intact/activation→connection→occupation/ruined hall→workers→inputs→power→all positive output capacity. Failed operation consumes nothing. Alloy510/incoming1/cap512 plus output2 idles; alloy509 succeeds. Quarry ore-full stops both outputs. Same-hour successful industry outputs can feed later industry; apothecary cannot retroactively feed clinic. Disabled solar makes0; occupied solar still24; hour totals account unused power explicitly.
- [ ] Test priorities with permuted IDs,0/512population, debt0/99/100, retained over-capacity input consumption, post-input output headroom, disconnected clinics and fractional supply. Idle reason is the first true reason, with no speculative resource reservation retained.

```zig
test "RC06 workers and power snapshot" {
    try std.testing.expectEqual(@as(u16, 12), availableWorkers(24, 0));
    try std.testing.expectEqual(@as(u16, 0), availableWorkers(24, 100));
    try support.checkCase("capital-first-operations-before-needs");
    try support.checkCase("recipe-capacity-atomicity");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC06 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC06 '`. Review arithmetic widths and receipt source/sink uniqueness independently of visual balances.

## Task 7: Implement needs, health, mortality and deliberate growth

**Files:** Create `RC/needs.zig`, `needs_tests.zig`.

**Interfaces:** Produces `stepNeeds(*Engine,*WorkBudget) Failure!Step`; consumes candidate population/stocks, beginning health debt, Task6 `HourBudget` and care. Also produces the independently testable pure function:

```zig
pub const NeedInput = struct { population: u16, debt: u8,
    food_streak: u16, water_streak: u16, growth_streak: u16,
    food: u32, water: u32, housing: u16, domestic_supplied: u16,
    cared_people: u16, growth_enabled: bool, occupied: bool, hall_intact: bool };
pub const NeedResult = struct { population: u16, debt: u8,
    food_streak: u16, water_streak: u16, growth_streak: u16,
    consumed_food: u16, consumed_water: u16, cause: PopulationCause,
    deficiencies: u8, change: enum { none, growth, death, growth_housing_blocked } };
pub fn evaluateNeeds(input: NeedInput) Failure!NeedResult;
```

- [ ] Write the zero-stock healthy4resident sequence: debt16,32,48; exactly1water death in hour3; both shortage streaks reset. Restore complete supply next hour without care→47, with full care→39. Baseline no-op must fail third-hour population3.
- [ ] Consume food/water independently using pre-death P. Accumulate all signed deficiency/care/recovery terms before one clamp0..100. At most one death: water streak3 first, food6 next, debt100 last. Report simultaneous deficiency bits; population0 resets streaks with no free care/growth/death event.
- [ ] Implement clinic reduction floor(8*cared/P) and growth criteria. Streak24 adds exactly1 only with room andP<512, otherwise GrowthHousingBlocked; reset either way. Growth off/occupation/ruined hall resets streak. New resident consumes next hour; population changes do not recalculate the same hour's worker snapshot.
- [ ] Test33people with one clinic gives floor(256/33)=7 reduction but incomplete care blocks growth; room exactlyP blocks at24; full512 cannot grow; partial food/water supply consumes all available and counts shortage. Verify no overflow from widened expressions or streak increment at supported horizon.

```zig
test "RC07 needs mortality precedence and clinic growth" {
    try support.checkCase("capital-two-hours");
    try support.checkCase("water-death-third-hour");
    try support.checkCase("fractional-care33");
    try support.checkCase("growth24-capacity-and-occupation");
    try support.checkCase("empty-city-no-spontaneous-recovery");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC07 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC07 '`. Compare all twelve R arithmetic anchors where applicable, not only the final population.

## Task 8: Finish paid construction, cancellation and safe dismantling

**Files:** Create `RC/construction.zig`, `construction_tests.zig`; modify `RC/commands.zig`, `operations.zig` serially.

**Interfaces:** Consumes Task4 connection cursors and Task6 `HourBudget`; produces construction-tier handler `pub fn stepConstruction(e:*Engine,budget:*WorkBudget) Failure!Step` and handlers for tags4/5 through `stepCommands`. Shares the operations cursor; no independent tick.

- [ ] Paid Residence at hour1 yields `[200,264,90,94,26,24,15,8]`, work4→2; hour2 work2→0/housing32; hour3 housing48. A same-hour-activation implementation must fail hour2 housing assertion.
- [ ] Each successful project takes exactly2workers and subtracts2work. Toggle pause preserves work with no recurring cost. Road project requires adjacency to completed network; reserved chain can be placed but advances outward in later hours. Emit one building status/hour, including completion, with activation=T+1 (allowed horizon+1).
- [ ] Cancellation refunds floor(cost*remaining/initial) per commodity only when paid; unfinished Residence remaining2 returns timber6/stone4/tools0. Completed paid Residence salvage is timber6/stone4/tools0. Authored/genesis structures refund zero despite nominal waived costs.
- [ ] Preflight full refund space before release. Dismantle rejects if reduced housing<P, capacity<stock+incoming, completed/project connectivity fails or active freight uses the cell. Include disconnected existing candidate changes and farms' two claims. Rejection preserves all IDs/claims/work/quota/stock; success retires objects without reusing IDs.

```zig
test "RC08 construction activation refunds and graph protection" {
    try support.checkCase("paid-residence-three-hours");
    try support.checkCase("cancel-partial-floor-refund");
    try support.checkCase("dismantle-route-and-capacity-guards");
    try support.checkCase("free-authored-no-salvage");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC08 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC08 '`. Interrupt every refund/preflight/claim-release cursor under allowances0/1/7; committed state remains unchanged until publication.

## Task 9: Found and resettle supported frontier cities

**Files:** Create `RC/expeditions.zig`, `expeditions_tests.zig`; modify `RC/commands.zig` only for tags7/12.

**Interfaces:** Produces `pub fn stepExpedition(e:*Engine,budget:*WorkBudget) Failure!Step`, consumed by `stepCommands`. Uses existing `CityRecord` and accepted outcome mapping; no route/path claim or interplanetary API.

- [ ] Founding from population12/stock `[100,100,100,100,10,10,10,10]` yields source8 and `[68,68,84,84,10,10,6,10]`, destination4 and `[32,32,0,0,0,0,0,0]` before needs. End its founding hour destination food/water28. A false mint/full-cost transfer must fail summed-stock sink assertions.
- [ ] Require same world/owner, unclaimed target hall, source retains2 after moving4, timber16/stone16/tools4 cost plus32food/32water transfer. New city receives explicit clean defaults, housing16/cap512 and consumes needs that hour.
- [ ] Resettle only population0 destination; require destination housing>=4 and delivery space, source retains2; debit4timber/1tool plus32food/32water and transfer4people. Preserve destination owner, hall integrity, occupation, toggles, unrelated stock/incoming; reset debt/streaks. No hidden hall repair.
- [ ] Check no-aid frontier ends0/0 on eighth hour and first death hour11. Food cargo alone cannot resurrect population0. Reject insufficient source population, cross-world, unknown city, occupied hall and final quota failure without consuming IDs/goods. Run the Task1 food-supported frontier schedule once public integration is available in Task16.

```zig
test "RC09 expeditions transfer custody and retain city defaults" {
    try support.checkCase("found-command-custody");
    try support.checkCase("frontier-eight-hours-hour11-death");
    try support.checkCase("resettle-preserves-ruined-hall-and-policy");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC09 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC09 '`. Verify accepted command debit-credit is exactly the consumed expedition materials.

## Task 10: Preserve freight cargo through roads, reservations, capture and cancellation

**Files:** Create `RC/freight.zig`, `freight_tests.zig`; modify `RC/commands.zig` for tags6/11/17 and `RC/invariants.zig` for cargo/reservation pairing.

**Interfaces:** Produces `stepFreight(*Engine,*WorkBudget) Failure!Step` and command preflight handler `pub fn stepShipmentCommand(e:*Engine,budget:*WorkBudget) Failure!Step`. Consumes current candidate ownership and route cells; produces one `freight_hour` per previously departed live shipment.

- [ ] Side4 route `[70,71,4,5]` has3edges: position0atT,1atT+1,2atT+2,arrivalT+3. Endpoints are distinct same-owner halls, every intermediate cell is completed hall/road of endpoint civilization, all cells unique/adjacent. Reject path1/66/repeats/farm traversal/foreign world/endpoints and departure+horizon overflow. A movement-on-departure implementation must fail position0.
- [ ] Preflight1..64 cargo, source stocks and destination stock+incoming against current functional/hard capacity. Transfer debits source and puts goods only in shipment; mint author-shipment puts goods directly in cargo with explicit source=mint and zero debit/credit. Destination only reserves; reserved arrival changes stock/incoming oppositely, even after warehouse destruction.
- [ ] Visit ShipmentId order. Check source_owner→destination_owner→source_hall→destination_hall→next_edge, retain cargo/position/reservation and increment stalled_ticks once on first failure. Capture destination beforeT+2 stalls at1; reclaim beforeT+3 moves to2;T+4 delivers. No delivery at nominalT+3, no timeout/loss/teleport.
- [ ] Current source owner alone can cancel after source capture. With source stock510/cap512, return4 fails ReturnCapacity; after capacity is restored or stock consumed, success returns4, releases destination incoming once and retires burned ID. Destination owner cannot seize/cancel. Reject leaves last_stall/position/counters unchanged at command boundary.
- [ ] Validate header+three route pages, child ownership, cursor cancellation at every edge/page and exact one arrival/removal. Active route blocks deliberate dismantling. Derived ETA is absent when stalled and a projection when unblocked.

```zig
test "RC10 freight seam capture return and reservation custody" {
    try support.checkCase("freight-seam-three-edges");
    try support.checkCase("freight-capture-stall-reclaim");
    try support.checkCase("freight-source-capture-cancel-capacity");
    try support.checkCase("freight-transfer-versus-mint");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC10 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC10 '`. Review aggregate owned goods independently of incoming reservations and verify no custody transfer on destination capture.

## Task 11: Apply frozen structural effects, occupation and repair

**Files:** Create `RC/combat_bridge.zig`, `combat_bridge_tests.zig`; modify `RC/commands.zig` for tags15/16. Tactical owner may modify its adapter only in its separately assigned sequential slot.

**Interfaces:** Produces `stepProjection`, `stepEffects`, `ProjectionRow`, `EffectBatch`, `EffectOutcome` exactly above. Consumes tactical-produced batch bound to sourceRevision/targetTick/identity and emits outcomes matched by ObjectiveId before shared candidate completion. Repairs run in public commands before combat.

- [ ] Project every activated completed nonroad structure including disabled/disconnected/ruined targets; all cells passable, cover=intact except farms/solar, capturable=intact hall. Roads/projects/nonactivated buildings never appear. Minimal all-building projection must fail road exclusion.
- [ ] Validate the entire batch before applying any write:<=4096distinct damage targets,<=2048occupations, contiguous ordinals1..N, frozen references, damage1..1000, aggregate once/target, one occupation/city with expected owner/distinct present new owner/ObjectiveId. Verify sorted damage then occupation. One invalid road effect alongside a valid target aborts the entire candidate without applying the valid damage.
- [ ] Apply damage clamp0; destroyed warehouse retains stock600/incoming64 despite functional512. Reserved arrival yields664/0. Destroyed residence does not remove people. Apply successful occupation only if hall survives; otherwise blocked_destroyed_hall retains owner and resets matching tactical attempt. Tactical-only lost_control never enters a city effect batch.
- [ ] Occupation transfers city/buildings/roads/stocks/pending training ownership, retains IDs/people/cargo launch owner, stops worker operations that hour and resets growth. Existing tactical reserves/live units retain original owner. At24people intact solar produces24, domestic3,unused21; needs consume24/24. Ruined hall plus one residence leaves housing16 and eight overcrowded people, adding2debt.
- [ ] Paid repair restoresmin(250,missing) for2timber/2stone; author repair restores1000 and receipts ceil(restored/250)*cost waived. Reject road/project/full-integrity/wrong owner. Fresh capacity after accepted repair must allow a later same-hour refund when eligible. Repair does not guarantee survival in that hour's combat.

```zig
test "RC11 effects validate whole batch and preserve involuntary custody" {
    try support.checkCase("road-effect-aborts-entire-batch");
    try support.checkCase("warehouse-destroy-arrive-repair");
    try support.checkCase("capture-solar-household-hour");
    try support.checkCase("destroyed-hall-objective-pairing");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC11 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC11 '`. Require separate adapter review by the tactical owner and fresh independent review; fake post-publication callbacks cannot satisfy the test.

## Task 12: Transfer paid training crew into reserved tactical units

**Files:** Create `RC/training.zig`, `training_tests.zig`; modify `RC/commands.zig` tags13/14 and `RC/operations.zig` training tier; composed/tactical grant adapters only under named shared ownership.

**Interfaces:** Produces `pub fn stepTraining(e:*Engine,budget:*WorkBudget) Failure!Step` and `DeploymentGrant`. Consumes paired qualifiedTrainingId/globalUnitId roster reservation and current city owner. Tactical consumes one grant per qualified producer, creates exactly one full-HP reserve and retires training in the same candidate.

- [ ] Pin three prices/crew/durations: infantry food8/tools2,crew1,duration2; armor food8/alloy8/tools4,crew2,duration4; artillery food8/alloy6/tools6,crew2,duration3. Require owned intact activated hall, one resident remaining, eligible target+duration<=horizon and all job/roster quotas before spending. Creator provenance cannot recruit.
- [ ] Infantry at capital tick1 yieldspopulation23,workers11; waterworks2/farm4/training2/forestry2,quarry idle, vector `[193,265,102,96,24,24,14,8]`,remaining1,unusedpower17. Tick2 produces `[202,274,108,96,24,24,14,8]`,reserveUnit1/crew1/eligible3 and no job. A retained-trainee-workforce implementation must fail quarry/stone assertions.
- [ ] Training comes after construction and before industry, sortedTrainingId. Each staffed hour takes2workers and subtracts1; first idle reason hall→occupation→workers→horizon. Capture pauses jobs and completion uses new current owner; existing reserves remain unchanged. At final horizon, would-complete job idles because eligibleT+1 is illegal, while cancellation remains available.
- [ ] Cancel returns all crew and floor(each paid good/2) regardless of progress; population+crew<=512, every refund fits current functional/hard capacity, housing may be exceeded. Alternate capital tick2 cancellation yields `[205,273,108,102,26,24,15,8]`,population24,nextUnit2,nojob/unit. Damaged storage rejects refund until repair; no lost reservation or crew.
- [ ] Validate producer includes world; equal localTrainingId on two worlds yields distinct grants. Duplicate/missing/mismatched class/crew/hall/owner/reserved ID is invariant abort, never capacity rejection. Unit death later emits sole full-crew tactical sink; partialHP damage retains crew. Tick3 ordinary deployment adds no goods/population cost and yields capital `[211,283,114,102,26,24,14,8]` in the no-hostility fixture.

```zig
test "RC12 recruitment staffing grants cancel and crew conservation" {
    try support.checkCase("infantry-two-hour-reserve");
    try support.checkCase("infantry-cancel-after-one-hour");
    try support.checkCase("training-all-classes-and-horizon");
    try support.checkCase("qualified-grant-duplicate-abort");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC12 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC12 '`. Review the custody equation residents+training+live/reserve crew against exact scenario/mint/growth/death/casualty sources and sinks.

## Task 13: Integrate all worlds under one budget and publication barrier

**Files:** Create/modify `CC/job.zig`, `root.zig`, `types.zig`, `bridge.zig`, `RC/invariants.zig`, `integration_tests.zig`; this task's sole writer owns all shared mutations. Consume completed tactical engine and the accepted full-scale provider/storage engine, not local stand-ins.

**Interfaces:** Completes `Authority.beginTick/poll/cancel` and invokes city step functions with the SAME `*WorkBudget`. Tactical owner supplies its ratified phase cursor; provider work and cleanup use the accepted storage contract. Produces the new composed identity's one atomic descriptor; no old identity gains these fields/phases.

- [ ] Write an all-world trace test with one visible and several unselected/empty worlds; every phase visits every world. For rounds0..3, phase3 finishes all worlds before4, through12, then next round. A per-world complete-round shortcut must fail trace equality.
- [ ] Implement phase0 input/reservations,1commands,2copied city/hostility projections,3..12four tactical rounds,13validated effects/objective outcomes,14freight,15infrastructure,16operations/training,17needs,18invariants/codec/hash/commit prep,19prepared,20publish,21retire. One u32 receipt ordinal runs across the whole tick. No extra strategic phase or city-owned clock.
- [ ] Validate custody, stock+incoming<=4096, exactly one live shipment per incoming amount, claims/footprints bijection, current stalled-route legitimacy, training/roster paired reservation, population/work/power bounds, activation/defaults/IDs/counters, receipt emitter counts and owner-consistent projections. Arithmetic/encoding/storage invariant defect aborts entire tick; ordinary shortage follows command/idle semantics.
- [ ] At prepared barrier committed views remain old; budget0 cannot publish; one charged unit installs the descriptor with already encoded commit and complete G/S/H references. No hashing, variable copy, validation or history allocation occurs during publication. After publication cancellation returns already_published and retirement drains under allowance; before publication cancellation discards candidate resources/claims/IDs/history and keeps committed sequence/policy.
- [ ] Hold old view leases through publication/retirement; prevent reusing leased storage and delay next tick when ownership requires. Inject abort/cancel at every phase, index-hole/pending-row/child transition, immediately before/after publication; verify free-once ownership and no half city/world. Corrupt cleanup enters faulted admission with bounded destruction, never economic repair by cargo deletion.

```zig
test "RC13 one global phase barrier and atomic descriptor" {
    try support.checkCase("all-world-four-round-phase-trace");
    try support.checkCase("infantry-three-hour-deploy");
    try support.checkCase("prepared-zero-budget-no-publication");
    try support.checkCase("cancel-every-phase-and-transition");
    try support.checkCase("old-view-lease-retirement");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC13 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC13 '`. Then run the entire isolated suite in both modes. Independent review must cover storage ownership and tactical ordering, not only city arithmetic.

## Task 14: Expose copied city, building, freight, training and receipt queries

**Files:** Create `RC/view.zig`, `view_tests.zig`; modify `CC/root.zig` query adapters under the same assigned writer.

**Interfaces:** Implements `beginQuery/pollQuery` above. Produces these value results; all bounded arrays are copied, and summaries are derived from exact receipts for the selected descriptor rather than extra canonical fields:

```zig
pub const CityView = struct { id: CityId, record: CityRecord,
    capacity: u32, housing: u16, over_capacity: [8]bool,
    last_hour: ?CityHourReceipt };
pub const BuildingView = struct { id: BuildingId, record: BuildingRecord,
    connected: bool, refundable: Goods, last_hour: ?BuildingHourReceipt };
pub const ShipmentView = struct { id: ShipmentId, record: ShipmentRecord,
    source_owner: CivilizationId, destination_owner: CivilizationId,
    projected_eta: ?Tick };
pub const TrainingView = struct { id: TrainingId, record: TrainingRecord,
    owner: CivilizationId, last_hour: ?TrainingHourReceipt };
pub const QuotaView = struct { global_used: [7]u32, global_limits: [7]u32,
    world_used: [7]u32, world_limits: [7]u32, roster_used: u32,
    roster_limit: u32 };
pub const ClaimPage = struct { rows: [16]ClaimRecord, cells: [16]CellId,
    count: u8, next: ?u64 };
```

`CityHourReceipt`, `BuildingHourReceipt`, `TrainingHourReceipt` are exact payload structs of tags8,9,32 from Task2. Quota arrays use city/building/claim/shipment/training/unit/objective order; roster includes training reservations separately. Queries report caller-visible constraints without making cap sums simultaneous promises.

- [ ] Start a query against an old lease, publish a newer tick, finish query and compare to the old exact record/receipt snapshot. A latest-root getter must fail this test. Reject cross-owner/expired/released/exhausted tokens without touching newer query/job controls.
- [ ] Meter sparse lookups, record/path copy, receipt scan and claims pagination; allowance0 changes no cursor/result bytes. Enforce admitted query lane bounds, bounded controls and release/cancel behavior selected in Task0. Immutable values may be copied after lease closure; borrowed page pointers never escape.
- [ ] Show per-commodity stock/incoming/functional capacity/over_capacity, population/debt/streaks, housing and last-hour work/power/care. Show priority, enable/work/paid/refund/connection/activation and actual first idle reason. Show freight launch/current owners,position,custody,stall reason/reservation; no guaranteed ETA when stalled. Show job crew/reservedUnit/remaining/horizon idle.
- [ ] Test zero-population ruined city, never-ticked genesis without hourly receipt,512population, old/missing record, full16-row page with next cursor, omitted17th row and separate provenance of explicit mint versus waived cost.

```zig
test "RC14 immutable copied explanations and metered queries" {
    try support.checkCase("old-view-query-during-publication");
    try support.checkCase("inspect-damaged-storage-and-stalled-freight");
    try support.checkCase("query-pagination-and-token-failure");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC14 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC14 '`. Full query results are presentation evidence only; they do not authorize mutation or change canonical hashes.

## Task 15: Authenticate history, exact event bounds and fresh replay

**Files:** Create/modify `CC/history.zig`, `codec.zig`, `replay.zig`, `RC/history_tests.zig`, `replay_tests.zig`; provider and session files remain with their named owner.

**Interfaces:** Consumes accepted `Prefix`/retention/provider request types and the exact new composed codec. Produces the approved provider-compatible city transcript and a fresh replay target authority. Proposed city facade: `pub fn beginReplay(self:*Authority, source:ReplaySource) Failure!JobToken`, where `ReplaySource` is the immutable authenticated retained-prefix capability from Task0's selected provider, not raw caller-authorized receipts. Its exact capability type must already be pinned there.

- [ ] Replay independent genesis+complete story from a fresh authority using public commands reconstructed from authenticated input events and recompute every outcome/state/hash. A checksum-only replay that accepts changed debit/credit or creator-mint input must fail. Compare initial G, each S/H and final descriptor/sequence/policy.
- [ ] Encode keys numerically `(realm,kind,local)`, optionals/tags/LE scalars exactly; logical state excludes history head, scheduling and page IDs. Preserve NYON-RC1 logical domains, NYON-FS1 transport-tree domains and NYON-ES1 node digests. Reject wrong identity, unknown tactical payload, trailing bytes, noncanonical capital defaults, foreign references/counter regressions and policy mismatch before installing a replay target.
- [ ] Prove each emit site matches exact framed size/multiplicity:19-byte receipt header+4length; one building status,one city summary,at mostone population change,one old shipment event,one training/grant,one input/outcome per effect/command. City outcomes are emitted only by city; tactical raw events/effect inputs only by tactical. Scenario_stock is sole initial goods source; linked minted-cargo command input is sole cargo mint.
- [ ] Enforce city-only6,258,688; city+integration7,962,624; combined12,746,752-byte tick envelope with tactical4,784,128 and shared inputs/outcomes counted once. Validate hostility frame within accepted header slack. Verify262,144genesis/65,536initial stream independently. Construct maximum-size syntactically legal emitters plus the exact independently proven reachable joint workloads pinned by Task1; label synthetic envelope, admission-peak and committed-state evidence separately.
- [ ] Exercise history backpressure before candidate mutation, provider reject/failure, full/partial1MiB segment boundaries,32MiB logical transaction admission, prefix authentication and pin retention, cancellation during source/target copy/hash and replay failure. Old session/authority remains usable after failed fresh replay. No provider callback runs inside authority poll, and a scheduled retry cannot expose candidate bytes as committed history.

```zig
test "RC15 exact transcript replay and retained prefix ownership" {
    try support.checkCase("ruler-journey-fresh-replay");
    try support.checkCase("replay-minted-cargo-custody-tamper");
    try support.checkCase("max-event-envelope-and-sole-emitters");
    try support.checkCase("history-backpressure-and-partial-segment");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC15 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC15 '`. Run independent `rich-city-reference.zig --check` again only if fixtures or encoder expectations changed; a new expected digest needs independent review, never a golden update to match current game output.

## Task 16: Run creator, ruler and combined public gameplay journeys

**Files:** Create `zig/src/rich_city_cli.zig`, `zig/tools/check-rich-city-cli.zig`, `RC/journey_tests.zig`; modify `zig/src/main.zig`, `zig/src/root.zig`, `zig/build.zig` serially. Test inputs remain frozen `FX/journeys/` files; do not regenerate them from implementation.

**Interfaces:** CLI consumes typed public genesis/tick/query/replay facade only; produces deterministic inspection/receipt output and nonzero typed failure exits. Proposed CLI entry `pub fn run(allocator:std.mem.Allocator,args:[]const []const u8) !void` in `rich_city_cli.zig`; caller owns host allocation outside poll. Add build target `check-rich-city-cli` which supplies built executable path to the Zig/std checker, matching the existing planet CLI pattern.

- [ ] Add checker commands `city new --scenario capital --role ruler --manifest <fixture>`, `city run --script <fixture>`, `city inspect --script <fixture>` as deterministic fixture harness entry points; real interactive host command serialization uses the same APIs. Before implementing the entry, a compiling command dispatcher returns explicit unsupported-command failure; the test expects successful two-hour capital and fails behaviorally.
- [ ] Execute the exact Task1 frozen ruler schedule from trusted public genesis through two-hour baseline, paid growth/industry chain, pause/scarcity, eight Apothecary hours,24successful care hours, paid infantry deployment, sustainable satellite food support and intentional/recoverable shortages. Assert every hour table and final G/S/H; inspect endowment, paid/waived/refunded goods, idle reasons, cargo/crew and projected ETA. Never hand-create a road, unit or medicine stock in this journey.
- [ ] Execute creator-empty with explicit author-city/building/mint/repair bypass and failed wrong-provenance actions; assert zero authored salvage and no hidden scenario origin. Execute combined-capital with mixed paid ruler and explicit creator envelopes and typed wrong-owner/actor rejections. Include alternative cancel-training and captured freight branches with full custody comparisons.
- [ ] Preview setup shows exact scenario/role/civilization/endowment/rules; staged/paused previews do not mutate authority. Show Spaceport external transfer unavailable with reason Strategic transport rules unavailable. A CLI successful story does not imply a native renderer, browser controls or accessible UI.
- [ ] Build the checker into the new gate. Capture command,exit,stdout/stderr artifact,case counts and frozen fixture hashes; missing input or a skipped story fails the checker.

```zig
test "RC16 exact public creator ruler combined journey" {
    try support.checkCase("journey-ruler-capital");
    try support.checkCase("journey-creator-empty");
    try support.checkCase("journey-combined-capital");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC16 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC16 '`; from `zig/` run `"$RC_ZIG" build check-rich-city-cli -Doptimize=ReleaseSafe`. Verify full journey timestamps/cells match the accepted artifacts, not a shorter replacement scenario.

## Task 17: Qualify legal quota extrema, joint workloads and bounded ownership

**Files:** Create `RC/quota_tests.zig`, `budget_tests.zig`, `ownership_tests.zig`, `zig/tools/rich-city-qualify.zig`; modify only test instrumentation hooks that compile out of production and approved accounting interfaces.

**Interfaces:** Consumes the complete public facade, accepted allocator ledger/provider instrumentation and Task1's exact independently reviewed `FX/custody/quota-workloads.json` inputs/hashes. Produces a machine-readable report from `rich-city-qualify` modes `--quotas`, `--ownership`, `--allocations`; records counters per phase/cursor and exact requested/resident memory separately. No cap-reduction configuration is added.

- [ ] RED inject a deliberate quota off-by-one in test-only candidate input/expected assertion and observe the correct rejection fail; then remove the deliberate defect. Run Task1's independently proven legal extrema and next-admission/one-over cases for each global/world/city limit, requiring the pinned first rejecting guard. Separately exercise each unchanged configured quota at boundary and boundary+1 in the narrow quota-predicate/storage test when no reachable public case proves that boundary; label that result synthetic and do not claim a public quota rejection if an earlier topology/resource guard wins. Run the exact pinned reachable joint workloads and compare their actual entity counts, legal routes and phase/state hashes; do not recreate the rejected impossible tuple or invent a replacement maximum. Roads count as buildings and claims; farms have two exclusive footprint claims; halls have one; training owns its one roster-reservation record.
- [ ] Report quota occupancy at each pinned phase: distinguish admission/pre-freight peak from committed live cargo after arrivals/retirement, and match destination reservations to surviving shipment custody at each boundary. Validate each route from the frozen coordinates and completed owned hall/road cells; a quota count alone never establishes freight feasibility. Any mismatch from Task1's independent witness is a failed test, not permission to rewrite its schedule or expected counts.
- [ ] Separately recheck83,216row/95,504page conservative storage envelope with actual maximum encoded records/routes in the explicitly synthetic allocator/codec stress. It is not a reachable city simulation state and must not run as an accepted gameplay tick or qualify the loaded-tick latency target. Materialize maximally distinct old/new synthetic roots, pinned old leases, one in-progress detached transition, source plus replay target, retained provider buffers and query controls under storage ownership invariants; run public cancellation/retirement paths separately on the pinned reachable workloads. Check free-once child ownership for every relevant path; all live page/row bounds include pending rows/linked children and exclude holes as prescribed by the accepted engine.
- [ ] Run each representative scenario with allowances0,1,7,256,4096 and independently seeded randomized schedules. At0 all candidate/control/history/ownership state is unchanged. At each nonzero call used<=allowance; canonical bytes/events/hashes and final public results match across schedules. Invalid allowance4097 rejects without moving the job. Duplicate/stale/wrapped/exhausted tokens cannot mutate a newer lane.
- [ ] Enumerate interrupted cuts for request copy,sort,lookup,index movement,COW child linkage,graph traversal,command preflight,training grant/effect pairing,output encoding/hash,prepared publication,rollback and retirement. Inject OOM only at setup, provider failures at accepted request boundaries, arithmetic and cleanup corruption at tagged fault sites; poll performs0 allocator calls. Failure preserves committed revision/policy/sequence or enters the prescribed faulted state after publication.
- [ ] Include graph-work upper-bound counters proving once/world traversal and metered membership/route checks. Do not replace linear sorted-index movement with an unreviewed cache or skip empty/unselected worlds. An unmet workload bound is a design/implementation finding, not permission for partial ticks.

```zig
test "RC17 quota edges polling schedules and free once ownership" {
    try support.checkCase("legal-quota-extrema-and-next-admission");
    try support.checkCase("configured-quota-boundaries-synthetic");
    try support.checkCase("pinned-joint-workloads-phase-counts");
    try support.checkCase("budget-equivalence-and-zero-noop");
    try support.checkCase("all-cow-and-grant-cancel-cuts");
    try support.checkCase("poll-allocation-count-zero");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC17 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC17 '`, then `"$RC_ZIG" run -O ReleaseSafe zig/tools/rich-city-qualify.zig -- --quotas`, separately `--ownership` and `--allocations`. A source arithmetic assertion is not a measured resident-memory result; reports must distinguish both.

## Task 18: Gate loaded performance, retention, session Continue and delivery limits

**Files:** Extend `zig/tools/rich-city-qualify.zig`; create `RC/session_tests.zig`, `zig/src/rich_city_wasm.zig`; modify `CC/session_bridge.zig`, `zig/build.zig`, `zig/tools/check.sh` only under explicit shared/session ownership. Create new `docs/qualification/2026-10-11-rich-city-final.md` and raw evidence directory. Native/browser runtime implementation stays with its delivery owners.

**Interfaces:** Consumes completed independently accepted durable-session/provider implementation and registered composed profile, emits tested save/reopen/Continue behavior for a fresh fixture-owned session. Qualification modes `--loaded-ticks`, `--retention`, `--session`, `--poll-latency` use the exact independently reviewed legal workload schedules/hashes pinned in Task1 and output hardware/compiler/fixture/profile identities and percentile methodology.

- [ ] Add session tests that save a new city session, terminate the fixture host, reopen and Continue the exact selected saved descriptor. Test wrong profile/rules/G/prefix, corrupt/truncated/partial tail, interrupted save/GC/retention/cancellation, latest/catalog mismatch and failed target replay. Failures preserve the current resident session and exact previous durable selection. Never open old user-profile data. A stub returning latest empty state must fail final G/S/H and stock/custody comparison.
- [ ] Qualify100,000retained ticks with the declared legal bounded workload and authenticated fresh replay; independently run maximum-load byte stress. Report total byte growth, segment/node boundaries, retained prefix ownership, source/target buffers, cleanup/GC and exact reopen evidence. Do not call the small story or a transient in-memory provider a durability pass.
- [ ] On named hardware, run10,000full loaded ticks for each Task1 accepted legal loaded-workload schedule with every declared quota unchanged; requirep95<=5ms for complete tick work, separately reporting request copy,simulation,validation,encoding/hash and retirement. Bind every sample to its fixture hash and actual phase/committed entity counts, including cargo arrivals and any prescribed replenishment commands. Independently accepted legal workload coverage must include the applicable quota extrema and interacting city/road/freight/training/tactical work; an arbitrary sparse case or synthetic all-maxima storage tuple cannot satisfy this gate. Maximum storage/encoding-envelope stress remains a separately labeled mandatory result, not a gameplay latency sample. Record sample method and whether the complete end-to-end sum includes provider waits; never omit them without an explicit separately reported measurement. At256units require pollp95<=2ms/max<=8ms, and0poll allocations. Those are distinct measurements; faster polls cannot excuse failed loaded-tick p95. Include linear index mutation and tactical BFS worst workloads.
- [ ] If feasibility fails, preserve raw results and return to the controller/design owner with the actual limiting operation. No reduced caps, skipped worlds, partial publication, selective samples or renamed poll metric can close the gate. Improvement requires its own reviewed bounded change, then repeat affected workloads.
- [ ] Add isolated build targets `test-rich-city`, `check-rich-city-cli`, `check-rich-city-wasm`; compile `rich_city_wasm.zig` freestanding wasm32 API/codec probe. Compile success remains distinct from real browser allocator/persistence/input/rendering behavior. Add city tests/CLI/probe to the existing Rust-free `zig/tools/check.sh`; preserve existing planet/classic gates and compiler pin.
- [ ] Run the complete gate after final changes, read the full owned diff, then obtain fresh independent spec and code/ownership reviews of exact source/fixture/report pins. Complete only when required tests and reported acceptance criteria pass. Record native graphics/browser runtime/accessibility/gamewide strategic work as separate incomplete delivery obligations if their owners have not qualified them; do not label the full game complete from this city plan.

```zig
test "RC18 exact selected city session continues after restart" {
    try support.checkCase("session-save-restart-selected-continue");
    try support.checkCase("session-partial-tail-preserves-resident");
    try support.checkCase("session-wrong-profile-rejected");
}
```

Run `"$RC_ZIG" test zig/src/rich_city_tests.zig -O Debug --test-filter 'RC18 '` and `"$RC_ZIG" test zig/src/rich_city_tests.zig -O ReleaseSafe --test-filter 'RC18 '`. From repository root:

```sh
"$RC_ZIG" run -O ReleaseSafe zig/tools/rich-city-qualify.zig -- --loaded-ticks
"$RC_ZIG" run -O ReleaseSafe zig/tools/rich-city-qualify.zig -- --poll-latency
"$RC_ZIG" run -O ReleaseSafe zig/tools/rich-city-qualify.zig -- --retention
"$RC_ZIG" run -O ReleaseSafe zig/tools/rich-city-qualify.zig -- --session
PATH="/Users/donaldfilimon/.zvm/bin:$PATH" ./zig/tools/check.sh
```

The required repository gate is the implemented Zig migration gate because the approved rewrite forbids Rust in its required pipeline. The historical Rust AGENTS commands describe preserved old source and are not evidence for new Zig city behavior. No Rust retirement is included in these city tasks.

## Review and handoff checklist

- [ ] Task0 exact E/T/R/authority/session/hostility/profile acceptance is complete and pinned; no superseded consumer snapshot remains.
- [ ] Task1 independent full schedule, exact qualified cells, command sequence, hourly table, G/S/H, legal quota extrema/joint-workload schedules with phase-count labels, and layout witness are accepted and added to this plan before product work.
- [ ] Each task has one writer; shared files are handed off explicitly; fresh independent spec and code-quality reviews refer to exact diff and test evidence.
- [ ] Every behavior test records an assertion-level RED, scoped Debug/ReleaseSafe GREEN and accumulated suite result. No missing import, mocked provider, empty filtered run or skipped scenario is counted as acceptance.
- [ ] Source/fixture changes stay inside named ownership and preserve prior uncommitted work. No automatic commit step from a generic skill overrides the explicit no-commit instruction.
- [ ] Final claims distinguish implemented semantics, compile/layout/static byte proof, unit/integration tests, public CLI journey, provider/session restart, loaded performance, native/browser runtime and complete-game delivery.

Spec coverage map: R units/limits/records→Tasks0–2,17; topology/infrastructure→4,8,10; costs/recipes/labor/power→5,6,8; needs/health/growth→7; freight/founding/resettlement→9,10; structure/capture/repair→11; training/custody→12; trusted capital→1,3; roles/receipts/codecs→2,5,15; phases/atomic publication→13; player explanations→14,16; complete journey→1,16; allocation/ownership/quotas→1,13,17; history/durability/performance→15,18. Strategic imports/exports remain unadmitted; spaceport behavior is explicitly tested in6/16.

Self-review at plan creation: task count19 including Task0; local kernel tests are explicit and complete public/tactical journeys wait for Task13; all product entry points depend on exact contract and witness gates; new strong ID types and codec families are distinct; worker/grant/effect ownership and next-hour activation agree with R; no implementation or test success is asserted. The unresolved full-story hashes, bridge signatures/layout and integrated consumer pins are named hard deliverables before source dispatch, not silent assumptions. This plan itself awaits independent review and delegated controller acceptance.

Narrow workload correction after [plan review](../../qualification/2026-10-11-rich-city-plan-review.md): Task1 now owns independently proven legal extrema and joint workload schedules; Tasks15/17/18 consume those exact witnesses. Configured quotas, conservative storage envelopes and the full loaded-tickp95<=5ms criterion remain unchanged. Admission peaks, committed counts and synthetic allocator/codec stress are explicitly distinct. This correction is a proposed response to the review's major finding and requires scoped independent rereview; the original review is preserved and no passing workload or performance result is claimed.
