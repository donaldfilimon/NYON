# NYON Zig Planet Economy Implementation Plan

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Canceled on 2026-10-19:** Donald canceled further Zig rewrite work. Unfinished Zig tasks and continuation instructions in this document are superseded; retain its historical evidence and existing source. See [the current implementation decision](../../../REWRITE-DECISION-2026-10-19.md).
<!-- END 2026-10-19 rewrite decision -->

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the approved one-planet authority and all four CLI stories with bounded atomic execution and independently verified history/replay.

**Architecture:** A separate `planet_economy` module owns paged state, one incremental candidate and explicit receipts. It imports only Zig/std plus the existing topology and role modules; hosts supply commands, trusted policy and work budgets.

**Tech Stack:** Zig exactly `0.18.0-dev.146+35accc06e`; standard-library SHA-256; existing shell gate; no new dependency or Rust.

**Spec:** [`2026-10-11-nyon-zig-planet-economy-design.md`](../specs/2026-10-11-nyon-zig-planet-economy-design.md). Donald approved its rules on 2026-10-11 at 02:27 EDT. Donald approved this implementation plan on 2026-10-11 at 02:47 EDT. All eight tasks are implemented and independently reviewed; the final Zig-only gate passed 585/585 tests in both Debug and ReleaseSafe. This completes the scoped in-memory authority and CLI stories; the expanded game remains Partial.

## Global Constraints

- Work in `/Users/donaldfilimon/dev/active/NYON` on existing canonical `main`; preserve dirty work and concurrent edits. No commit, push, dependency acquisition, worktree or branch creation.
- Required compiler: exactly `0.18.0-dev.146+35accc06e`; rules identity `nyon.planet-economy.v0`. No Classic/Living imports or silent reuse of their invariants, IDs, clocks or saves.
- One planet `PlanetId(1)`, present civilization `CivilizationId(7)`, side 4..32/default 8; one tick = one game hour. Complete expanded game/platform/save dependencies remain deferred as specified, not removed.
- Capacities: 64 settlements; 256 buildings/projects, 32 per settlement; 64 shipments; 64 commands/tick; 32 population/settlement; 65,535 inventory/resource including incoming reservations; freight 1..256 units over 1..64 edges; target tick ≤1,000,000.
- History: 4,096 receipts ×1,024 bytes; staging 1,024 ×1,024; no eviction. Memory ceiling 16 MiB; normative maximum requested reservation 8,642,560 bytes; directories 2×2,048 references; pool 4,096 slots ×272 bytes; pages 256 bytes.
- One shared budget, 0..4,096/default 256 units. One primitive copies ≤256 bytes or hashes ≤64 bytes; no allocation in polling; genesis/validation/admission/sorting/encoding/hash/cleanup all metered.
- Paid ruler operations remain paid; creator bypass is explicit, receipted and structurally validated. Record every published tick's trusted policy and all structurally admitted envelopes, including rejects and empty ticks.
- No legacy data access, saves, UI/platform delivery, deployment or whole-game completion claim. Native semantics, wasm compilation and real browser/host execution are separate evidence.

## Review Focus

- RF1: Borrowed command buffers mutated after `beginTick` must not alter accepted input; Task 6 owns `pe/jobs/copied-input`.
- RF2: A canceled candidate that already wrote invisible history-tail bytes must not leak receipts into retry/replay; Task 7 owns `pe/robustness/history-tail-cancel`.
- RF3: A new shipment reusing a retired slot must have a fresh ID and correct route/reservation; Task 4 owns `pe/phases/shipment-slot-reuse`.
- RF4: A long route that folds across seams must not evade repeated-cell detection; Task 3 owns `pe/commands/seam-loop`.
- RF5: An empty tick after role handoff must retain the selected policy, tick boundary and unchanged sequence watermark in replay; Task 6 owns `pe/replay/empty-handoff`.

---

## File ownership and execution order

Tasks run sequentially: **1 → 2 → 3 → 4 → 5 → 6 → 7 → 8**. Each task has a red/green cycle and an independent implementation review before the next. All paths below are relative to the canonical root. New files live under `zig/src/planet_economy/` unless their full path is given. A task owns only its listed files; later edits to shared files are limited to that task's listed integration. No task owns `surface.zig`, `play_role.zig`, the approved spec, or unrelated Classic files.

| Task | Files / responsibility |
| --- | --- |
| 1 | Create `ids.zig`, `types.zig`, `rules.zig`, `wire_vectors.zig`, `contracts_tests.zig`, `root.zig`, `zig/src/planet_economy_tests.zig`: pure typed contracts and independent vectors. |
| 2 | Create `state.zig`, `view.zig`, `job.zig`, `state_tests.zig`; update module/aggregator: storage, creation cursor, read values. |
| 3 | Create `command.zig`, `command_tests.zig`; update module/aggregator: canonical envelope codec and command cursor. |
| 4 | Create `phases.zig`, `phases_tests.zig`; update module/aggregator: deterministic economic phases. |
| 5 | Create `history.zig`, `history_tests.zig`; update module/aggregator: receipt codec, chain, bounded staged history. |
| 6 | Modify `job.zig`, `view.zig`, `root.zig`; create `replay.zig`, `stories.zig`, `jobs_tests.zig`, `replay_tests.zig`; update aggregator: public lifecycle and full stories. |
| 7 | Create `robustness_tests.zig`; update aggregator; fix only demonstrated defects in `state/command/phases/history/job/replay.zig`: maximum bounds, cancellation and failures. |
| 8 | Create `zig/src/planet_economy_cli.zig`, `zig/src/planet_economy_wasm.zig`, `zig/src/planet_economy_cli_tests.zig`, `zig/tools/check-planet-cli.zig`; modify `zig/src/main.zig`, `zig/src/root.zig`, `zig/build.zig`, `zig/tools/check.sh`, `zig/README.md`, qualification record: CLI and gate. |

## Shared types and interfaces

Use these names consistently. `Allocator` means `std.mem.Allocator`; `Authority`, `Builder`, `CreateProgress` are defined in `job.zig`, while modules own their named Cursor/Encoder/Finalizer/Runner types. `types.zig` owns data contracts; `ids.zig` owns wrappers. Aliases at `planet_economy/root.zig` make the public facade usable without importing storage internals.

- `PlanetId=enum(u32)`, `SettlementId/BuildingId/ShipmentId/Tick/Revision/CommandSequence/JobId=enum(u64)`; `CellId=struct {planet:PlanetId,local:u64}`. Reuse `play_role.CivilizationId/SessionPolicy/Provenance` and `surface.Direction`.
- `Capacities=struct {settlements:u16=64,buildings:u16=256,shipments:u16=64,history_receipts:u16=4096}`; each admitted 1..its maximum. `Config=struct {seed:u64,side:u16=8,capacities:Capacities=.{},startup:?Startup=null}`; `Startup=enum(u8){ruler_start=0}` denotes exactly `ruler-start.v0`; identity/planet/civilization are fixed constants, not caller overrides.
- `Resource=enum(u8){food=0,material=1}`, `BuildingKind=enum(u8){farm=0,quarry=1,house=2}`, `CargoSource=enum(u8){transfer=0,mint=1}`. `Command` is the seven-variant union defined by the payload table below; `CommandHeader={sequence:CommandSequence,provenance:Provenance,actor:?CivilizationId}`. Lists in host-side `Command` borrow `[]const CellId`.
- `Envelope=struct {bytes:[]const u8}` contains a complete canonical encoded envelope, not arbitrary text. `beginTick` copies at most 64×1,024 payload bytes into authority-owned slots before returning; malformed content is detected in budgeted preparation. Caller buffers may then die. `command.encodeEnvelope(header:CommandHeader,command:Command,out:[]u8) Failure!Envelope` constructs host input; it is outside polling and must fit 1,024 bytes.
- `WorkBudget=struct {remaining:u16}`; `Step=enum{pending,done}`. `Phase=enum{prepare,commands,arrivals,construction,production,population,finalize,publish,cleanup,discard}`. `Progress=union(enum){pending:PollStats,published:TickSummary,canceled,failed:Failure,invalid_request:Failure,idle}`; `PollStats={phase:Phase,units_used:u16}`; `TickSummary={tick:Tick,revision:Revision,accepted:u16,rejected:u16,chain_head:[32]u8}`.
- `Failure=error{OutOfMemory,CapacityUnsupported,InvalidEnvelope,InvalidPolicy,TooManyCommands,EnvelopeTooLarge,DuplicateSequence,SequenceAlreadyPublished,WrongRevision,Busy,InvalidBudget,StaleJob,StaleView,NotFound,InvalidPlanet,InvalidCell,HistoryFull,TickHorizon,IdExhausted,InvariantViolation,NotReady}`. `CancelResult=enum{requested,already_published,stale_job}`. These lifecycle failures are not gameplay rejection receipts.
- `RejectReason=enum(u16)` uses the exhaustive numbered list below. `Receipt` is a tagged union plus common header exactly matching the event table; no implicit native-layout serialization.
- `SettlementView={revision,id,owner,hall:CellId,population:u8,food:u32,material:u32,incoming_food:u32,incoming_material:u32,housing:u8,hunger:u8,fed_streak:u8}`; `CellView={revision,id,terrain:Terrain,settlement:?SettlementId,building:?BuildingId,is_hall:bool}`; `Terrain=enum(u8){fertile=0,rocky=1}`. Unannotated fields use their named wrapper (`revision:Revision`, `id:SettlementId` or `CellId`, `owner:CivilizationId`).
- `Stats={requested_bytes:usize,peak_pages:u16,live_pages:u16,pages_copied:u64,work_units:u64,history_used:u16,phase:?Phase}` reports noncanonical execution metrics. `ReceiptCursor=u16`; `ReceiptView={len:u16,bytes:[1024]u8}`; `ReceiptPage={written:u16,next:?ReceiptCursor}`. `receipts` fills 0..64 caller-owned rows; zero-length output makes no progress and returns the original cursor; reject output length >64 as `CapacityUnsupported`. Revision mismatch is `StaleView`.

Storage internal interfaces: `state.Storage.init(allocator:Allocator,config:Config) Failure!Storage`, `Storage.deinit(self:*Storage) void`; `state.Candidate.begin(storage:*Storage) Candidate`; `Candidate.advancePrepare(self:*Candidate,budget:*WorkBudget) Failure!Step`, `Candidate.advanceDiscard(self:*Candidate,budget:*WorkBudget) Failure!Step`, `Candidate.publish(self:*Candidate) void`. The latter is called only after a budget unit is consumed and all preparation succeeds. `Candidate` owns no separate allocation; it references the spare root and pool. Cursor types belong to their modules; public query methods never expose Candidate/page references. Task 2 also supplies `Candidate.readRecord(self:*const Candidate,kind:RecordKind,index:u16) Failure!Record`, `state.RecordWriter.init(index:u16,value:Record) RecordWriter`, and `RecordWriter.advance(self:*RecordWriter,candidate:*Candidate,budget:*WorkBudget) Failure!Step`. `RecordKind=enum{claim,settlement,building,shipment,route_cell}` and `Record=union(RecordKind)` use fixed-stride records: Claim stores optional settlement/building IDs plus hall flag; Settlement stores all SettlementView fields except revision (including its ID); Building stores ID, settlement, kind, anchor/facing/resolved footprint and remaining ticks (0 means active); Shipment stores ID, endpoints, resource/quantity, departure/arrival and route length. Full local route IDs occupy the separate three-page region. Settlement building membership is found by a metered building scan, not an unaccounted inline ID list. Route reads/writes use `Candidate.routeCell(self:*const Candidate,shipment_slot:u16,index:u16) Failure!CellId` and the same RecordWriter with `Record.route_cell={index:u16,cell:CellId}`. For `Record.route_cell={index:u16,cell:CellId}`, RecordWriter.init's outer index is the shipment slot and the payload index is the route cell. Logical receipt staging uses `Candidate.stage(self:*Candidate,receipt:Receipt,budget:*WorkBudget) Failure!Step`, retaining input references only to candidate-owned buffers until final encoding. Callers repeat a pending write/stage with the same scratch-owned value; no hidden unmetered multi-page copy is permitted.

## Canonical internal wire contract — freeze before encoder implementation

This completes the spec's required encoding decision; it is **not a durable save format**. Primitive notation: `b=u8`, `h=u16`, `w=u32`, `q=u64`; all integers little-endian. `?X` is b=0 absent or b=1 followed by X. `list(X)` is h count then X elements; strings are h byte length then exact UTF-8 bytes. No padding/trailing bytes. Enums use explicit values; unknown tags/noncanonical option/bool bytes reject the whole envelope structurally. `cell=w planet,q local`; `policy=b role,?w civilization`; role/provenance values match `play_role.zig` (0/1/2). Direction encoding is east=0, west=1, north=2, south=3, independent of inferred native backing layout.

Envelope = `q sequence,b provenance,?w actor,b command_tag,payload`. Payload fields below are in exact byte order; `bypass` is canonical boolean. Acceptance stores the same complete envelope, including sequence/provenance/actor. A route has 2..65 cells for successful admission; encoded arrays >65 or oversized envelopes are structural errors, while encoded path lengths 0/1 are gameplay `InvalidPathLength`.

| Command tag/name | Payload layout | Accepted-effect layout (after envelope) |
| --- | --- | --- |
| 0 `author_settlement` | w owner, cell anchor, b population, w food, w material, b bypass | q settlement, w food_minted, w material_minted, b population_minted |
| 1 `grant` | q settlement, w food, w material, b population, b bypass | w food_minted, w material_minted, b population_minted |
| 2 `author_build` | q settlement, b kind, cell anchor, ?b facing, b bypass | q building, list(cell) footprint, w material_waived, b ticks_waived |
| 3 `author_ship` | q source, q destination, b resource, h amount, list(cell) path, b cargo_source, b bypass | q shipment, h transferred, h minted, q departure_tick, q arrival_tick, h reserved |
| 4 `found` | q source, cell anchor | q settlement, w food_moved, w material_spent, b population_moved |
| 5 `build` | q settlement, b kind, cell anchor, ?b facing | q building, list(cell) footprint, w material_spent, b remaining_ticks |
| 6 `ship` | q source, q destination, b resource, h amount, list(cell) path | q shipment, h transferred, h reserved, q departure_tick, q arrival_tick |

Accepted footprint order is anchor then resolved neighbor; do not sort it. Internal creator-active building fields record waived cost and 2 ticks; ruler `build` effect records remaining=2 before construction phase decrements it. `author_ship` records exactly one of transferred/minted nonzero. Commands/events never store arbitrary strings.

Receipt header = ASCII `NYON-PE0-RECEIPT` (16 bytes), b event_tag, q tick, h ordinal, b provenance, ?q sequence. Genesis uses tick/ordinal 0; each tick's input receipt ordinal 0 and subsequent ordinals increment by one. Command header provenance/sequence must equal the embedded envelope; every other event uses autonomous provenance=2 and absent sequence. Header domain pins the rules identity for every receipt.

| Event tag/name | Payload after header, in order |
| --- | --- |
| 0 `genesis` | string `nyon.planet-economy.v0`, q seed, h side, h settlement_capacity, h building_capacity, h shipment_capacity, h history_capacity, ?b startup, w planet=1, w civilization=7 |
| 1 `tick_input` | q target_tick, q expected_revision, ?policy previous_policy, policy current_policy, h envelope_count |
| 2 `accepted` | h envelope_byte_length, raw envelope bytes, matching command's accepted-effect fields above |
| 3 `rejected` | h envelope_byte_length, raw envelope bytes, h reason, ?h offending_index |
| 4 `arrived` | q shipment, q source, q destination, b resource, h quantity, w destination_stock_after, w destination_reservation_after |
| 5 `completed` | q building, q settlement, b kind |
| 6 `produced` | q building, q settlement, b workers, b resource, w quantity, w stock_after |
| 7 `idle` | q building, q settlement, b reason (0 workforce/1 output_capacity), b workers_needed, b workers_available, b resource, w quantity, w stock_before, w incoming_reserved |
| 8 `supplied` | q settlement, b needed, b consumed, b shortage, w food_after |
| 9 `demography` | q settlement, b kind (0 growth/1 starvation/2 housing_blocked), b population_before, b population_after, b housing, b hunger_after, b fed_streak_after |
| 10 `tick_complete` | q revision, q last_sequence, h accepted, h rejected, h settlements, h buildings, h active_shipments |

Supply precedes a demographic event; its quantities are computed before growth/death. Supply does not serialize intermediate counters; demography serializes final counters. Tick-complete does not include its own chain hash. `offending_index` is present only for footprint/path errors: footprint anchor=0/neighbor=1; path nonadjacency identifies the target cell index of the bad edge, repeated/foreign/invalid cell identifies that cell. PathEndpointMismatch identifies index0 or the last index. All other rejections use absent index. Validation precedence is the spec's role → tag → bypass → existence → ownership → range → topology → capacities → resources. For several range faults, inspect fields in payload order; within path/footprint inspect ascending index.

`RejectReason` tags: 1 CreatorForbidden; 2 RulerForbidden; 3 CivilizationRequired; 4 WrongCivilization; 5 InvalidActor; 6 InternalCommandOnly; 7 CommandProvenanceMismatch; 8 ExplicitBypassRequired; 9 UnknownCivilization; 10 UnknownSettlement; 11 WrongOwner; 12 InvalidPlanet; 13 InvalidCell; 14 InvalidAmount; 15 InvalidPopulation; 16 InvalidFacing; 17 TerrainMismatch; 18 OccupiedCell; 19 DisconnectedFootprint; 20 SameEndpoint; 21 InvalidPathLength; 22 RepeatedCell; 23 NonAdjacentPath; 24 ForeignPathCell; 25 SettlementCapacity; 26 BuildingCapacity; 27 SettlementBuildingCapacity; 28 FreightCapacity; 29 PopulationCapacity; 30 InventoryCapacity; 31 InsufficientFood; 32 InsufficientMaterial; 33 InsufficientPopulation; 34 TickHorizon; 35 IdExhausted; 36 InvalidGrant; 37 PathEndpointMismatch. Unknown tag 0 is invalid. Range-invalid enums are structural, never gameplay rejects. InvalidGrant means all grant deltas zero; resource deficits are tested food then material then population. Capacity checks run settlement/building/per-settlement/freight/population/inventory/ID/horizon order where applicable.

Chain: `H0=SHA256("NYON-PE0-HISTORY")`; `Hnext=SHA256(Hprev || w receipt_length || receipt_bytes)`. The following **independently constructed** witnesses were calculated with Python `struct`/`hashlib` from this table, without authority code. Copy literals into `wire_vectors.zig`; runtime/gate validation uses Zig/std only. Each listed chain value starts independently from H0 and is an encoding witness, not a claim that these isolated records form a playable transcript.

```text
H0=5e9de0d8a394dfd0a508c3d2c78f8ea0e3a0ca284eda7e8535734e686c68b977
G0: genesis seed2 side4 default caps startup absent; length80
hex=4e594f4e2d5045302d524543454950540000000000000000000000020016006e796f6e2e706c616e65742d65636f6e6f6d792e7630020000000000000004004000000140000010000100000007000000
chain=897ea224b676ada48723c7b9c1e65d9de04f341663c910fc432c351207c05aff
I1: tick1 ordinal0 creator policy/null civilization, previous absent, expected revision0, one envelope; length50
hex=4e594f4e2d5045302d5245434549505401010000000000000000000200010000000000000000000000000000000000000100
chain=4968d2be58816a0f24de6369e8af666a29a6d233fb20d40428ae99343013f2c1
A2: tick2 ordinal1 creator/null actor sequence2 grant settlement1 food1 material0 population0 bypass1, accepted mint(1,0,0); length77
hex=4e594f4e2d5045302d524543454950540202000000000000000100000102000000000000001d000200000000000000000001010000000000000001000000000000000001010000000000000000
chain=7a555e0ffecee7e2d35664d0442e5d8737720192a045063cf3feb08678d42c9e
R2: same envelope/header coordinates as A2 but rejected CreatorForbidden, offending index absent; length71
hex=4e594f4e2d5045302d524543454950540302000000000000000100000102000000000000001d000200000000000000000001010000000000000001000000000000000001010000
chain=906ee30eab6a2c2785c98018c496b3560dac85448763ee55be5c175d19153a99
```

## Tasks

Every task's RED command uses the exact GREEN command listed for that task. First write its named tests, run and record nonzero status caused by the missing behavior (compile errors allowed only for the deliberately missing interface), implement, then rerun expecting exit 0. Never count an unrelated compiler/environment failure as a useful red test. Tests use `std.testing`; pseudocode assertions below prescribe values, not a replacement test framework. Add each new test module to `zig/src/planet_economy_tests.zig` as it is introduced.

### Task 1: Typed rules, wire fixtures and topology contract

**Files:** Create `zig/src/planet_economy/{ids,types,rules,wire_vectors,contracts_tests,root}.zig` and `zig/src/planet_economy_tests.zig`. **Dependencies:** existing `surface.Topology` and `play_role` only.
**Interfaces:** Produces all Shared types; `rules.validateConfig(config:Config) Failure!void`; `rules.terrain(seed:u64,cell:CellId) Terrain`; `rules.footprint(topology:surface.Topology,kind:BuildingKind,anchor:CellId,facing:?surface.Direction) Failure!Footprint`, where `Footprint={len:u8,cells:[2]CellId}`; vector constants `{bytes:[]const u8,chain:[32]u8}`. Footprint helper maps topology InvalidSide to CapacityUnsupported and topology InvalidCell to InvalidCell; it resolves geometry only; gameplay validation belongs to Task 3.

- [ ] Write `pe/contracts/limits-and-types`, `pe/contracts/seams`, `pe/contracts/wire-witnesses`: assert max config admitted; sides 0/1/3/33/65535 rejected; ID wrapper types differ; terrain(2,71)=fertile, terrain(2,74)=rocky; footprint farm71/east=[71,4]; neighbor78/north=34 and14/north=43 with reverse east; independently hash G0/I1/A2/R2 bytes to the exact heads above.

The named test constructs these observations and includes:

```zig
try std.testing.expectEqual(Terrain.fertile, rules.terrain(2, .{ .planet = @fromBackingInt(@as(u32, 1)), .local = 71 }));
try std.testing.expectError(error.CapacityUnsupported, rules.validateConfig(.{ .seed = 2, .side = 33 }));
```

- [ ] RED: `zig test zig/src/planet_economy_tests.zig --test-filter pe/contracts`.
- [ ] Implement only the shared contracts, constants, geometry helpers and literal witnesses. Exhaust all cells/sides4..32 against an independent six-basis oracle written in tests; test oracle must not call production `neighbor` to derive expected neighbors. All 24 directed face seams/corners must be covered.
- [ ] GREEN: same command; expected all `pe/contracts/*` tests pass, exit0. Review the complete wire/tag tables and independent literals before permitting Task 5 to encode them.

### Task 2: Bounded paged storage, genesis and read queries

**Files:** Create `zig/src/planet_economy/{state,view,job,state_tests}.zig`; modify `zig/src/planet_economy/root.zig` and `zig/src/planet_economy_tests.zig`. **Interfaces:** **Consumes:** Task 1 types/rules, existing topology. **Produces:** Storage/Candidate interfaces above; `job.beginCreate(allocator:Allocator,config:Config) Failure!Builder`, `pollCreate(builder:*Builder,budget:*WorkBudget) CreateProgress`, `cancelCreate(builder:*Builder) void`, `takeAuthority(builder:*Builder) Failure!Authority`; `CreateProgress=union(enum){pending,ready,canceled,failed:Failure,invalid_request:Failure,idle}`; `Builder.deinit(self:*Builder) void` allowed only after transfer or drained terminal state. `Authority.deinit(self:*Authority) void`; `Authority.summary(self:*const Authority) TickSummary`; `Authority.stats(self:*const Authority) Stats`; `Authority.settlement(self:*const Authority,id:SettlementId) Failure!SettlementView`; `Authority.cell(self:*const Authority,id:CellId) Failure!CellView`. Authority initially supports creation/views; tick methods arrive in Task 6.

- [ ] Write `pe/state/max-layout`, `pe/state/genesis-cursor`, `pe/state/cow-isolation`, `pe/state/setup-oom`: assert region pages `[24,768,64,128,32,192,8]`, total1216/root, reservation8,642,560≤16MiB; max config creates; budget0 leaves cursors identical; budgets1/256 produce identical terrain and empty genesis; clone a claim page without changing active root; cancel/drain frees all candidate references; failing setup allocator leaks nothing.

The named test constructs these observations and includes:

```zig
try std.testing.expectEqual(@as(usize, 8_642_560), authority.stats().requested_bytes);
try std.testing.expectEqualDeep(before_cell, try authority.cell(cell71));
```

- [ ] RED: `zig test zig/src/planet_economy_tests.zig --test-filter pe/state`.
- [ ] Implement the spec's fixed strides and pool, incremental directory retention/clone/release and generation/roundtrip/four-distinct-neighbor/reciprocity validation. Reserve all storage during setup; no per-poll allocation. Invariant checks are cursor work. Genesis logical receipt is staged now; Task 5 encodes it before creation becomes publicly ready in final integration.
- [ ] GREEN: same command; expected all storage tests pass, exit0, no testing-allocator leaks. Compile-time assertions pin record stride and all reserved offsets; never use `CapacityUnsupported` for a valid max configuration.

### Task 3: Complete role, envelope and economic command admission

**Files:** Create `zig/src/planet_economy/{command,command_tests}.zig`; modify `zig/src/planet_economy/root.zig` and `zig/src/planet_economy_tests.zig`. **Interfaces:** **Consumes:** Candidate, types, footprint and existing `SessionPolicy.admit`. **Produces:** `command.encodeEnvelope(...)` above; `command.Cursor.init(policy:SessionPolicy,envelopes:[]const Envelope,last_sequence:CommandSequence,target:Tick) Cursor`; `Cursor.advance(self:*Cursor,candidate:*state.Candidate,budget:*WorkBudget) Failure!Step`. Accepted/rejected logical receipts are appended to bounded candidate staging; Cursor owns only authority scratch indices, no allocations. Preparation fully parses/sorts/validates all envelopes before any command effect.

- [ ] Write `pe/commands/role-tag-bypass`, `pe/commands/atomic-costs`, `pe/commands/creator-actions`, `pe/commands/seam-loop`, `pe/commands/batch-order`: assert creator provenance under ruler→CreatorForbidden; ruler-tagged grant→CommandProvenanceMismatch; false bypass→ExplicitBypassRequired; paid farm debits6 and remaining2; author farm debits0/active/waived6+2; second-cell overlap rejects without ID/resource changes; seam loop `[70,71,4,71,4,5]`→RepeatedCell at index3; duplicate/old sequence aborts whole candidate; sorting `[3,1,2]` applies1,2,3.

The named test constructs these observations and includes:

```zig
try std.testing.expectEqual(RejectReason.CommandProvenanceMismatch, rejected.reason);
try std.testing.expectEqual(@as(u32, 6), paid_farm_effect.material_spent);
```

- [ ] RED: `zig test zig/src/planet_economy_tests.zig --test-filter pe/commands`.
- [ ] Implement all seven payloads, structural envelope codec, fixed rejection precedence and complete command-local scratch validation before effects. Test found moves2 people/4 food and consumes20 material; author_ship mint and transfer reservation/debit differences; all terrain/ownership/capacity/path/facing/grant-zero/exact-limit cases from the spec. IDs increment only on accepted creation. Every potentially multi-cell/record scan uses the caller's budget.
- [ ] GREEN: same command; expected all command tests pass, exit0. Reject malformed bool/enum/optional encoding, trailing bytes, >65 route entries and >1,024 bytes structurally before gameplay; validly encoded gameplay rejects remain receipts.

### Task 4: Construction, freight, workforce and population phases

**Files:** Create `zig/src/planet_economy/{phases,phases_tests}.zig`; modify `zig/src/planet_economy/root.zig` and `zig/src/planet_economy_tests.zig`. **Interfaces:** **Consumes:** Candidate and command-created records. **Produces:** `phases.Cursor.init(phase:Phase,target:Tick) Cursor`; `Cursor.advance(self:*Cursor,candidate:*state.Candidate,budget:*WorkBudget) Failure!Step` for arrivals/construction/production/population only. Candidate staging receives typed autonomous events in phase/ID order.

- [ ] Write `pe/phases/construction-and-work`, `pe/phases/supply-counters`, `pe/phases/freight-reservation`, `pe/phases/shipment-slot-reuse`: assert paid build T remaining1, T+1 active/first output; authored active produces at T; two producers need2 workers each; workforce failure wins over capacity; idle producer reserves no workers; hunger3 loses1/reset0; fed8 grows1 or housing-blocks/reset0; shipment created T path3 arrives T+3 before supply, never earlier; retired slot's next ID differs and reservations/path match new shipment.

The named test constructs these observations and includes:

```zig
try std.testing.expectEqual(@as(u8, 1), starving_settlement.population);
try std.testing.expectEqual(@as(u8, 0), starving_settlement.hunger);
try std.testing.expect(old_shipment_id != new_shipment_id);
```

- [ ] RED: `zig test zig/src/planet_economy_tests.zig --test-filter pe/phases`.
- [ ] Implement ascending-ID cursors, all-or-nothing outputs, incoming reservations, one-time housing activation, partial shortage and population0 resets. Test house cap32, same-tick grants/founding and next-tick demographic workforce changes. No clocks, route searches, per-cell movement mutation or production allocation.
- [ ] GREEN: same command; expected all phase tests pass, exit0; budgets1/7/256 give identical typed effects. Compare quantities and conservation, not only event counts.

### Task 5: Canonical receipts, incremental hash and visible history

**Files:** Create `zig/src/planet_economy/{history,history_tests}.zig`; modify `zig/src/planet_economy/root.zig` and `zig/src/planet_economy_tests.zig`. **Interfaces:** **Consumes:** Task 1 wire contract/literals, Task 3 envelope bytes, Candidate event staging. **Produces:** `history.Encoder.init(receipt:*const Receipt,out:[]u8) Encoder`, `Encoder.advance(self:*Encoder,budget:*WorkBudget) Failure!Step`; `history.Chain.init() Chain`, `Chain.begin(self:*Chain,receipt:[]const u8) Failure!void`, `Chain.advance(self:*Chain,budget:*WorkBudget) Failure!Step`, `Chain.head(self:*const Chain) [32]u8`; `history.Finalizer.init(candidate:*state.Candidate) Finalizer`, `Finalizer.advance(self:*Finalizer,budget:*WorkBudget) Failure!Step`. Views/encoder input borrowing ends only at done; candidate owns those buffers throughout.

- [ ] Write `pe/history/independent-wire`, `pe/history/all-tags`, `pe/history/bounded-tail`: assert G0/I1/A2/R2 exact lengths/bytes and independent heads; all11 event tags encode complete prescribed fields; every command effect layout is covered; zero budget writes/hashes nothing; each hash primitive handles≤64 bytes; HistoryFull never changes visible count/head; encoded receipt length≤1,024 including header/effects.

The named test constructs these observations and includes:

```zig
try std.testing.expectEqualSlices(u8, wire_vectors.A2.bytes, encoded[0..encoded_len]);
try std.testing.expectEqualDeep(wire_vectors.A2.chain, chain.head());
```

- [ ] RED: `zig test zig/src/planet_economy_tests.zig --test-filter pe/history`.
- [ ] Implement table-driven bounded scalar/list encoding with cursors; encode one logical record into bounded working scratch before replacing its staging slot, so encoding never overwrites unread input; copy finalized receipt bytes only into invisible history tail; finalize candidate count/head without changing active metadata. Compare literals directly; never regenerate them from encoder output. Add independently field-assembled expectations for remaining event/command variants, including option absence/presence and maximum 65-cell path.
- [ ] GREEN: same command; expected all history tests pass, exit0. Assert command header/envelope provenance and sequence equality; no state hash or unmetered serialization loop.

### Task 6: Public tick lifecycle, replay and all authority stories

**Files:** Create `zig/src/planet_economy/{replay,stories,jobs_tests,replay_tests}.zig`; modify `zig/src/planet_economy/{job,view,root}.zig` and `zig/src/planet_economy_tests.zig`. **Interfaces:** **Consumes:** all prior Cursor/Finalizer/Storage interfaces. **Produces:** `Authority.beginTick(self:*Authority,expected:Revision,policy:SessionPolicy,batch:[]const Envelope) Failure!JobId`; `Authority.poll(self:*Authority,job:JobId,budget:*WorkBudget) Progress`; `Authority.cancel(self:*Authority,job:JobId) CancelResult`; `Authority.receipts(self:*const Authority,revision:Revision,cursor:ReceiptCursor,out:[]ReceiptView) Failure!ReceiptPage`; `replay.Cursor.init(source:*const Authority) Cursor`, `Cursor.advance(self:*Cursor,target:*Authority,budget:*WorkBudget) Failure!Step`; `stories.Runner.init(allocator:Allocator,story:Story) Failure!Runner`, `Runner.poll(self:*Runner,budget:*WorkBudget) Failure!StoryProgress`, `Runner.receipts(self:*const Runner,revision:Revision,cursor:ReceiptCursor,out:[]ReceiptView) Failure!ReceiptPage`, `Runner.acknowledge(self:*Runner,revision:Revision) Failure!void`, `Runner.deinit(self:*Runner) void`; `stories.run(allocator:Allocator,story:Story,budget:u16) Failure!StoryResult` is the test convenience driver over Runner, not a separate simulation. `Story=enum{combined,creator,ruler_start,policy_replay}`; `StoryResult={ticks:u64,chain_head:[32]u8,assertions:u32}`; `StoryProgress=union(enum){pending,published:StoryTick,awaiting_observer,complete:StoryResult}`; `StoryTick={case_index:u8,summary:TickSummary,a:?SettlementView,b:?SettlementView}`. Runner exposes a published observation once, then waits for explicit acknowledgment while the host pages actual receipt bytes and formats its bounded value rows; it never calls a host callback from authority. No later tick or variant replaces that authority until acknowledgment. The CLI uses Runner, not only the final StoryResult. Runner preallocates its finite authority instances during init (at most four for two original/replay policy witnesses); per-authority memory bounds remain enforced. Runner initialization failure releases prior allocations; Runner propagates terminal errors only after draining its live jobs under the supplied budget, and deinit requires complete/drained state. Replay borrows an unchanged source revision; target must be fresh with matching config and ready genesis; stale source returns StaleView.

- [ ] Write `pe/jobs/copied-input`, `pe/jobs/publication`, `pe/replay/empty-handoff`, `pe/stories/combined`, `pe/stories/creator`, `pe/stories/ruler-start`, `pe/stories/policy-replay`. Assert copied request survives caller overwrite; no receipt/state visible before swap; published once then cleanup/idle; Busy until cleanup; stale JobId cannot affect next job.

The named test constructs these observations and includes:

```zig
try std.testing.expectEqual(@as(u32, 18), final_a.food);
try std.testing.expectEqual(@as(u32, 24), final_a.material);
try std.testing.expectEqual(@as(u8, 1), final_b.population);
```

- [ ] RED: `zig test zig/src/planet_economy_tests.zig --test-filter pe/jobs --test-filter pe/replay --test-filter pe/stories`.
- [ ] Implement one phase dispatcher with the single remaining-budget pointer, typed terminal delivery and cleanup. Invalid budget or stale job returns `invalid_request` without changing job state; a valid subsequent poll can continue. The same nonterminal invalid-budget rule applies to pollCreate. `failed` remains a once-delivered terminal candidate outcome followed by cleanup. Creation now encodes/hashes genesis before ready. Implement incremental replay extraction from genesis/tick_input/full accepted+rejected envelopes/tick_complete, with no second unbounded log; tick preparation verifies trusted policy via `SessionPolicy.init` without inferring it from envelope provenance.
- [ ] Pin combined story tuples `(Afood,Amaterial,Bpopulation,Bfood,Bhunger)` exactly `[(4,10,2,4,0),(6,12,2,2,0),(8,14,2,0,0),(10,16,2,2,0),(12,18,2,0,0),(14,20,2,0,1),(16,22,2,0,2),(18,24,1,0,0)]`; A population always4; farm footprint71/4; quarry74; arrival4; final totals food18/material24/population5. Assert reject sequences6/7 and watermark7, not merely the final stocks.
- [ ] Pin creator story `(Afood,Amaterial,Bfood)=[(2,2,4),(4,4,2),(6,6,0),(8,8,2)]`, A housing8/active three buildings/waived material18/ticks6/minted freight4; ruler-start `(food,material)=[(8,20),(4,14),(6,14)]` with creator→ruler policy at tick2; policy-replay tick2/3 food `(5,1)` versus `(4,0)` and watermark2, preserving grant reject and empty tick. Failed old-sequence/canceled sequence3 adds no replay input or policy change.
- [ ] GREEN: same command; expected all lifecycle/replay/stories pass, exit0. For all stories run budgets1/7/256/4096 and alternating zero/nonzero scheduling; require identical logical queries, complete receipt bytes and chain heads for a given story. Accepted-command counts are exactly6 for creator, 2 for ruler-start, 2 for the creator-policy replay witness and 1 for the ruler-policy witness (with one gameplay rejection); state and full receipt assertions remain mandatory.

### Task 7: Maximum capacities, cancellation and failure proof

**Files:** Create `zig/src/planet_economy/robustness_tests.zig`; modify `zig/src/planet_economy_tests.zig`; attributable fixes only in `zig/src/planet_economy/{state,command,phases,history,job,replay}.zig`. **Interfaces:** **Consumes/produces:** unchanged public interfaces; adds adversarial tests and only attributable fixes.

- [ ] Write `pe/robustness/every-cursor-cancel`, `pe/robustness/history-tail-cancel`, `pe/robustness/simultaneous-max`, `pe/robustness/limits`, `pe/robustness/no-poll-allocation`: cancel at each primitive boundary through publication and disposal, resubmit the identical batch and compare full state/receipts; before swap preserves old root/head/policy/watermark, after swap returns already_published; invisible tail bytes never appear after retry; max side/capacities construct and all mutable pages can be dirtied while active root remains intact; peak page use≤4,096; allocations after setup=0.

The named test constructs these observations and includes:

```zig
try std.testing.expectEqual(@as(usize, 0), polling_allocator_attempts);
try std.testing.expect(authority.stats().peak_pages <= 4096);
try std.testing.expectEqualDeep(before_summary, authority.summary());
```

- [ ] RED: `zig test zig/src/planet_economy_tests.zig --test-filter pe/robustness`.
- [ ] Add test-only cursor/allocator instrumentation without public mutation hooks. Exercise exact/one-over entity and inventory bounds, HistoryFull, staging invariant fault, IDs at exhaustion, tick1,000,000 and arrival-horizon rejection, tiny history config, invalid budgets>4,096, pending-view revision and output paging. Inject setup allocation failure and prepublication invariant failures; old state remains queryable. Meter comparisons, scans, clones, refcounts, encoding and retirement against the same budget counter.
- [ ] GREEN: same command, then `zig test zig/src/planet_economy_tests.zig -O safe`; expected exit0/all tests pass/no leaks. Benchmark poll256 separately on named macOS hardware: record samples/p95/max/peak pages/requested bytes against ≤2ms p95 target; report unmeasured/missed target honestly without weakening semantic assertions.

### Task 8: CLI integration, Rust-blocked gate and wasm compile boundary

**Files:** Create `zig/src/{planet_economy_cli,planet_economy_wasm,planet_economy_cli_tests}.zig` and `zig/tools/check-planet-cli.zig`; modify `zig/src/{main,root,planet_economy_tests}.zig`, `zig/build.zig`, `zig/tools/check.sh`, `zig/README.md`, `docs/qualification/2026-10-11-zig-rewrite.md`. **Interfaces:** **Consumes:** `stories.Runner`, Story, public authority. **Produces:** `planet_economy_cli.run(allocator:Allocator,args:[]const []const u8,out:*std.Io.Writer) !void`, called by `main` only for `planet-economy-demo`; facade export `nyon.planet_economy`; wasm probe `export fn planetEconomyCompileProbe(buffer:[*]u8,len:usize) u64` using caller memory with FixedBufferAllocator and exercising every public lifecycle/query/replay function sufficiently to force compilation. No host/browser API or execution claim.

- [ ] Write `pe/cli/stories-and-errors` in the CLI test module: default combined and all explicit story names emit identity/config, authored-start notice where applicable, published receipt lines, each expected table row and final PASS; wrong seed/side for fixed stories, unknown/missing/duplicate flags, budget0 or>4,096 and unknown story fail nonzero with typed diagnostic. Permit only seed2/side4 for these fixed acceptance scripts; other valid authority configs remain library tests, not misleading demo rows. Internal zero-budget scheduling remains tested by Task 6.

The named test constructs these observations and includes:

```zig
try std.testing.expectEqual(@as(u8, 0), valid_process_exit);
try std.testing.expect(invalid_process_exit != 0);
try std.testing.expectEqualStrings(expected_tick8_row, actual_tick8_row);
```

- [ ] RED: add the CLI test import, then `zig test zig/src/planet_economy_tests.zig --test-filter pe/cli`; run `zig build --build-file zig/build.zig run -- planet-economy-demo --seed 2 --side 4 --budget 256` and expect unsupported-command failure before integration.
- [ ] Implement parser/formatter and root/main wiring without changing Classic commands. Drive `stories.Runner` with public API inputs, page real published receipts, print StoryTick rows, then acknowledge; never reconstruct output from expected constants or a hard-coded state world. Test convenience `stories.run` drains the same interface. Add `zig build` steps `test-planet`, `check-planet-cli` and `check-planet-wasm`; the first runs the isolated aggregate, CLI checker is the native executable `zig/tools/check-planet-cli.zig` using Zig/std subprocess invocation with expected statuses/output; its `main(init:std.process.Init) !void` takes the just-built nyon-zig executable as its sole argument, wasm creates an object for `wasm32-freestanding` from the probe. Required gate adds these checks; preserve exact pin/Rust blockers and existing checks; add the new Zig checker to `zig fmt --check` explicitly because the existing gate only formats `build.zig` and `src`. Document scoped evidence and deferred game/host/save requirements.
- [ ] GREEN: `zig build --build-file zig/build.zig check-planet-cli`; `zig build --build-file zig/build.zig check-planet-wasm`; `zig/tools/check.sh`; `git diff --check`. Expected exit0 for each; all four real CLI stories and rejected-argument subprocesses verified, native tests Debug/ReleaseSafe green, wasm authority object compiled. Record full logs and actual command exit codes. Build/test wasm object is compilation only; no live browser, IndexedDB, rendering or final Rust-free repository claim.

## Plan review and handoff

After all eight tasks and their reviews, perform a whole-slice diff/requirements review and rerun the required gate only if the final review changes code or identifies an unresolved concern. Keep independent fixture literals unchanged unless the approved contract itself changes through review. Report code/semantic/CLI/compile/performance evidence separately. Do not commit or publish.

**Donald approved this plan and implementation on 2026-10-11 at 02:47 EDT. Execution is subagent-driven; implementation and qualification status are recorded in the task ledger and qualification report. Approval is not completion evidence.**
