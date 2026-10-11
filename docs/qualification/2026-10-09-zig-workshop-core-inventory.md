# Task 4 Workshop core inventory and sequential ownership

Date: 2026-10-09. Status: **Current source inventory; Proposed execution slices; Pending Zig implementation and verification.** This is read-only preparation for Task 4, not authorization to start it ahead of the active task and not evidence that any gate passed. Only this report was written. No compiler, build, test, benchmark, native launch, browser journey, or archive generation was run.

Canonical source: `/Users/donaldfilimon/dev/active/NYON`. The inspected non-Living Rust core consists of `src/{lib,ids,model,pack,command,simulation,history,archive}.rs` beneath `crates/nyon-workshop-core/`. Paths and line numbers below refer to that checkout on this date. A listing of `zig/src` contained Classic modules and no Workshop module; Workshop Zig implementation is therefore Pending in this inventory, without making any claim about uninspected branches or other checkouts.

## Authority and scope that must remain intact

`AGENTS.md:3-8` identifies the accepted Workshop design and master plan and explicitly distinguishes plans from implementation. Binding design: `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:7-11` (Accepted, plus accepted Library addendum). Task 4 is complete pure Workshop authority/history/archive and all 18 Forge steps, not merely types or a toy industrial example: `docs/superpowers/plans/2026-10-09-nyon-zig-migration.md:123-132`. The game migration inventory leaves pure authority and history/archive Open: `docs/qualification/2026-10-09-zig-migration.md:35-37`.

The first slice below is a prerequisite only. Task 4 remains open until strict packs, all 14 creator operations, six-phase 10 Hz mechanics, checked atomic failures, immutable revision DAG, undo/redo, branches, replay, canonical archives and the entire Forge journey have their independent and differential witnesses. Task 6 owns the platform/session/storage/UI portions required for real quit/Continue and portable transfer. Task 8 owns final artifact and live qualification; their dependencies cannot be relabeled as covered by a pure-core round trip.

The pure dependency boundary is Current in source: `lib.rs:1-10`, each module's imports, and `tests/rules_v1_facade.rs:43-96`. No client, GPU, windowing, Tokio, browser, wall clock or platform storage belongs here. The accepted design states this boundary at `...v2-design.md:125-149`. WorkshopV1 and LivingV2 identity vocabularies stay separate; `AGENTS.md:96-104` and `crates/nyon-workshop-core/tests/living_wire.rs:769-869` define the existing guard. Rust source membership is pinned at `living_wire.rs:693` and `:1042-1056`; a Rust module added during migration would require updating that list/count. A Zig port needs its own complete membership/isolation witness; passing the Rust scan would say nothing about new Zig files.

## Exact source dependency order

`ids.rs` depends only on serialization/error support. `pack.rs:9` depends on IDs/string validators. `model.rs:6-9` depends on IDs and `CreatorBatchV1` only for `RevisionRecordV1:258-265`. `command.rs:6-18` depends on IDs, state/model limits and validated pack. `simulation.rs:6-13` depends on IDs/model/validated pack and does **not** depend on command/history/archive. `history.rs:6-15` depends on command execution, state/revision records, catalog and simulation. `archive.rs:6-17` depends on all of those, including history identity formulas and command replay.

Thus the source graph is:

```text
version constants + IDs/string/geometry
    -> state entity records/capacities/canonical state
    -> pack wire + strict parsing + validated immutable catalog
IDs -> creator wire declarations -> revision record declarations
state + catalog + creator wire -> atomic creator execution
state + catalog -> deterministic simulation
creator execution + simulation + revision records -> history/replay/branches
history + replay + canonical wire -> authenticated canonical archives
all preceding -> complete 18-step pure Forge -> platform Forge dependencies
```

There is a **declaration cycle**, not a gameplay execution cycle: model includes a revision containing command wire while command implementation imports model. Do not introduce a Zig runtime cycle to mimic Rust file organization. Proposed split: first state/model slice excludes `RevisionRecordV1`; the later command-wire slice declares every operation and the revision record in a separate wire/record module. Alternatively keep wire declarations independent of command implementation. Preserve the actual serialized schema whichever file owns it.

## Bounded sequential ownership slices

The target paths are Proposed under `zig/src/workshop/`; they are ownership assignments for a future implementation, not files created by this report. Each owner handles its implementation and focused tests, publishes the reviewed interface, and yields before the next dependent owner starts. A reviewer owns no implementation edits unless explicitly reassigned. Do not modify existing Living work, root client modules or Classic authority in these slices. The orchestration owner alone handles shared `zig/src/root.zig`, build/gate registration and ledger changes after each reviewed interface, avoiding cross-slice edits to shared registration.

| Slice | Exclusive proposed ownership | Source to port and dependency | Required exit witness before dependent execution |
| --- | --- | --- | --- |
| 4A | `ids.zig`, `model.zig`, first-slice tests and dedicated immutable fixtures | `ids.rs:3-312`; `model.rs:11-253`; `lib.rs:34-35`; no revision record yet | Nominal IDs, legal-string gates, coordinates/isqrt, every entity/state field, sorted maps, capacities, canonical state bytes/digest; independent frozen positive/negative witnesses and allocation-failure preservation |
| 4B | `pack_wire.zig`, `pack.zig`, pack tests/fixtures | `pack.rs:11-666`; depends on 4A string/ID contracts | Exact wire/variant fields; strict preflight/parser; immutable validated catalog; all caps/refs; stable canonical bytes and built-in frozen hash |
| 4C | `command_wire.zig`, `record.zig`, command-wire tests/fixtures | `command.rs:20-157`; `model.rs:258-265`; depends on 4A only | All 14 variant encodings, local/existing refs, receipts and revision record bytes; 128 operations/65,536-byte constants; field-order and escape-sensitive fixtures |
| 4D | `command.zig`, creator tests/fixtures | `command.rs:159-887`; depends on reviewed 4A/4B/4C | All creator semantics, stale tick/cursor, checked ordinals, reference ordering/kinds, hash formulas, dependency reporting, all capacities and whole-authority failure preservation |
| 4E | `simulation.zig`, simulation tests/fixtures | `simulation.rs:15-561`; depends on 4A/4B; sequential execution follows 4D | Six ordered phases, recipes/reserves, direct logistics, interval boundaries, events/IDs/digests, overflow/capacity/collision/invariant atomicity |
| 4F | `history.zig`, history tests/fixtures | `history.rs:17-679`; depends on 4C/4D/4E | Immutable revisions, paused undo/explicit redo, sibling fork IDs/names, branch ticks, replay, bounded derived checkpoints and cadence-independent canonical bytes |
| 4G | `archive.zig`, archive tests/fixtures | `archive.rs:19-683`; depends on reviewed 4F and all replay primitives | Exact 16 MiB canonical envelope/payload; integrity/catalog/DAG/branches validation; bounded replay, zero budget, cancellation and incomplete finish; authenticated negative fixtures |
| 4H | `forge_tests.zig`, complete Workshop oracle/journey fixtures and report | `tests/common/mod.rs:53-184`; `tests/two_system_forge.rs:11-173`; all prior interfaces | Every numbered core Forge witness, real alloy, sibling recovery and complete canonical pack/archive parity; explicitly carry live quit/store/Continue/transfer residuals into Task 6/8 |

4B and 4C have independent dependency sets, but this proposal deliberately executes one reviewed slice at a time. 4D may be further bounded into geometry/creation, ownership/industries/routes, and removal/hazards without reducing the 14-operation exit requirement. 4G may be separated into authenticated envelope/DAG validation then incremental replay while preserving the same overall exit gate. No slice may replace known frozen goldens with values emitted by the implementation under test.

## Deep first slice: IDs, strings, integers, state and limits

### Nominal types and exact wire representations

Current source types are `WorkshopTick(u64)`, `RevisionId([u8;32])`, `EntityId([u8;16])`, `BranchId([u8;16])`, `CatalogHash([u8;32])`, `StateDigest([u8;32])`, and `BatchLocalId(u16)` (`ids.rs:5-24,85-87`). Keep them distinct Zig struct types; a shared raw-array alias or generic integer alias must not permit accidental identity rewrapping. Fixed-array identities compare bytewise lexicographically (`Ord` derives), not by native-endian integer interpretation. This ordering is canonical map order and conflict/event order later.

`ids.rs:26-75` serializes fixed IDs as **lowercase** hexadecimal strings, exactly 32 characters for 16-byte IDs and 64 for 32-byte IDs; deserialization rejects other lengths and every character outside `0-9a-f`. Zero-filled IDs are constructible; there is no nonzero sentinel contract. Tick/local wrappers serialize as JSON **numbers**, not Classic decimal strings; do not copy the Classic scenario seed/string rules into Workshop. Exact `u64` integers must survive parsing/encoding above 2^53 without a float intermediate.

`ObjectRefV1` has `{"kind":"existing","id":"<32 lower hex>"}` or `{"kind":"local","id":<u16>}` (`ids.rs:250-255`). Unknown-field rejection is explicit on `GalaxyPointV1:219-224`, state structs and command records; `ObjectRefV1` itself has no `deny_unknown_fields` annotation. Freeze the actual consumer boundary behavior rather than claiming every arbitrary serde entry point is equally strict. Strict pack/archives impose their own stronger full-document checks.

Version constants remain `WORKSHOP_RULES_VERSION = 1` and `WORKSHOP_TICK_HZ = 10` (`lib.rs:34-35`). Versioned namespaces below are literal byte sequences ending in a zero byte. Identical serialized widths between Workshop/Living do not authorize conversions. Classic is a separate frozen compatibility island (`...v2-design.md:96-112`), and imported unknown versions are rejected, not migrated (`:388-390`).

### Validated strings and ownership

`CatalogId` and `ObjectName` have private Rust string fields and both constructors/deserializers call validators (`ids.rs:89-189`). No trimming, case folding, whitespace normalization or Unicode normalization occurs. Invalid input is rejected. Zig must enforce an equally inspectable validation boundary and explicit allocation ownership; caller-backed slices and temporary parse buffers must not escape into authority/catalog state. Cloning/freeing nested string/maps is part of atomic temporary-state ownership, not an incidental utility.

| Value | Exact acceptance rule | Current source |
| --- | --- | --- |
| Catalog ID | `[a-z][a-z0-9_-]{0,31}`; 1..32 ASCII bytes | `ids.rs:277-283` |
| Pack ID | `[a-z][a-z0-9._-]{0,47}`; 1..48 ASCII bytes | `ids.rs:269-275` |
| Object/display name | 1..64 bytes in ASCII `0x20..0x7e`; no leading/trailing/repeated spaces | `ids.rs:285-291`; pack uses same validator at `pack.rs:448-468` |
| Description | 0..512 bytes in printable ASCII, with spaces permitted | `ids.rs:293-295` |
| Branch label | Same 1..64 printable/name spacing rules, separate string metadata | `history.rs:417-431`; archive rechecks at `archive.rs:656-662` |

Quotes and backslashes are valid printable name bytes and therefore test canonical JSON escaping. A canonical encoder must reproduce serde JSON escape output and stable field order, not just produce parse-equivalent JSON. A pack ID permits `.` while a Catalog ID does not; `nyon.core` is a pack identity, not a Catalog ID (`assets/workshop/core-pack-v1.json:4`). Error taxonomy from `IdentifierError` is CatalogId/ObjectName/CoordinateOutOfRange/DistanceOverflow (`ids.rs:257-267`).

### Integer geometry and tick arithmetic

Coordinates are **signed i64** stored in units of 1/1024 parsec, inclusive ±134,217,728 (`131_072 * 1_024`); the constructor/deserializer enforce range (`ids.rs:3,191-217`). `GalaxyPointV1.distance_units` checked-subtracts and squares signed i128 deltas, checked-adds squared axes, then takes floor integer sqrt and checked-converts to u64 (`:234-247`). `integer_sqrt:297-312` uses integer binary search/division; no float approximation is permitted. Opposite extreme corners have floor distance 379,625,062; known cases and symmetry are literal witnesses in `tests/simulation.rs:129-152`. Route travel later uses `max(1, (distance+255)/256)` with checked addition; same-system routes take one tick (`simulation.rs:375-388`). Tests pin distances 255/256/257, diagonal 363 and extreme travel 1,482,911 (`tests/simulation.rs:156-196`).

Arithmetic semantics belong to the interfaces now even where execution is later: no wrapping, saturating authoritative quantities, silent loss or partial publish. `simulation.rs:128-148` checks tick+1 before swapping candidates; arrival addition is checked at `:418-424`; inventory helpers use checked add/sub at `:507-535`. `history.rs:582-592` uses saturating subtraction **only for derived checkpoint cadence**, not authoritative inventory. Do not replace that with a claim that source contains no saturating operation anywhere.

### Complete state schema and canonical bytes

Preserve every field and declaration order in `model.rs:38-144`:

- Faction: name, RGB u8 triple. System: name, bounded position.
- Star: system entity, name, archetype catalog ID.
- World: system, primary star, name, archetype, orbit radius u32 milli-AU, period u64 ticks, phase u32 millidegrees, sorted CatalogId→u64 inventory.
- Lane: system endpoints and u64 integer distance.
- Deposit: world, resource, u64 reserve.
- Industry: world, definition, optional linked deposit, enabled flag.
- Route: source/destination worlds, resource, u64 batch units.
- Shipment: route, source, destination, resource, units, dispatched tick, arrival tick.
- Hazard: lane, catalog definition, start tick, duration, cancelled flag.
- State: tick then factions, systems, stars, worlds, lanes, deposits, industries, routes, shipments, hazards, owners in that exact order. Every entity collection and ownership map is EntityId-ordered.

Default state is tick zero and all maps empty (`model.rs:146-163`). `canonical_bytes:187-189` uses minified serde JSON; `digest:191-239` hashes `NYON-WORKSHOP-STATE-V1\0` plus that shape. `digest_with_dynamic` must produce the same bytes using candidate tick/worlds/deposits/shipments; static fields remain those of source state. Catalog hash, genesis seed, branch identity/labels, selected view, pause/speed/camera/checkpoints are **not** fields in state digest. Seed/catalog affect identity derivation, not empty-state digest.

Source caveat: `WorkshopStateV1` and its record fields are public/deserialize-capable; the model has **no standalone whole-state validator**. `entity_kind:166-180` returns the first matching collection and does not detect an ID duplicated across kinds; `contains_entity:183-185` is only membership. The normal construction gate is creator validation, while archive authority is reconstructed by validated replay; a manually populated model is not proven valid merely by hashing it. A Zig first slice must distinguish representable schema from validated authority state. Do not silently make arbitrary raw state directly installable.

The complete invariant owners are distributed:

| Invariant | Current enforcement owner |
| --- | --- |
| Cross-kind entity collision and duplicate local IDs | `command.rs:703-717`; shipments `simulation.rs:490-504,414-417` |
| Earlier compatible local reference only, existing-kind match | `command.rs:720-741` |
| Unique system positions; primary star in same system; valid star/world archetypes | `command.rs:356-425` |
| Nonzero orbit radius/period, phase <360,000 | `command.rs:420-424` |
| Distinct system lane endpoints and no duplicate undirected lane | `command.rs:442-462` |
| Deposit/catalog quantities 1..1,000,000,000 | `command.rs:464-476`; pack at `pack.rs:490-499` |
| Ownership faction reference and eligible target kinds | `command.rs:489-497,664-673` |
| Extractor has deposit of output resource on same world; other industry kinds cannot link deposits | `command.rs:498-538` |
| Routes have distinct worlds, resource and valid quantity, unique directed resource relationship, direct inter-system lane | `command.rs:540-581` |
| Rename only faction/system/star/world; non-cascading removal gives all current dependencies in ID order | `command.rs:590-615,674-690,782-847` |
| Hazard start >= current tick, duration nonzero and <= pack maximum, checked end | `command.rs:616-650` |
| Replay checks same semantics plus revision/digest identity | `history.rs:488-508`; archive replay `archive.rs:271-288` |

First-slice tests should define the raw-state/validated-authority boundary; full enforcement and capacity edge execution wait for their owning slices. They must not invent a new digest field or tighten serialized compatibility without explicit versioned design.

### Every capacity, without narrowing

| Resource | Limit | Source owner |
| --- | ---: | --- |
| Factions / systems / stars / worlds | 16 / 64 / 128 / 512 | `model.rs:11-14` |
| Lanes / deposits / industries / routes | 256 / 1,024 / 2,048 / 2,048 | `model.rs:15-18` |
| In-flight shipments / scheduled-or-active hazards | 4,096 / 128 | `model.rs:19-20`; shipment failure `simulation.rs:411-413`; hazard population `command.rs:755-768` |
| Creator operations / canonical batch bytes | 128 / 65,536 | `command.rs:20-21,284-295` |
| Branches / accepted revisions | 64 / 10,000 | `history.rs:17-18,210-229` |
| Canonical archive bytes / raw pack bytes | 16,777,216 / 1,048,576 | `archive.rs:19`; `pack.rs:11` |
| Pack stars / worlds / resources / industries / hazards | 32 / 64 / 32 / 64 / 32 | `pack.rs:371-380` |
| JSON nesting / quantity / duration ticks | 32 / 1,000,000,000 / 36,000 | `pack.rs:12-14` |
| Processor inputs / outputs; solar outputs | 1..8 / 1..4; 1..4 | `pack.rs:409-429,471-487` |
| Save slots | 16 | Accepted design `:231`; client/store ownership, outside pure authority |
| Derived checkpoint tick/revision cadence, cache entries | 600 / 128 / 16 | `history.rs:45-51`; derived/excluded from canonical archives |

Important source detail: the 128 hazard cap counts non-cancelled hazards whose checked end is after current tick, including future scheduled hazards. Expired/cancelled records remain in the map and do not consume that cap (`command.rs:755-768`); do not erroneously impose a permanent 128-record map size or delete records to make room. Typed rejection/fault preserves authority; no eviction or history discard is accepted (`...v2-design.md:213-235`).

The current maximal legal batch schema is smaller than its byte cap: `tests/creator.rs:468-580` enumerates all 14 variants with maximum printable/escaped fields and pins 60,298 bytes for 128 maximal CreateWorld operations. Preserve 65,536 anyway; do not change the accepted cap or invent a currently constructible valid >65,536-byte batch. Test overflow/malformed raw input separately from valid-operation schema bounds.

## Pack, command, simulation, history and archive handoffs

Pack has no separately implemented catalog module: `ValidatedCatalogPackV1` owns private validated wire, canonical bytes and hash (`pack.rs:190-210`), exposing read-only queries at `:212-274`. Strict preflight size/BOM/UTF-8/depth is `:503-542`; recursive no-float/duplicate-key parsing and trailing-value check is `:572-666`. Root/variants deny unknown fields; schema checks include global definition uniqueness, same-pack resources and recipe restrictions (`:356-499`). Decode sorts definitions and recipe resource quantities, validates, emits stable field-order JSON and hashes with the pack V1 domain (`:316-343`); export returns those canonical bytes (`:346-354`). Pack import need not start canonical; its export does. Archive import must be canonical. The Library requires exact canonical bytes when retrieving its stored matching catalog (addendum `:47-58`).

Commands create a detached cloned candidate (`command.rs:304-328`) and publish only after all operations and checked next ordinal succeed (`:203-227`). An empty operations vector is not rejected by the Rust code; preserve rather than inventing a nonempty-batch condition. Seed-u64 occupies the first eight of 32 seed bytes in little endian (`:181-185`). Current operations are CreateFaction, CreateSystem, CreateStar, CreateWorld, ConnectLane, CreateDeposit, SetOwner, PlaceIndustry, ConnectRoute, SetIndustryEnabled, RenameObject, RemoveObject, ScheduleHazard, CancelHazard (`:33-109`). Normal reject taxonomy is `:122-157`; rejections consume no canonical revision/ordinal/state. Removal is non-cascading, including in-flight shipment dependencies and lane-route dependencies (`:782-847`).

Simulation uses detached dynamic candidate maps; state only swaps at `simulation.rs:139-142`. Six phases are boundaries, ordered delivery, interleaved solar/extractors in industry ID order, processors in industry ID order, routes in route ID order, event hash/state digest/tick increment (`:73-149,151-275`). Do not run all solar then all extractors: accepted phase 3 traverses both kinds together. Recipe duration gates are absolute tick multiples (`:217,255-257`). Extractors consume full energy input and output `min(output quantity,reserve)` (`:308-354`). Inventory zero entries are retained by subtraction (`:522-535`), affecting canonical bytes. Hazard ratios apply sequentially in hazard ID order with floor after each ratio (`:394-400,464-487`); same-system routes bypass lane hazards (`:375-387`). Shipment ID uses route+dispatch tick, and capacity/collision checks precede consumption (`:411-429`). Fault taxonomy is Arithmetic/TickExhausted/ShipmentCapacity/IdentityCollision/Invariant/Encoding (`:51-64`).

History stores branches/head ticks independently from active cursor/tick (`history.rs:22-36,100-109`). Editing behind head forks; candidate authority, branch/revision collisions and capacities are checked before normal publish (`:210-290`). Undo rematerializes parent through the **same active tick**, never inverses/deletion (`:320-335`). Redo is explicit if ambiguous, can select another branch's lineage, and also rematerializes the same tick (`:337-394`). Branch switching uses that branch's stored head+last tick (`:397-414`). `step()` is an explicit operation even when the history paused flag is set (`:297-309`); pause suppresses stepping in the client session, not inside pure step itself. `advance_ticks` commits each successful tick; it is not a whole-count atomic transaction (`:312-318`). Checkpoints are bounded derived data, verified before replay use and independent per lineage (`:434-514,569-624`).

Archive bytes are sorted revisions/branches by IDs because they come from BTreeMap values, not chronological/topological order (`archive.rs:102-113,329-345`). Outer schema includes exact identity/version/catalog, 32-byte seed **numeric array**, records, branches, active view, final digest and integrity SHA numeric array (`:57-84`). Canonical envelope inspection (`:166-191`) verifies bytes/version/integrity only; `archive_catalog_hash:155-164` deliberately does not validate full authority. Full decode separately checks catalog, sorted uniqueness, parent/cursor/tick/ordinal/hash, cycles, branch reachability/origins and active tick semantics (`:324-372,489-654`). Replay processes every branch and active ancestor target, checks revision post-digests and final digest; one budget unit is an entire tick or revision (`:252-301,374-471`). No claim of bounded individual phase work follows from that unit. Budget zero can complete zero-work archives; incomplete finish is typed; dropped job publishes nothing. Source unit tests at `archive.rs:693-810` authenticate altered fixtures and witness huge replay gaps and invalid branch origins.

## Versioned identity namespace handoff

Freeze these exact formulas before executing dependent slices. All domains have a terminal NUL. `prefix16` takes the **first** 16 digest bytes. LE widths and optional markers must be literal fixtures, not native struct memory images.

| Identity | Current formula source |
| --- | --- |
| CatalogHash | SHA256(`NYON-WORKSHOP-PACK-V1\0` + canonical pack bytes), `pack.rs:335-342` |
| StateDigest | SHA256(`NYON-WORKSHOP-STATE-V1\0` + canonical state), `model.rs:218-238` |
| RevisionId | SHA256(`NYON-WORKSHOP-REVISION-V1\0` + rules u32 LE + catalog32 + seed32 + parent marker [0], or [1]+parent32 + tick u64 LE + ordinal u64 LE + canonical-batch-length u64 LE + batch), `command.rs:850-874` |
| Created EntityId | prefix16 SHA256(`NYON-WORKSHOP-ENTITY-V1\0` + revision32 + EntityKind u8 + local u16 LE), `command.rs:877-886` |
| Shipment EntityId | prefix16 SHA256(`NYON-WORKSHOP-SHIPMENT-V1\0` + route16 + dispatch tick u64 LE), `simulation.rs:537-545` |
| Root BranchId | prefix16 SHA256(`NYON-WORKSHOP-ROOT-BRANCH-V1\0` + catalog32 + seed32), `history.rs:648-654` |
| Fork BranchId | prefix16 SHA256(`NYON-WORKSHOP-FORK-BRANCH-V1\0` + source16 + cursor marker [0], or [1]+cursor32 + first revision32), `history.rs:656-678` |
| Tick events digest | SHA256(`NYON-WORKSHOP-TICK-EVENTS-V1\0` + JSON `[tick,events]`), `simulation.rs:122-127` |
| Archive integrity | SHA256(`NYON-WORKSHOP-ARCHIVE-INTEGRITY-V1\0` + payload without integrity field), `archive.rs:678-682` |
| Portable archive SHA | Plain SHA256(full canonical archive bytes), `archive.rs:135-137` |

`EntityKind` declaration order fixes hash input bytes: faction0, system1, star2, world3, lane4, deposit5, industry6, route7, shipment8, hazard9 (`model.rs:22-36`). Names serialize snake_case; industry catalog kinds are kebab-case (`pack.rs:16-33`). This distinction is part of wire compatibility.

## Frozen and independent witnesses: present versus still required

**Current source witnesses, unexecuted here:**

| Witness | Exact source | Evidence boundary |
| --- | --- | --- |
| Built-in input pack, real industrial definitions | `assets/workshop/core-pack-v1.json:1-59`; `tests/common/mod.rs:8-12` | Committed human-readable input; decode through normal validator. Raw pretty input is not exported canonical bytes |
| Frozen built-in catalog hash | `tests/pack.rs:8-22` | Literal 32-byte expected hash plus round trip; independent derivation provenance unknown in inspected files |
| Frozen empty canonical state, digest, two root branch identities, first creation revision/entity/post-state for two seeds | `tests/creator.rs:183-277` | Literal external-to-Zig oracles; provenance does not establish independently derived spec corpus by itself |
| Exact integer distance and route travel constants | `tests/simulation.rs:129-196`; unit `ids.rs:319-330` | Literal numerical witnesses; ideal for independent isqrt/ceiling arithmetic derivation |
| Map insertion order independence | `tests/simulation.rs:379-449` | Equality between differently inserted Rust maps, not a frozen external byte oracle |
| Creator kinds/reference/stale/remove/hazard limits | `tests/creator.rs:39-180,280-465,584-594` and helper tests `:608-964` | Current source regression witnesses; none were run here |
| Max batch wire 60,298 and 14 variants | `tests/creator.rs:468-580` | Exact literal + schema construction; must preserve accepted independent cap |
| Full pre-step state on arithmetic fault | Unit `simulation.rs:573-624` | Canonical before/after preservation with u64::MAX energy |
| Exact phase order, hazard floor/half-open, pacing grouping, max route quantity, shipment fault | `tests/simulation.rs:14-125,200-377` | Existing semantic regressions; not native/browser timing evidence |
| History siblings, ancestor tick recovery, cross-branch redo, checkpoint cadence | `tests/history.rs:8-281` | Same implementation replay/cadence comparisons; not independent reference implementation |
| Archive round trip, noncanonical/tamper/catalog mismatch, envelope-only inspection | `tests/archive.rs:11-119` | Current regression sources; canonical archives constructed during tests rather than committed V1 archive fixture files |
| Complete core Forge final digest | `tests/two_system_forge.rs:150-159` | Literal digest and step count, plus core journey; no real store/process quit |

No non-Living `tests/fixtures/workshop-v1/` corpus or spec-derived V1 fixture generator was present in the inspected core file listing. Living fixture/vector tools are a different authority island and do not supply Workshop independence. Do not label the above Rust goldens as independently derived without finding their provenance. This is **unknown provenance**, not an allegation that their values are wrong.

**Proposed first-slice fixture freeze, before using Zig output:** a dedicated Workshop V1 corpus containing literal input, expected exact bytes/error, expected scalar/hash, formula source and independent derivation record. Produce bytes/reference hashes through a small separately authored spec/source-contract calculator (for example a standard-library script) that does not call either production implementation. Review and freeze it, compare to the existing Rust literals, then use it from Zig tests; Rust differential checks remain a second evidence class. Do not write this corpus by calling the Zig encoder and accepting its output as goldens.

Minimum 4A corpus:

1. Every nominal ID width/order and lowercase representation: zero/all-FF, ascending/descending byte cases, each nibble, wrong lengths, uppercase, punctuation/non-ASCII, type-confusion compile witnesses. Tick/local zero/max and out-of-range/negative/float values, above-2^53 exact integers.
2. Catalog/pack/name/description boundary strings (0/1/32/33, pack48/49, name64/65, description512/513), permitted versus forbidden dot, each spacing violation, controls/DEL/non-ASCII, quotes/backslashes and their exact canonical escapes. No implicit normalization.
3. Coordinates at both limits, ±one beyond and i64 extremes; points 0, diagonal floors, 3-4-5 and opposite extreme corners. Independently calculate square, integer root and ceiling travel witnesses. First slice owns geometry only; route execution asserts the travel values later.
4. Exact empty-state bytes and digest, a compact state covering **all ten entity records plus ownership**, inventory zero versus absent and sorted ID/resource insertion permutations. Every record field and enum tag included; field removal/reorder must fail expected bytes, not merely still round-trip.
5. Capacity constant inventory with accepted values; raw-state validity boundary with duplicate cross-kind ID, missing primary/system, invalid orbital/catalog/ownership references retained as **invalid fixtures** for later authority validators, not installable valid states. Full max/max+1 atomic execution belongs to 4D/4E/4F/4G.
6. Independent namespace preimages for all kinds and optional parent/cursor markers, NUL domain, exact LE widths, seed expansion and first16 truncation. 4A can freeze the ID contracts; reviewed formulas execute in their proper command/history/simulation owners.
7. Explicit allocator-failure witnesses at validated strings, map insertion, clone and canonical encode boundaries: no leaked owned storage, dangling slices, changed live state or partial installed result. These are Zig migration requirements; Rust source alone does not witness Zig allocator behavior.

Later freeze extends the same corpus with pack canonical bytes/hash/error cases, every operation's wire and checked rejection, event bytes/digest and per-phase state, root/fork branch preimages, canonical nontrivial archives, authenticated malformed DAG/branches/ticks/versions/content and all Forge revisions/digests. Collision/exhausted ordinal/identity/tick faults need deterministic injection or carefully controlled test fixtures; existing ordinary creator-path coverage is insufficient to assert every rare fault was tested.

## Complete 18-step Forge ownership and evidence

Accepted numbered journey is `...v2-design.md:42-63`. Existing source test is `tests/two_system_forge.rs:11-159`; creation fixture `tests/common/mod.rs:53-184` includes 15 created entities and two ownership assignments in one atomic batch. Batching steps 2–8 is current test behavior, not permission to remove their individual product tools or acceptance witnesses.

| Step | Required behavior and current source witness | Future owner / residual |
| --- | --- | --- |
| 1 | Blank or deterministic seeded Workshop; test uses blank state with seed0x46524745 at `:15-18` | 4D/4F; real creation screen Task6 |
| 2 | Two systems; `:20-24` | 4D, all capacity/reference constraints |
| 3 | Star and world in each; `:25-27` | 4D, correct primary system/archetype/orbits |
| 4 | Two factions and ownership; `:28-30` | 4D; visual faction tools Task6 |
| 5 | Ore deposit reserve100; `:31-32` | 4D/4E, extraction must consume real reserve |
| 6 | Solar, extractor, foundry; `:33-34` | 4B/4D/4E, pack recipe rules |
| 7 | Direct lane; `:35-36` | 4D/4E integer geometry/logistics |
| 8 | Energy and ore routes; `:37-38` | 4D/4E real shipments/arrival |
| 9 | Real alloy; `:40-53` pins first production receipt tick3 and state tick4 | 4E/4H; full event/inventory/reserve trace, not only positive alloy assertion |
| 10 | Create a world while running; `:55-75` | 4D/4F immediate current-tick batch; paced live run Task6 |
| 11 | Schedule storm and observe deterministic logistics; `:77-94` | Test currently records digest after storm but does not itself assert logistics capacity; `tests/simulation.rs:68-108` supplies explicit floor and activation/expiry witness. 4H must combine both |
| 12 | Pause and undo last accepted batch at same tick; `:96-100` | 4F; no replacement-active rule is client Task6 |
| 13 | Redo exact former digest; `:102-105` | 4F/4H |
| 14 | Undo then alternate edit preserves siblings; `:107-123` | 4F; explicit ambiguous redo witness required |
| 15 | Branch switching restores both digests; `:125-130` | 4F/4H branch ticks/selection |
| 16 | Save, quit, Continue explicit selected valid branch; `:132-136` only encode/decode archive | 4G/4H core prerequisite; actual durable store/process/menu/Continue Task6/8. Existing test is Partial for the accepted product step |
| 17 | Pack and archive export/reimport; `:138-142` core bytes | 4B/4G/4H; native picker/durable placement and browser handoff Task6/8 |
| 18 | Same catalog, graph, branches/active view, tick and digest; `:144-159` | 4H exact bytes and identities; verify from real selected durable generation in live journey |

Accepted Library addendum `docs/superpowers/specs/2026-09-04-nyon-workshop-library-addendum.md:43-109,127-154,180-188` additionally requires exact catalog resolution, full replay before activation, observed-generation CAS, same-slot predecessor recovery, paused unsaved imported sessions, portable-transfer durability/handoff distinction and multi-slot lifecycle/race journeys. Those remain part of whole Workshop scope even though storage/UI does not move into the pure core. Partial core evidence cannot satisfy them.

## Qualification handoff and limits of this inventory

For each future slice require reviewed contracts, focused native Zig Debug and ReleaseSafe checks, fixed independent fixtures, Rust-versus-Zig bytes/receipt/rejection comparisons where applicable, allocation/failure witnesses and clean intended diff. Those are Proposed checks here, with no outcomes asserted. The orchestration owner must use the live Zig gate and record its exact commands/status after registration; do not claim Rust CI commands prove Zig implementation.

Existing repo native/fuzz/wasm commands remain those in `AGENTS.md:10-67` while Rust paths still exist. `AGENTS.md:66-67` separates compiled/static results from live runtime, accessibility and manual visual acceptance. Accepted Workshop qualification `...v2-design.md:462-499` requires source/artifact/runtime/release labels, named native/browser hosts and measured performance at entity caps. Existing acceptance ledger `docs/qualification/2026-10-09-zig-acceptance-ledger.md:61` requires all 18 Workshop steps separately with pointer/keyboard/screen-reader/reduced-motion journeys.

Current fact: source implementation and regression witnesses were read. Pending: all future Zig Workshop source, independent V1 fixture derivation/freeze, actual gates and differential results, full platform Forge, live matrix, accessibility/performance/recovery/offline acceptance. Unknown: independent derivation provenance for the existing Rust V1 literal goldens. Out of scope for this preparation: any implementation, build/test run, Living edit, Classic edit, store/UI edit, commit/push/PR or production dependency addition.
