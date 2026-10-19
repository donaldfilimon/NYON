# NYON Coordinated Galaxy Implementation Plan

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Canceled on 2026-10-19:** Donald canceled further Zig rewrite work. Unfinished Zig tasks and continuation instructions in this document are superseded; retain its historical evidence and existing source. See [the current implementation decision](../../../REWRITE-DECISION-2026-10-19.md).
<!-- END 2026-10-19 rewrite decision -->

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the approved coordinated multi-planet authority, complete canonical replay and two-world CLI transaction journey.

**Architecture:** Independently allocated planet authorities prepare under one coordinator-owned lane and caller allowance. A retained global descriptor exposes complete old snapshots during hidden installation; one descriptor switch publishes all worlds. The existing island stays independently qualified under its unchanged identity.

**Tech Stack:** Zig exactly `0.18.0-dev.146+35accc06e`, Zig/std SHA-256, existing shell gate; no production dependency or Rust.

**Spec:** [2026-10-11-nyon-zig-coordinated-galaxy-design.md](../specs/2026-10-11-nyon-zig-coordinated-galaxy-design.md), written rules approved by Donald on 2026-10-11 at 07:05 EDT. Donald authorized continuation and improvement on2026-10-11 at07:37EDT after this plan was presented for review; execute the preserved subagent-driven workflow. Execution method remains the previously selected subagent-driven development.

## Global Constraints

- Canonical `/Users/donaldfilimon/dev/active/NYON`, existing main; preserve all dirty work/instruction files. No commits, pushes, worktrees, branches, dependency acquisition, deployment or old-data access.
- Identity `nyon.coordinated-galaxy.v0`; never advertise integrated receipts as `nyon.planet-economy.v0`. Existing island public semantics, vectors, compiler and gate remain unchanged except additive integration and behavior-preserving extraction.
- Systems/planets/civilizations each 1..4, contiguous IDs; each system has a planet. Default/demo two planets, two civilizations, one system. Side4..32/default8/demo4.
- Each planet: settlements1..64, buildings1..256, freight1..64, history1..4096; inherited32building/settlement, population/housing/resource/local-route limits remain normative.
- Global history1..4096/default4096, retained slots2048bytes and staging512slots. Commands0..256, at most64/planet and1024body bytes.
- One hour/tick, target exactly completed+1 and≤1,000,000; every world executes every phase. Global sequence ordering and watermark; optional genesis policy null.
- Planet reservation8,642,560bytes each; coordinator exactly12,582,912bytes; maximum47,153,152bytes below64MiB. Replay second owner explicitly totals94,306,304bytes maximum; allocator/OS overhead is not included.
- One owner/thread, no callbacks/reentrancy, one lane through cleanup, two roots/descriptors,64read leases, no borrowed page pointers or exposed mutable planet handles.
- Shared allowance0..4096/default256; ≤256byte copy/zero and≤64byte hash input/unit; no poll allocation or per-planet fresh allowance. Sole beginTick copy exception≤263,168bytes including routing IDs.
- New strategic mechanics, durable storage and all graphical platforms remain mandatory later work. This plan delivers neither a complete expanded game nor Rust repository retirement.

## Review Focus

1. A host keeps an old view across publication: both worlds/history stay old, new reads stay new; release permits cleanup (Tasks4/6).
2. Cancellation occurs after one hidden installation: all committed values/heads/policy/sequences remain unchanged and retry succeeds (Tasks6/9).
3. A cleanup refcount defect makes ordinary retirement undrainable: explicit fault shutdown frees allocations once after leases release (Task7).
4. An empty tick follows maximum sequence or a policy transition: every world advances with exact global/local watermark and policy bytes (Tasks3/5/8).
5. Creation fails after some independent allocations: every owned reservation drains/frees without destroying existing authority (Tasks2/7/9).

## File structure and review ownership

New `zig/src/coordinated_galaxy/`: `ids.zig`, `types.zig`, `state.zig` (reservation/descriptors/ownership), `planet.zig` (private staged adapter), `command.zig`, `view.zig`, `history.zig`, `job.zig`, `replay.zig`, `stories.zig`, `root.zig`, corresponding focused test files and `wire_vectors.zig`. New aggregate `zig/src/coordinated_galaxy_tests.zig`; CLI `zig/src/coordinated_galaxy_cli.zig`, tests and Wasm probe; native checker `zig/tools/check-coordinated-cli.zig`.

Only designated tasks touch existing island files. Extract algorithm context seams where required; do not copy entire island implementation into another authority. Shared algorithm functions receive explicit integrated context (planet identity, present owners, global watermark, root handle and codec identity) while island wrappers supply the exact old context. Root/state layout and staged lifecycle belong to separate integrated adapter. Any extraction must preserve independent v0 vectors and original regression tests. Never turn direct island `Authority.poll` into the coordinator driver.

Sequential implementations; each task receives fresh implementer and fresh spec/quality reviewer, exact owned files, approved spec and predecessor Interfaces. Workers are not alone and must not revert other changes or spawn workers. Shared root/test aggregation edits are sequential. Record RED/GREEN commands, actual exits and exact uncommitted snapshots in `.superpowers/sdd/2026-10-11-nyon-zig-coordinated-galaxy/`; no commit-based review or unauthorized commits. One final integrated gate and whole-slice review after all task reviews; preserve evidence and residuals.

## Shared interfaces

Task1 owns exact declarations used below, including Receipt, ReceiptValue/LocalReceiptValue, ReceiptPage, Summary, Stats, CreateProgress, ReplayProgress and StoryProgress shape contracts. state.Storage is the coordinator storage implementation owned by Task2; later task controls remain inside its pre-reserved blocks. All referenced types live in `coordinated_galaxy/types.zig` unless named otherwise. `WorkBudget` is the island's same mutable remaining counter. `Step` is pending/done; `Phase` follows spec ten-phase order. Errors include spec failures and explicit Busy, WrongRevision, WrongTick, NotReady, StaleJob, StaleView, ViewCapacity, Shutdown, CleanupFault, IdExhausted, SequenceAlreadyPublished, HistoryFull, InvalidConfig, InvalidBudget, EnvelopeTooLarge and TooManyCommands; no silent conversion to gameplay rejection.

- Config: `seed:u64`, `system_count:u8`, `civilization_count:u8`, `planet_count:u8`, `global_history_capacity:u16=4096`, `planets:[4]PlanetConfig`; only first planet_count entries are initialized/read. PlanetConfig contains system ID, side and four local capacities; planet ID is its ascending slot+1.
- RoutedEnvelope: `planet:PlanetId`, `body:[]const u8`; EnvelopeHeader retains sequence/provenance/optional actor. Public entity refs are `{planet,local}` typed by entity kind.
- PreparedRoot: opaque planet/root slot/local tick/revision/history count/head; outside private adapter never mutable. Descriptor: global tick/revision/sequence/optional policy/head/count plus fixed initialized planet entries.
- ViewToken: descriptor slot/generation and lease slot/generation; value APIs copy output. Progress has pending(stats), published(summary), waiting_for_views, canceled, failed(Failure), faulted(job,cause), invalid_request(Failure), idle. CancelResult includes requested/already_published/stale_job/faulted.
- Planet operations: `beginPrepared(*PlanetAuthority, Revision, Tick, SessionPolicy, []const u16, CommandSequence) Failure!void`; `advancePhase(*PlanetAuthority, Phase, *WorkBudget) Failure!Step`; `freezePrepared(*PlanetAuthority,*WorkBudget) Failure!?PreparedRoot`; `installHidden(*PlanetAuthority,*WorkBudget) Failure!Step`; `restoreHidden(*PlanetAuthority,*WorkBudget) Failure!Step`; `retire(*PlanetAuthority,RootHandle,*WorkBudget) Failure!Step`.
- Public job: `beginCreate(Allocator,Config) Failure!Builder`; `pollCreate(*Builder,*WorkBudget) CreateProgress`; `cancelCreate(*Builder) void`; `takeReady(*Builder) Failure!Galaxy`; `deinitBuilder(*Builder) Failure!void`; `beginTick(*Galaxy,Revision,Tick,SessionPolicy,[]const RoutedEnvelope) Failure!JobId`; `poll(*Galaxy,JobId,*WorkBudget) Progress`; `cancel(*Galaxy,JobId) CancelResult`; `deinit(*Galaxy) Failure!void`.
- Queries: `acquireView(*Galaxy) Failure!ViewToken`; `releaseView(*Galaxy,ViewToken) Failure!void`; `summary(*const Galaxy,ViewToken) Failure!Summary`; `settlement(*const Galaxy,ViewToken,SettlementRef) Failure!SettlementValue`; `cell(*const Galaxy,ViewToken,CellId) Failure!CellValue`; `receipts(*const Galaxy,ViewToken,u16,[]ReceiptValue) Failure!ReceiptPage`; `planetReceipts(*const Galaxy,ViewToken,PlanetId,u16,[]LocalReceiptValue) Failure!ReceiptPage`. Output slices≤64values, exact copied lengths, no partial output on stale token. Root-bound private query helpers pay incremental scans when used by jobs/replay.

### Task 1: identity contracts and independent wire witnesses

**Files:** Create ids/types/root/wire_vectors/contracts_tests/wire_vectors_tests and aggregate; modify `zig/src/root.zig` only for additive test import/export.
**Interfaces:** Produces all shared declarations above plus `validateConfig(Config) Failure!void`, `planetSeed(u64,PlanetId) u64` in types; independent fixtures consumed by Tasks5/8/10. No production codec in fixture generator.

- [ ] Write this seed witness in `contracts_tests.zig` before `planetSeed` exists:

```zig
test "cg/contracts/seed-witness" {
    try std.testing.expectEqual(@as(u64, 2), types.planetSeed(2, @fromBackingInt(1)));
    try std.testing.expectEqual(@as(u64, 0x9E3779B97F4A7C17), types.planetSeed(2, @fromBackingInt(2)));
}
```

- [ ] Write `cg/contracts/limits-and-seed` asserting seed2 planet1=2, planet2=`0x9E3779B97F4A7C17`, side4 cell71 fertile; reject zero/>4counts, system-without-world, side3/33, global history0/4097. Assert missing defaults do not read unused planet entries.
- [ ] Write `cg/wire/independent-domain-and-genesis` with exact hex, byte lengths and SHA heads for one-world genesis, first empty tick, two-world creator/ruler transition and rejected command. Independently encode approved field formulas using a small reference helper confined to fixture tests; pin handwritten concatenation/hex before hashes. Do not derive goldens from implementation Encoder.
- [ ] Run `zig test zig/src/coordinated_galaxy_tests.zig --test-filter cg/contracts`; capture genuine failing assertion/missing implementation, then implement declarations/config validation/seed formula and rerun to pass. Wire encoder comparison becomes active in Task5, not falsely marked passed now.
- [ ] Check exact layout-facing type names/signatures against shared interfaces; run `zig fmt --check zig/src/coordinated_galaxy zig/src/coordinated_galaxy_tests.zig`. Freeze snapshot for review.

### Task 2: separate planet allocation and staged local simulation

**Files:** Create planet/state/planet_tests/state_tests; modify island rules/command/phases/history/invariants/view only for explicit shared context extraction and corresponding original tests.
**Interfaces:** Consumes Task1; produces PlanetAuthority and six staged operations above; `createPlanet(Allocator,PlanetConfig,PlanetId,u64,u8) Failure!PlanetBuilder`, `advancePlanetCreate(*PlanetBuilder,*WorkBudget) Failure!Step`, `takePlanet(*PlanetBuilder) Failure!PlanetAuthority`; RootHandle and explicit-root value helpers. `state.Storage` owns coordinator reservation, descriptor arrays and planet allocations; no heap allocation of job/replay/lease metadata outside the accounted reservations. Task2 defines integrated AlgorithmContext in planet.zig and explicitly passes it through shared algorithm seams; island wrappers select fixed planet1/owner7 and old codec. No shared global context or thread-local policy. State owns fixed CoordinatorReservation, allocation ledger and two descriptor storage slots.

- [ ] Write `cg/planet/staged-no-publication` preparing world2 civilization2 with farm71/east footprint71/4. Assert active old summary/explicit old-root queries unchanged after every phase, freeze returns target1/newhead, hidden install changes internal marker only, restore retains old root; no public planet mutation export.
- [ ] Write `cg/state/reservation-and-setup-unwind` asserting coordinator12,582,912bytes, four-world47,153,152; allocator failure at each allocation returns/frees all acquired reservations exactly once; initialization budget0 changes nothing. Assert independent worlds with local settlement1 have distinct qualified refs.
- [ ] Run aggregate filtered `cg/planet` and `cg/state` RED, implement staged cursors and allocation-ledger reservation. Every record/ref/topology/hash step spends existing allowance before work. Expose phase boundaries and global watermark, never call island publishing poll.
- [ ] Run filtered GREEN and original island aggregate `zig test zig/src/planet_economy_tests.zig`; require actual pass count, unchanged wire fixtures. Compile-time assert all integrated controls/records/codec buffers fit existing planet and coordinator envelopes.
- [ ] Self-review extraction diff against old behavior, fixed-owner island context and no unsafe lifetime escape; freeze exact snapshot.

### Task 3: global admission, sorting, routing and multi-owner commands

**Files:** Create command/command_tests; modify integrated planet hooks only, never change island default owner rules.
**Interfaces:** Consumes Tasks1/2. Produces `command.Cursor.init(*state.Storage,SessionPolicy,[]const RoutedEnvelope) Failure!Cursor`, `advance(*Cursor,*WorkBudget) Failure!Step`, owned per-planet lengths and global watermark for staged beginPrepared; logical payload encoder reuses shared grammar.

- [ ] Write `cg/commands/global-order-and-ownership`: scrambled sequences1..4 across planets sort globally; local subsequences retain order; duplicate/old/zero/unknownworld/>64route cause structural failure before any gameplay. Creator can author owners1..4, ruler WrongOwner consumes sequence only on publication, cross-world cell/path rejected and creator same-owner freight enforced.
- [ ] Write `cg/commands/borrowed-snapshot-and-boundaries`: overwrite caller bodies immediately after beginTick-copy helper, max256×1024bodies+route IDs copy remains exact, zero-budget sorting stable; all bounded malformed optional/tag/length cases preserve committed state.
- [ ] Write entity-exhaustion and max-global-sequence cases: entity IdExhausted rejects without resources/ID use, later command proceeds; maxsequence accepted once, then empty tick valid, later nonempty oldsequence structural failure.
- [ ] Run `zig test zig/src/coordinated_galaxy_tests.zig --test-filter cg/commands` RED; implement incremental parse/sort/route/admission with explicit rejection precedence; rerun GREEN under budgets1/7/256/4096 and alternating0.
- [ ] Freeze routed transcript and allocator/work instrumentation for review; no controller job publication yet.

### Task 4: leased descriptors and explicit-root queries

**Files:** Create view/view_tests; extend state descriptor/lease ownership.
**Interfaces:** Consumes Tasks1/2. Produces public query signatures above and `prepareDescriptor(*state.Storage,*WorkBudget) Failure!Step`, internal descriptor-only switch `publishDescriptor(*state.Storage) void`; no O(planets) loop in switch. View token validation remains bounded; free lease slots tracked explicitly instead of unbounded scans.

- [ ] Write `cg/views/old-new-and-stale`: acquire old view, install hidden new world roots, old reads both old; switch descriptor, new view reads both new while old history counts still old. 64leases succeed/65th ViewCapacity; double release/forgedgeneration returns StaleView; no interior pointer escapes.
- [ ] Write `cg/views/exhaustion-and-backpressure`: released maxgeneration slot never reused, exhausted descriptor generation fails pre-lane; release old lease permits retirement, new lease does not block old cleanup. Healthy deinit with any lease returns Busy without freeing.
- [ ] Run filtered `cg/views` RED, implement descriptor-token ledger and copied root-bound queries. Route all public reads via descriptor, never planet active marker. Queries cap64outputvalues and preserve buffers on stale validation failure.
- [ ] Rerun GREEN; explicit-root metadata/receipt history-tail bounds and value lifetime tests must pass across hidden restore and old cleanup. Freeze snapshot.

### Task 5: global/local canonical transcripts and history capacity

**Files:** Create history/history_tests; complete wire_vectors_tests comparisons. Modify integrated local codec context only.
**Interfaces:** Consumes Tasks1–3. Produces `history.Encoder.init(*const Receipt,[]u8) Encoder`, `advance(*Encoder,*WorkBudget) Failure!Step`, `len(*const Encoder) usize`; `Finalizer.init(*state.Storage,*const Descriptor) Finalizer`, `advance(*Finalizer,*WorkBudget) Failure!Step`; transcript head/count values for descriptor preparation.

- [ ] Write `cg/history/independent-vectors` comparing every tag/domain/field to Task1 hex and heads, including full rejected body, null previouspolicy, creator→ruler transition, local empty-world global watermark, maximum legal path receipts and SHA two-padding-block boundary.
- [ ] Write `cg/history/capacity-exact-fit`: one-world capacities1/3/4 admit genesis at1, first empty tick needs3additional slots and fails at3, succeeds at4 then next fails. Demand=`2+planet_count+command_count`;256commands/fourworlds262records fit512staging; local HistoryFull aborts globally with no visible tail/head/count changes.
- [ ] Run `cg/history` and `cg/wire` RED; implement exact spec tags0..4/global and integrated local domain/tag/header/payload grammar. Hash≤64bytes/unit, encode/copy≤256bytes/unit, no padding/pointer encoding. All receipt tails invisible until descriptor publication.
- [ ] Rerun GREEN for all independent witnesses and retained local/global receipt paging; preserve island independent wire suite. Freeze snapshot.

### Task 6: globally atomic job, phase barriers and bounded rollback

**Files:** Create job/job_tests; finalize root public facade. Extend state/planet private hooks as specified, sequential ownership.
**Interfaces:** Consumes Tasks1–5. Produces public create/tick/poll/cancel/deinit APIs; stats exposes phase/planet/cursor/units/requested bytes/liveleases/history. Phase stats must identify hidden installation count so the real CLI witness can cancel after world1 installation without importing private mutable handles. Next lane admission only after retirement/drain. Creation ready means all immutable worlds and local/global genesis are encoded/hashed/valid.

- [ ] Write `cg/jobs/all-world-phases`: worlds execute commands→arrivals→construction→production→population together in ascending planet order; empty/rejected world still advances, every completed local tick/revision equals global. Request expected/target errors fail before lane.
- [ ] Write `cg/jobs/partial-install-abort-and-retry`: budget1 reaches hidden world1 installation, cancel, committed old2world values/history/policy/sequence unchanged, drain frees candidates, same input retry publishes once. Cancel immediately before descriptor switch aborts; after switch already_published, cleanup stays Busy.
- [ ] Write `cg/jobs/view-lease-and-zero`: retained old lease spans publication, poll returns waiting_for_views without work; release drains old roots, then next tick allowed. Zero budget changes no cursor/flags/terminaldelivery. Stale previousjob cannot affect new lane.
- [ ] Run `cg/jobs` RED; implement one shared counter across preparation/routing/all phases/local/global finalizers/invariants/hidden install/rollback/retirement. Build all descriptor fields and check capacity before switch. One paid constant operation switches index; published once, no allocation/recoverable work needed after switch.
- [ ] Rerun GREEN budgets1/7/256/4096/alternating0, compare byte-identical heads/state; drain on failed/canceled creation and tick before destruction. Freeze review snapshot.

### Task 7: permanent cleanup fault and safe destruction

**Files:** Extend job/state/view; create fault_tests.
**Interfaces:** Consumes Tasks2/4/6. Produces `beginFaultShutdown(*Galaxy) Failure!void`, `destroyFaulted(*Galaxy) Failure!void`; faulted progress/cancel and Shutdown admission behavior.

- [ ] Write `cg/fault/undrainable-cleanup`: inject postpublication old-ref failure; repeated poll(faultedid) returns faulted without traversal/work including budget0, stalejob invalid, cancel faulted. Published leased query remains valid; beginFaultShutdown idempotently closes acquire/tick/replay; existing queries/releases legal; destruction Busy while leased then frees all independently owned reservations once without damaged-ref traversal. Ordinary deinit never silently destroys faulted lane.
- [ ] Write `cg/fault/creation-allocation-ownership` failure after each successful planet allocation and partial COW retention; healthy cancel drains, no double free. Fault ledger corruption is explicitly outside supported semantic fault model.
- [ ] Run `cg/fault` RED; implement permanent fault state/ownership-ledger release and nonpoll destruction exception exactly per approved spec; rerun GREEN with allocation accounting.
- [ ] Freeze fault lifecycle/state diff and evidence for review.

### Task 8: complete coordinated replay and state comparison

**Files:** Create replay/replay_tests; root export only additive.
**Interfaces:** Consumes Tasks1–7. Produces `replay.begin(Allocator,*Galaxy,ViewToken) Failure!Cursor`, `replay.poll(*Cursor,*WorkBudget) ReplayProgress`, `replay.cancel(*Cursor) void`, `replay.takeVerified(*Cursor) Failure!Galaxy`, `replay.deinit(*Cursor) Failure!void`. Replay owns a new galaxy and its own source lease; passed ViewToken must be valid current/retained source revision. It duplicates a source lease explicitly rather than stealing caller ownership; destruction releases only its own lease. Target construction uses exact source config/capacities/genesis. Source can publish one next tick but its leased roots cannot retire until replay drains.

- [ ] Write `cg/replay/all-inputs-and-policy`: accepted+WrongOwner+empty+creator→ruler ticks recompute every local/global receipt byte/count/head and every logical record/terrain/counter; target config same exact history limits. Pollbudgets vary without output changes; no head-only acceptance.
- [ ] Write `cg/replay/adversarial-and-lifetime`: missingcommit/alteredrole/reorderedinput/unknownworld/duplicate seq/localhead tamper fail; source oldlease stable across next global publication, cancellation releases owned target+sourcelease but leaves caller view/state. Failed target never installs into source. Maximum requested source+target94,306,304 accounting explicit.
- [ ] Run filtered `cg/replay` RED; implement incremental extraction/copy/drive/comparison/cleanup with same provided counter, no fullworld memcmp or synchronous final hash. Allzero/control generation stats excluded from canonical replay only as spec states.
- [ ] Rerun GREEN including first-nullpolicy/maxsequencefollowedempty/history-exactfit cases; freeze snapshot.

### Task 9: robustness, capacity and changed-path performance evidence

**Files:** Create robustness_tests; evidence benchmark harness in ignored plan workspace, not required product dependency.
**Interfaces:** Consumes Tasks1–8; uses public facade except clearly labeled internal fault injection.

- [ ] Write `cg/robustness/exhaustive-boundaries`: enumerate every charged creation and representative two-world nonempty tick prepublication cancellation cut including pendingretain/copy/swap and each hiddeninstall; compare complete prior descriptor/localroots/history/policy/sequences, pageownership and sameinput retry. Enumerate postpublication cleanup cuts separately; do not call one fixture exhaustive allcapacities.
- [ ] Write `cg/robustness/maximum-and-no-allocation`: four side32 worlds with maximum records/history/dirtiedpages,64leases,256commands/64each, no poll allocator calls, aligned controls/reservations, malformed inputs and every counter exhaustion branch. Test budgets0/1/4096 and invalid4097 with unchanged state.
- [ ] Run focused RED as appropriate then GREEN; fix only identified violated approved invariants within owned earlier modules through original task implementer and scoped review. Do not loosen assertions/goldens to hide defects.
- [ ] Measure≥10,000 actual native ReleaseFast budget256polls with representative and maximum-loaded fixtures; report namedCPU/OS/compiler/config/concurrency, phases, p95≤2ms/max≤8ms targets, copy latency separately, request bytes vs allocator/process overhead. An emptyfixture alone cannot qualify maximum-loaded latency. Failure remains explicit gap and drives correction, not silent target revision.
- [ ] Freeze test/evidence snapshot with exact cancellation scope, observed counts/exits and performance limitations.

### Task 10: real CLI, Wasm compilation and final integrated qualification

**Files:** Create stories/stories_tests, coordinated_galaxy_cli/cli_tests/Wasmprobe and check-coordinated-cli.zig; modify main/root/build/check.sh/README and qualification documents only additive.
**Interfaces:** Consumes all prior APIs. Produces `stories.Runner.begin(Allocator,Config) Failure!Runner`, `poll(*Runner,*WorkBudget) StoryProgress`, `acknowledge(*Runner) Failure!void`, `cancel(*Runner) void`, `deinit(*Runner) Failure!void`; StoryProgress carries copied actual observations/receipts, no hardcoded output-state substitution. Runner drives approved five-tick transaction journey and complete replay with bounded observation/drain.

- [ ] Write `cg/cli/ordered-journey`: real two-world seed2/side4 story snapshots: tick1 food14/12 material20/20; tick2 food16/16 material12/20; tick3 food18/20 material12/12 housing8/4; tick4 food20/24 housing8/8. Tick5 grants specify sequence8world1food1 and sequence9world2food1; failed partial-install attempt leaves tick4 unchanged, sameinput succeeds with food23/29, accepted2 and watermark9. Print complete real heads/receipts and qualified IDs, old tick4 view and new tick5 view, cancel-afterpublication and replay PASS. No growth at5ticks.
- [ ] Write CLI default/strictflags/budgets/malformedarg/writerfailure tests; allfailure paths release views and drain Runner jobs before deinit. Native subprocess checker compares every ordered world/tick row, abort unchanged marker and final replay outcome. No display constant masquerades as authority observation.
- [ ] Run RED focused CLI/story tests then implement main dispatch `coordinated-galaxy-demo`, retaining planet/Classic commands. Defaultseed2/side4/budget256; allow decimal seed, side4..32 and budget1..4096, reject repeated/unknown/trailing flags and overflow. Tests varyingseed/side test validcreation/error behavior; exact demonstration positional fixture is explicitly seed2/side4, reject incompatible demo inputs with explanatory error rather than inventing placements.
- [ ] Add isolated `test-coordinated` with inheritedstdio, `check-coordinated-cli` subprocess build step and `check-coordinated-wasm` freestanding ReleaseSafe object compiling create/poll/views/shutdown/replay/Runner exports using fixed-buffer ownership. Compileprobe is not browser runtime qualification.
- [ ] Extend Zig gate formatter input for new checker, nativechecker and wasmstep. Run `./zig/tools/check.sh` once on frozen integratedsource: require observed exit0, explicit Debug/ReleaseSafe counts, release, bothnewCLIandexistingchecks, new+islandwasmcompile. Run `git diff --check`; inspect untracked content via formatting/fullsnapshot as Git alone omits it.
- [ ] Whole-slice strongest available independent review reads exact owned diff and approvedspec once; one combined finalfixwave/scopedrereview for actionable findings. Record actual gate/review/parity/capacity/latency evidence and remaining fullgame/platform/save/Rustretirement gaps in qualification. Preserve evidence/no commits; never claim fullgoalcompletion from CLI+compileproof.

## Dependency review

| Producer→consumer | Binding shared interface | Disposition |
| --- | --- | --- |
| 1→2..10 | IDs/config/errors/progress/wire witnesses | Defined before dependent source |
| 2→3..9 | Private staged operations/root-bound state/allocation ledger | No independent island publication |
| 3→5/6 | Sortedglobal inputs/localroutes/watermark | One ordering contract |
| 4→6/7/8 | Descriptorleases/queryvalues/switch | Explicit oldroot lifetime |
| 5→6/8 | Canonical fulltranscripts/heads/capacity | Independentfixtures retained |
| 6/7→8/10 | Job/drain/faultshutdown ownership | No borrowed mutableplanet access |
| 8→9/10 | Verified replaytarget/sourcelease | Separate allocation total disclosed |
| 9→10 | Capacity/cancellation/latency evidence | Numeric gaps cannot be waived by compilation |

## Controller self-review and handoff

Spec coverage checked: limits/identities/memory Tasks1/2/9; stagedeconomics/routing Tasks2/3/6; descriptors Task4; abort/cleanup/fault Tasks6/7; codecs/history Task5; replay Task8; allproof/CLI Task9/10. All five Review Focus items map to named assertions. Producer/consumer names use the Shared Interfaces block, and no task author invents replacement gameplay. The explicit integrated adapter extracts shared algorithms under preserved v0 wrappers; duplication is not a migration shortcut.

Added plan decisions are interface spellings and test/implementation decomposition; approved rules remain authoritative. Query copies are public observations, while replay/Runner scans pay the shared allowance. Fault destruction is the approved explicit outside-poll allocator operation. Self-review also checked demo tick5 arithmetic independently:20+1+6-4=23 and24+1+6-2=29; seed XOR low bits3 gives fertile71 on world2. Full snapshot comparison includes retained history occupancy, not only heads. Each phase/barrier and destruction signature has one owning task; no implicit parallel source writer. Neither gate logs nor test counts are fabricated in this plan: every verification line above is future work.

Human review required before dispatch. User already selected subagent-driven development; request confirmation this plan captures the approved spec, without asking for the execution method again. Once approved, create its ignored evidence ledger, capture existing dirty ownership/baselines, and dispatch Task1. No commits despite generic skill examples.
