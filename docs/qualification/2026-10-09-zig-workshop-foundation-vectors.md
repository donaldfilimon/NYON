# Task 4A foundation contracts and independent-vector seams

Date: 2026-10-09. **Current:** read-only source inspection and this report. **Proposed:** the interfaces, fixture derivation and tests below. **Pending:** fixture creation/review/freeze, Zig implementation, every runtime/gate/differential result. This report freezes the inspected evidence at the SHA-256 bindings below; it does not claim that an independent fixture corpus is already frozen.

Canonical checkout: `/Users/donaldfilimon/dev/active/NYON`. Inspection observed dirty `main...origin/main [ahead 11]`; no existing edits were changed. Only this report was written. No source generator, fixture generator, build, test, benchmark, dependency/profile edit, commit or remote action was performed. The required migration-plan Task 4 and existing inventory were read first; subsequent inspection was limited to non-Living foundation contracts and their narrow hash/genesis/test seams.

## Authority and evidence classes

| Document | Status and role |
| --- | --- |
| `docs/superpowers/specs/2026-09-02-nyon-v2-design.md` | **Accepted normative Workshop V1 product design**, explicitly Accepted at `:7`; owns pure-core boundary `:125-149`, types/geometry `:176-211`, capacities and atomic failure `:213-235`, identity semantics `:318-320`, canonical pack rules `:370-390`, and digest exclusions `:341,356`. |
| `docs/superpowers/specs/2026-09-04-nyon-workshop-library-addendum.md` | **Accepted normative addendum**, `:3,8,14-18`; preserves V1 formats/authority and extends Library/transfer behavior. These platform/session requirements do not move into 4A. |
| `docs/superpowers/plans/2026-10-09-nyon-zig-migration.md` | Execution target, not implementation proof. Task 4 requires complete pure Workshop and independent fixtures before Zig-output oracles at `:132-158`; decomposition and declaration-cycle split at `:144-150`. |
| `docs/qualification/2026-10-09-zig-workshop-core-inventory.md` | Current source inventory plus **Proposed** slices, not a new normative specification or a passed gate (`:3,33-50`). This report refines 4A only. |
| `docs/superpowers/specs/2026-09-08-workshop-library-route-design.md` | Header says **Design**, authorized for implementation planning, and disclaims source/runtime completion (`:5-11`). Reference for later product routing; no new 4A authority inferred from its filename or implementation notes. |
| Historical Intergalactic Warfare design; Living V2/V3 materials and Proposed fixture directories | Outside this 4A authority. Accepted Workshop design distinguishes historical RulesV1 reference at `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:15`; current core separation is `AGENTS.md:96-104`. No Living corpus supplies Workshop V1 independence. |

The accepted design specifies broad identity/hash semantics and canonical behavior but does not spell out every state-record JSON field, the literal state-domain bytes, or every preimage width/marker. Those details below are **Current source compatibility contracts** with exact citations. An independently authored calculator derived from these details is **source-contract-derived**, not automatically a complete spec-derived reference implementation. Record that provenance honestly and review the contract before freezing fixtures.

## Smallest reviewed boundary

**Proposed:** start with `zig/src/workshop/ids.zig` and `model.zig`, foundation tests and a dedicated immutable Workshop V1 fixture directory. IDs owns nominal scalar/fixed-byte types, validated string helpers, geometry and `ObjectRefV1`; model owns entity records, capacities, sorted-state representation, equality/copy/free and canonical state encoding/digest. Keep canonical writer helpers private to model unless reuse later justifies a third file. Version constants stay pure and may be colocated without adding command/catalog imports. Root/build registration stays with the orchestration owner. This follows `docs/qualification/2026-10-09-zig-workshop-core-inventory.md:37-43` and `crates/nyon-workshop-core/src/lib.rs:34-35`.

Proposed reviewable API contract, names subject to existing Zig conventions:

- IDs: parse/format fixed hex, bytewise compare/equality; `CatalogId.initOwned`, `ObjectName.initOwned`, `clone`, `deinit`, immutable borrowed bytes; pure `isPackId`/`isDescription` helpers; checked `GalaxyCoord.init` and `GalaxyPoint.distanceUnits`.
- Model: `initEmpty`, deep `clone(allocator)`, `deinit(allocator)`, content `eql`, deterministic map lookup/insertion/iteration, `entityKind`, `containsEntity`, `canonicalBytes(allocator)` returning owned bytes and `digest(allocator)` returning a typed digest or allocation error. A writer-based encoder can be a private alternative; encoding and hashing must use one field-order contract.
- Model's canonical dynamic view borrows static maps plus candidate tick/worlds/deposits/shipments for the duration of encoding only. It must match encoding a full state with those substitutions (`crates/nyon-workshop-core/src/model.rs:191-238`). No temporary borrowed buffer becomes owned authority storage.
- Do not add a public raw-state installation API or claim `digest` validates authority. Keep any future validation/installation capability behind the command/replay owners.

**4B owns** pack wire variants, strict whole-document parser/preflight, catalog definitions/recipe caps/reference validation, immutable catalog and canonical pack bytes/hash (`crates/nyon-workshop-core/src/pack.rs:190-210,316-342`; `docs/qualification/2026-10-09-zig-workshop-core-inventory.md:42`). 4A exposes the shared string/ID rules but does not port a catalog.

**4C owns** all creator wire declarations/receipts and `RevisionRecordV1`, including record field order `id,parent,tick,ordinal,batch,state_digest` (`crates/nyon-workshop-core/src/model.rs:256-265`; `docs/qualification/2026-10-09-zig-workshop-core-inventory.md:43`). The Rust declaration cycle is model importing `CreatorBatchV1` at `crates/nyon-workshop-core/src/model.rs:6-8` while command imports model. Zig model must not import command execution. Put wire declarations in `command_wire.zig` and record in `record.zig`, depending on IDs/model only as needed; command execution later imports both. This is the explicit migration-plan requirement at `docs/superpowers/plans/2026-10-09-nyon-zig-migration.md:149-150`.

4D/4E/4F/4G own validated mutation, full capacities, simulation, histories and authenticated replay. 4A may freeze their namespace preimage contracts as fixtures, but must not grow into those implementations. All Forge and platform Continue/transfer work remains later (`docs/superpowers/plans/2026-10-09-nyon-zig-migration.md:146-158`).

## Exact 4A types, value rules and ordering

| Type / value | Exact contract and source |
| --- | --- |
| `WorkshopTick` | Distinct wrapper of `u64`; JSON number, zero default, numeric ordering (`crates/nyon-workshop-core/src/ids.rs:5-9`). |
| `BatchLocalId` | Distinct wrapper of `u16`; JSON number, zero is legal (`crates/nyon-workshop-core/src/ids.rs:85-87`; `crates/nyon-workshop-core/tests/common/mod.rs:22-23`). |
| `RevisionId`, `CatalogHash`, `StateDigest` | Three distinct `[u8;32]` wrappers, each 64 lowercase hex characters on wire (`crates/nyon-workshop-core/src/ids.rs:11-24,71-75`). |
| `EntityId`, `BranchId` | Two distinct `[u8;16]` wrappers, each 32 lowercase hex characters (`crates/nyon-workshop-core/src/ids.rs:14-18,72-73`). Zero is a representable value; no nonzero sentinel validator exists (`:43-65`). |
| Fixed identity parsing | Exact decoded-string length; only `0-9a-f`; rejects uppercase/punctuation/non-ASCII. Array comparison is bytewise lexicographic, from derived `Ord`, not host-endian integer comparison (`crates/nyon-workshop-core/src/ids.rs:11-24,26-75`). JSON string escape acceptance is a consumer-parser concern; lowercase decoded IDs do not by themselves require canonical input text. |
| `CatalogId` | Owned validated bytes, `[a-z][a-z0-9_-]{0,31}`, 1..32 bytes; constructor and deserializer validate; lexical byte ordering (`crates/nyon-workshop-core/src/ids.rs:89-137,277-283`). |
| `ObjectName` | Owned validated 1..64 printable ASCII bytes `0x20..0x7e`; no leading/trailing/doubled spaces; constructor and deserializer validate; content equality and lexical ordering (`crates/nyon-workshop-core/src/ids.rs:140-189,285-291`). |
| Pack ID / description helpers | Pack ID `[a-z][a-z0-9._-]{0,47}`, 1..48 bytes; description 0..512 printable ASCII bytes, including repeated/edge spaces (`crates/nyon-workshop-core/src/ids.rs:269-275,293-295`). A dot is permitted only for pack IDs. |
| `GalaxyCoord` | Private validated `i64`, inclusive `[-134217728,134217728]`, units 1/1024 parsec. `GalaxyPointV1` fields are `x,y`; unknown fields explicitly denied (`crates/nyon-workshop-core/src/ids.rs:3,191-231`). |
| Distance | Checked signed `i128` subtraction/squaring/sum, then floor integer square root and checked `u64` conversion (`crates/nyon-workshop-core/src/ids.rs:234-247,297-312`). Legal coordinate range makes overflow unreachable through normal constructors; do not fake a legal coordinate overflow witness. |
| `ObjectRefV1` | Existing: `{"kind":"existing","id":"<32 lowercase hex>"}`; Local: `{"kind":"local","id":<u16>}`. Tagged enum declaration is `crates/nyon-workshop-core/src/ids.rs:250-255`; no `deny_unknown_fields` annotation here. Do not generalize stronger pack/archive parser rules to arbitrary raw scalar/ref deserialization. |
| Errors / versions | `IdentifierError` is CatalogId, ObjectName, CoordinateOutOfRange, DistanceOverflow (`crates/nyon-workshop-core/src/ids.rs:257-267`). Rules version `u32=1`; tick rate `u32=10` (`crates/nyon-workshop-core/src/lib.rs:34-35`). Allocation failure is an explicit Zig ownership/API requirement, not a witnessed Rust identifier-error variant. |

No trimming, case folding or whitespace/Unicode normalization occurs in the validators (`crates/nyon-workshop-core/src/ids.rs:94-100,145-151,269-295`). Names can contain quotes and backslashes; canonical JSON must escape them. Integer encoding/parsing must preserve all `u64` values, including those above 2^53, without a floating-point intermediate (type contract at `crates/nyon-workshop-core/src/ids.rs:9,87`; state `crates/nyon-workshop-core/src/model.rs:67-70,114-126`). Characterize signed/unsigned negative-zero lexemes and consumer error mapping separately before freezing decoder fixtures; no runtime result was obtained here.

`EntityKind` is a `repr(u8)` enum with snake_case JSON names. Exact discriminator order is **Faction=0, System=1, Star=2, World=3, Lane=4, Deposit=5, Industry=6, Route=7, Shipment=8, Hazard=9** (`crates/nyon-workshop-core/src/model.rs:22-36`). Pack industry kinds are separate kebab-case catalog wire at `crates/nyon-workshop-core/src/pack.rs:16-33`, owned by 4B.

## Every entity field, canonical order and equality

Field order below is declaration/serialization order; all record/state structs explicitly deny unknown fields and derive value equality (`crates/nyon-workshop-core/src/model.rs:38-144`). Names and catalog IDs retain their validated type wrappers; all references below are `EntityId` unless indicated.

| Record | Ordered fields and types | Exact source |
| --- | --- | --- |
| Faction | `name:ObjectName, color_rgb:[u8;3]` | `crates/nyon-workshop-core/src/model.rs:38-43` |
| System | `name:ObjectName, position:GalaxyPointV1` | `crates/nyon-workshop-core/src/model.rs:45-50` |
| Star | `system, name:ObjectName, archetype_id:CatalogId` | `crates/nyon-workshop-core/src/model.rs:52-58` |
| World | `system, primary, name:ObjectName, archetype_id:CatalogId, orbit_radius_milli_au:u32, orbit_period_ticks:u64, phase_millidegrees:u32, inventory:CatalogId→u64` | `crates/nyon-workshop-core/src/model.rs:60-71` |
| Lane | `a,b,distance_units:u64` | `crates/nyon-workshop-core/src/model.rs:73-79` |
| Deposit | `world,resource_id:CatalogId,reserve_units:u64` | `crates/nyon-workshop-core/src/model.rs:81-87` |
| Industry | `world,definition_id:CatalogId,linked_deposit:?EntityId,enabled:bool` | `crates/nyon-workshop-core/src/model.rs:89-96` |
| Route | `source,destination,resource_id:CatalogId,batch_units:u64` | `crates/nyon-workshop-core/src/model.rs:98-105` |
| Shipment | `route,source,destination,resource_id:CatalogId,units:u64,dispatched_tick:WorkshopTick,arrival_tick:WorkshopTick` | `crates/nyon-workshop-core/src/model.rs:107-117` |
| Hazard | `lane,hazard_id:CatalogId,start_tick:WorkshopTick,duration_ticks:u64,cancelled:bool` | `crates/nyon-workshop-core/src/model.rs:119-127` |
| State | `tick:WorkshopTick,factions,systems,stars,worlds,lanes,deposits,industries,routes,shipments,hazards,owners` | `crates/nyon-workshop-core/src/model.rs:129-144` |

Every entity map is `EntityId→record`; owners is `EntityId→EntityId`; every map serializes as a JSON object sorted by its key's derived ordering. Inventory sorts `CatalogId` lexically. Record field order is **not** alphabetical. Optional linked deposit serializes as `null` or the hex string. Default state has tick zero and all eleven maps empty (`crates/nyon-workshop-core/src/model.rs:129-163`).

Canonical state bytes are minified JSON with decimal integer numbers, lowercase string IDs, ordered object fields/maps, booleans and null; there is no newline or native struct-memory encoding. `canonical_bytes` serializes the state, and `digest_with_dynamic` separately declares the identical shape (`crates/nyon-workshop-core/src/model.rs:187-238`). Its output hash is full 32-byte SHA-256 of `b"NYON-WORKSHOP-STATE-V1\0" || canonical_state_bytes` (`:218-238`). Catalog hash, seed, branch identity/labels, selected view, pause/speed/camera/checkpoints are absent from the state schema; design independently excludes presentation/branch metadata (`docs/superpowers/specs/2026-09-02-nyon-v2-design.md:341,356`).

**Proposed equality:** compare typed scalar bytes/numbers and all nested fields, string contents and maps by key/value contents, never allocator addresses, capacities, padding or insertion order. Clone equality and identical encoding are necessary; hash equality alone is not a structural-equality implementation. Rust derives deep equality on state and records (`crates/nyon-workshop-core/src/model.rs:38-144`) and validated strings (`crates/nyon-workshop-core/src/ids.rs:89,140`). Zero-valued inventory entries must remain distinct from absent entries, because subtraction retains a zero map entry (`crates/nyon-workshop-core/src/simulation.rs:522-535`). A clone owns independent strings/map storage; mutating/freeing one copy must not affect the other. Rust copy strategy is detached `state.clone()` before mutation (`crates/nyon-workshop-core/src/command.rs:304-328`); Zig allocation behavior remains Pending.

## Raw schema, validation and capacity boundary

**Current:** there is no standalone whole-state validator in model. `entity_kind` searches collections in kind order and returns the first hit; `contains_entity` only checks membership (`crates/nyon-workshop-core/src/model.rs:165-185`). Public state fields/derived deserialization allow structurally representable states that are not valid authority (`:129-144`). Canonical serialization/digest does not establish reference, ownership, catalog, geometry or capacity validity (`:187-238`).

Cross-kind collision checks belong to creator allocation (`crates/nyon-workshop-core/src/command.rs:703-717`), with simulation checks for shipment creation as routed in the prior inventory (`docs/qualification/2026-10-09-zig-workshop-core-inventory.md:103-120`). Do not add a new installable raw-state fast path or silently tighten a raw schema contract. First-slice fixtures should label dangling primary/system/catalog/owner references and cross-kind duplicates **invalid authority examples**, although raw serialization can be exercised. Later command/archive replay must reject invalid installation. Exact duplicate JSON-map-key behavior at arbitrary raw-state serde entry points was not executed or fully characterized; do not advertise a foundation strict archive decoder.

4A exports these exact constants: factions16, systems64, stars128, worlds512, lanes256, deposits1024, industries2048, routes2048, shipments4096, hazards128 (`crates/nyon-workshop-core/src/model.rs:11-20`), matching accepted hard limits (`docs/superpowers/specs/2026-09-02-nyon-v2-design.md:213-226`). **Hazards128 means scheduled-or-active**, not historical map length: cancelled or checked-end-at/before-current-tick records do not consume capacity; they remain stored (`crates/nyon-workshop-core/src/command.rs:755-768`). Do not cap the hazard storage vector at128 or evict old records. Owners has no separate declared cap (`crates/nyon-workshop-core/src/model.rs:11-20,143`).

The remaining constants and operational max/max+1 checks belong to later owners: batch128/65536 (4C/4D), catalog/raw pack and recipe limits (4B), history limits (4F), archive16MiB (4G), slots16 (Task6). The existing exact table is `docs/qualification/2026-10-09-zig-workshop-core-inventory.md:122-139`. Declaring constants in 4A is not an execution witness for capacity rejection or atomic preservation.

## Namespace fixtures: exact domain, widths, marker and truncation

The following are **Current source formulas**. Every domain below ends in an actual NUL byte. All scalar byte fields explicitly shown LE use fixed widths; IDs/hashes are raw byte arrays, not their hex JSON spelling. Only state hashing executes in 4A; freeze other preimages for later owners without porting their operations.

| Hash / identity | Preimage in order | Source / execution owner |
| --- | --- | --- |
| StateDigest | `NYON-WORKSHOP-STATE-V1\0 || canonical_state` | `crates/nyon-workshop-core/src/model.rs:218-238`; 4A |
| CatalogHash | `NYON-WORKSHOP-PACK-V1\0 || canonical_pack` | `crates/nyon-workshop-core/src/pack.rs:335-342`; 4B |
| RevisionId | `NYON-WORKSHOP-REVISION-V1\0 || rules:u32LE || catalog:32 || seed:32 || parent:[0] or [1]||parent:32 || tick:u64LE || ordinal:u64LE || batch_length:u64LE || canonical_batch` | `crates/nyon-workshop-core/src/command.rs:850-874`; 4D |
| Created EntityId | first16 of SHA256(`NYON-WORKSHOP-ENTITY-V1\0 || revision:32 || kind:u8 || local:u16LE`) | `crates/nyon-workshop-core/src/command.rs:877-886`; 4D |
| Shipment EntityId | first16 of SHA256(`NYON-WORKSHOP-SHIPMENT-V1\0 || route:16 || dispatch_tick:u64LE`) | `crates/nyon-workshop-core/src/simulation.rs:537-545`; 4E |
| Root BranchId | first16 of SHA256(`NYON-WORKSHOP-ROOT-BRANCH-V1\0 || catalog:32 || seed:32`) | `crates/nyon-workshop-core/src/history.rs:648-654,675-678`; 4F |
| Fork BranchId | first16 of SHA256(`NYON-WORKSHOP-FORK-BRANCH-V1\0 || source:16 || cursor:[0] or [1]||cursor:32 || first_revision:32`) | `crates/nyon-workshop-core/src/history.rs:656-678`; 4F |

Seed-u64 expansion is its eight LE bytes followed by 24 zero bytes, not an integer written into all32 bytes and not hex text (`crates/nyon-workshop-core/src/command.rs:181-185`). Root genesis identities depend on the catalog and seed, while the empty-state bytes do not (`crates/nyon-workshop-core/tests/creator.rs:183-210`; `crates/nyon-workshop-core/src/model.rs:129-163`). Tick events/archive integrity are later owners and already inventoried; this report does not reopen them.

## Existing fixtures and their provenance

**Current source witnesses, unexecuted in this task:**

| Witness | Exact source | Provenance classification |
| --- | --- | --- |
| Pretty built-in core pack input | `assets/workshop/core-pack-v1.json`; included at `crates/nyon-workshop-core/tests/common/mod.rs:8-12` | Committed input fixture; not an independently derived canonical expected byte stream. Hash binds its file below. |
| Built-in pack literal 32-byte hash | `crates/nyon-workshop-core/tests/pack.rs:8-22` | Rust literal oracle, external to future Zig; independent derivation unknown. Round-trip bytes are generated by Rust during test. |
| Exact empty-state JSON literal and 32-byte digest | `crates/nyon-workshop-core/tests/creator.rs:183-196` | Frozen Rust byte/hash literals. The bytes can additionally be hand-derived from model's ordered schema/defaults; the SHA literal has no independently documented derivation in inspected files. |
| Two root branches, seeds `0x46524745` and `0x0badf00d` | `crates/nyon-workshop-core/tests/creator.rs:185-210` | Literal first16 expectations. Root formula is independently inspectable, but actual test values' provenance remains unknown. |
| CreateSystem first revision, entity, post-state, seeds7 and `0x0badf00d` | `crates/nyon-workshop-core/tests/creator.rs:212-276` | Rust literal expected identities/digests; actual batch/state are produced by Rust authority. More than4A; retain as later comparison witnesses. |
| Coordinate extremes/floor distance | `crates/nyon-workshop-core/src/ids.rs:319-330`; `crates/nyon-workshop-core/tests/simulation.rs:129-152` | Literal numeric expectations with elementary independent arithmetic derivation available. Not a byte/hash corpus. |
| Map/inventory insertion-order independence | `crates/nyon-workshop-core/tests/simulation.rs:379-449` | Two Rust-generated canonical outputs compared to each other. Neither is a literal independent byte oracle. Raw world fixture refers to a star not inserted into state (`:382,403-414`), demonstrating an encoding witness rather than a valid authority genesis. |
| Pack collection-order independence | `crates/nyon-workshop-core/tests/pack.rs:65-83` | Rust value mutation, Rust re-encode, Rust decode/hash comparison. Useful metamorphic test; no independent byte/hash reference. |

Focused file listings under core tests/fixtures and tools found Living fixtures/vector tools but no non-Living Workshop V1 corpus or independently authored V1 calculator. The prior inventory reaches the same scoped conclusion at `docs/qualification/2026-10-09-zig-workshop-core-inventory.md:168-194`. This is an inspected-surface absence, not a claim that no independent vector could exist anywhere on disk. No history/archives or broad operation inventory was revisited. No generator was run; no Rust result was relabeled independent.

## Minimum independent corpus to freeze before Zig output

**Proposed derivation process:** record each case's literal inputs, exact expected bytes/value/error, source/spec formula, evidence class, independently authored derivation and reviewer. Bind source/spec hashes and fixture-file hashes. A future standalone calculator may use standard SHA-256 and explicit literal concatenation, but must not invoke/import either Rust or Zig implementations, call their encoders or accept implementation output as goldens. Review generated reference values against separately hand-derived bytes and the existing Rust literals. Hash derivation is Pending here; no such calculator or fixture file was written.

Minimum groups:

1. **Scalar/ID wire and nominality:** each of five fixed-ID types, zero/allFF/nibble-pattern values; exact lengths, malformed lowercase/uppercase/punctuation/Unicode; all distinct-type compile rejection witnesses. Tick0/max and local0/max, JSON values above2^53, range/float/negative errors with consumer boundary named. Explicit byte-order pair whose lexical order differs from little-endian-integer order. Sources: `crates/nyon-workshop-core/src/ids.rs:5-87`.
2. **String/coordinate legality:** catalog0/1/32/33, pack0/1/48/49, name0/1/64/65, description0/512/513; every invalid spacing/control/DEL/non-ASCII class and valid quote/backslash. No normalization. Coordinate±limit, ±(limit+1), i64 extremes; zero, floor diagonals and3-4-5. Sources: `crates/nyon-workshop-core/src/ids.rs:191-247,269-312`.
3. **Canonical state coverage:** empty state plus a coherent compact nonempty graph spanning all ten record kinds and owners. Include both linked-deposit null/value, both boolean values, inventory zero/absent, RGB extremes, negative coordinate, maximal representable integer widths and escaped name in schema-only cases. Each fixture has reviewed exact field order and exact bytes, not merely a round trip. Separate valid-authority examples from raw-schema stress fixtures requiring later rejection. Sources: `crates/nyon-workshop-core/src/model.rs:38-163,187-238`.
4. **Ordering/equality:** reverse/permuted entity, owners and inventory insertion builds must equal the same independently frozen bytes. Test maps keyed with `a`, `a-`, `a0`, `a_` to distinguish ASCII lexical ordering; test equal deep clones with different allocations and changes to every field producing inequality. Zero inventory versus absent is byte-distinct. Sources: `crates/nyon-workshop-core/src/ids.rs:89,140`; `crates/nyon-workshop-core/src/model.rs:38-144`; `crates/nyon-workshop-core/src/simulation.rs:522-535`.
5. **Namespaces:** literal state domain/preimage and digest; revision parent none/some, fork cursor none/some, root with two catalog/seed inputs, created entities for all ten kind bytes/local 0/max, shipment tick0/max. Freeze exact raw preimages and full SHA output before recording first16 results. Seed expansion, widths, domain NUL and first-versus-last truncation must be independently checkable. Execute only 4A state hashes now; defer other production functions to their owners using the table above.
6. **Validation/capacity ownership:** freeze constants and raw invalid examples (cross-kind duplicate, missing references, bad orbits, unsuitable owner, invalid catalog). Do not assert4A is a validator. Future4D/4E witnesses exercise max/max+1 and checked overflow without eviction/partial publication; hazard historical-map count must be distinguished from active/scheduled count. Sources: `crates/nyon-workshop-core/src/model.rs:11-20,165-185`; `crates/nyon-workshop-core/src/command.rs:703-717,755-768`.
7. **Allocation failure:** fail each allocation point for validated string creation/clone, map insertion/key ownership, deep state clone and canonical encode/digest. Assert no leaks/dangling references/partial installed result; source content, equality and canonical bytes remain intact. Free caller input immediately after success to prove independent ownership. Partial clone/parse allocations unwind deterministically; transferred values have one destruction owner. Mutations needing allocation construct a detached replacement before swapping. This is Proposed Zig evidence motivated by Rust owned boxes/maps and detached state cloning (`crates/nyon-workshop-core/src/ids.rs:91,142`; `crates/nyon-workshop-core/src/model.rs:129-144`; `crates/nyon-workshop-core/src/command.rs:304-328`), not proof Rust exposes allocator failures as typed rejections.

### Hand-derivable literal byte candidates

These are **source-contract-derived manual candidates**, not newly generated implementation outputs. The rows show derivation rather than asserting a SHA calculation was performed.

| Input | Exact expected bytes / reasoning |
| --- | --- |
| 16 identity bytes `00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f` | JSON `"000102030405060708090a0b0c0d0e0f"`: high nibble then low nibble per byte (`crates/nyon-workshop-core/src/ids.rs:33-39`). |
| 32 identity bytes, two consecutive copies of that 16-byte pattern | JSON `"000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f"`; same direct nibble derivation. |
| Tick max / local max | ASCII JSON `18446744073709551615` / `65535`, no quotes; transparent unsigned wrappers (`crates/nyon-workshop-core/src/ids.rs:8-9,86-87`). |
| Point(-1,2) | ASCII JSON `{"x":-1,"y":2}` from ordered fields `x,y` (`crates/nyon-workshop-core/src/ids.rs:219-224`). |
| Rules version 1 / local 0x1234 / tick 0x0102030405060708 | Preimage hex `01 00 00 00` / `34 12` / `08 07 06 05 04 03 02 01`, directly applying u32/u16/u64 LE (`crates/nyon-workshop-core/src/command.rs:860,870-872,882`). |
| Seed 0x46524745 | First 8 bytes `45 47 52 46 00 00 00 00`, followed by 24 zero bytes (`crates/nyon-workshop-core/src/command.rs:181-184`). |
| State domain | ASCII `NYON-WORKSHOP-STATE-V1` followed by byte `00`, never literal backslash+zero (`crates/nyon-workshop-core/src/model.rs:219`). |
| Empty state | The literal below follows the exact state declaration/default order (`crates/nyon-workshop-core/src/model.rs:131-163`), with a separately existing Rust byte literal at `crates/nyon-workshop-core/tests/creator.rs:188`. Its SHA-256 value still needs independent derivation. |

```json
{"tick":0,"factions":{},"systems":{},"stars":{},"worlds":{},"lanes":{},"deposits":{},"industries":{},"routes":{},"shipments":{},"hazards":{},"owners":{}}
```

The code-block display newline is not part of expected bytes. State hashing uses only the displayed JSON bytes prefixed by the domain+NUL. Existing Rust expected digest bytes are `[206,75,30,39,46,141,19,137,28,79,142,241,117,199,155,180,106,232,251,94,188,210,50,65,112,26,250,183,236,101,79,13]` (`crates/nyon-workshop-core/tests/creator.rs:190-195`). This value is a **Rust literal comparison witness**, not an independently calculated result in this report.

Manual geometry derivation candidates: floor sqrt(2)=1, floor sqrt(8)=2, sqrt(25)=5, floor sqrt(50)=7; opposite corners use squared distance `2 × 268435456²` and the reviewable condition `379625062² ≤ 2 × 268435456² < 379625063²`, matching literal source expectations (`crates/nyon-workshop-core/tests/simulation.rs:135-145`). These are arithmetic formulas suitable for independent review; route travel mechanics remain 4E.

## Decision gaps and exit evidence

1. **Pending independent derivation:** every hash expected value, nonempty all-record canonical bytes, escaping cases and namespace preimages beyond the small manual candidates still need independent authoring/review/freeze. Existing Rust goldens alone do not close migration-plan `:152-154`.
2. **Proposed source-contract authority:** accepted design lacks exhaustive canonical state wire/hash preimages. Review whether to freeze a concise compatibility supplement alongside vectors. Do not label source-derived fields as words already present in the accepted spec; do not change them to make a calculator simpler.
3. **Proposed raw-state interface:** 4A should provide encoding/copy primitives without public install. Keep valid-authority construction/reconstruction with 4D/4G; whole-state validator availability is absent today, not silently assumed.
4. **Proposed container/allocator API:** choose a deterministic sorted representation without unconditional hazard128 storage capacity; define insert key/value ownership on success/failure, clone unwinding and encoding result ownership before dependent work. Freeze semantic APIs rather than a Rust-specific BTree allocation strategy.
5. **Unknown raw-decoder quirks:** duplicate keys, scalar/ref unknown-field handling beyond explicit annotations, escape-normalized identifiers and negative-zero numeric parsing need named consumer characterization if4A includes decoders. These cannot be inferred from pack strict parsing or claimed tested here.

Exit evidence for future 4A: reviewed contract+source bindings; reviewed independently derived corpus; foundation Debug/ReleaseSafe tests for literal bytes/digests, geometry, equality/order and allocator failures; separate Rust differential witnesses; reviewed intended diff. Every one is **Pending** here. No full Forge, catalog validation, command execution, history/archive, live native/browser, performance or platform claim follows from this preparation.

## Inspected source bindings

SHA-256 values below are file fingerprints obtained by `shasum -a 256`, not Workshop-domain reference hashes. They bind cited source text to this inspection; reread changed sources before implementation.

| File | SHA-256 |
| --- | --- |
| `AGENTS.md` | `39da35b681a4a2d4866377ee0f090d22bb1c1266f49f0857e5ac2cbd3c403cb9` |
| `docs/superpowers/plans/2026-10-09-nyon-zig-migration.md` | `0a4e9063b7d8271686f94b68001c3e0258c97923ddf6b81088f838956d3c7545` |
| `docs/qualification/2026-10-09-zig-workshop-core-inventory.md` | `aeacfced4a2307ba0110234e3312a10ea3520dbba8072dd6a5d4b28c1efe75ca` |
| `docs/superpowers/specs/2026-09-02-nyon-v2-design.md` | `f386a8c074ad87c431a1066f7a55a8a96270c84692554b9c4a919c2d543e195d` |
| `docs/superpowers/specs/2026-09-04-nyon-workshop-library-addendum.md` | `784781b8f7c23eabbe3dbed0c2fd90fbf1a7a9bd7701802bc5ced759f3c898ae` |
| `docs/superpowers/specs/2026-09-08-workshop-library-route-design.md` | `ea0c47236232e2b36ffa016ef7c323c1cd438082108efc4dbcf602b65dfa42fc` |
| `crates/nyon-workshop-core/src/ids.rs` | `489895dc85c97f524a3727677fb63be68e3678897e811a48c7013789af82637d` |
| `crates/nyon-workshop-core/src/model.rs` | `e69a03a72cd278e3159858a281ef71526b3683218a3fa5d7955ce4814f77a6c9` |
| `crates/nyon-workshop-core/src/lib.rs` | `358dcc4f59f8c3395b0c330d33e4fd0cf8c265f77ce6802467048f6f372e5f1a` |
| `crates/nyon-workshop-core/src/pack.rs` | `acc1c3d19b80a051461e0628bde62f66dfafca4cd846a874a36b829b7fe5bb3c` |
| `crates/nyon-workshop-core/src/command.rs` | `576716226d01e5f23855456e27f1d88aa1d39c2192612a26114a59725024f782` |
| `crates/nyon-workshop-core/src/history.rs` | `c5cd94e8c196a4bd0bcda637184fd597d04d69fbc565a6dc4c2dda85c5903080` |
| `crates/nyon-workshop-core/src/simulation.rs` | `c4076de2542d2688546da88d33eedde1c85e35b9407684b7651e040c5bbc819b` |
| `crates/nyon-workshop-core/tests/creator.rs` | `06c8a488c7b301903d955856a01b6b371d8de1899a09f7d1fedca0002d26cc3f` |
| `crates/nyon-workshop-core/tests/pack.rs` | `619cf3390a01077cc9fcce0d590507dffb49905a92b036d29f40e8fba2769e0a` |
| `crates/nyon-workshop-core/tests/simulation.rs` | `2848258cc2872d924126c4c76aa139aa2430ac2f699a4da5fce39da0f74715d1` |
| `crates/nyon-workshop-core/tests/common/mod.rs` | `80c3f6af89c54395f53c110d592e9aca820b08d5e7ce5eca9025dd689490f1dd` |
| `assets/workshop/core-pack-v1.json` | `5565bd115dcc53858d47e9e712c270d5ad460a4bd441625ff9ffaf59d65a6db5` |

Gaps: independent V1 corpus and calculator absent on the inspected surface; raw-state validation API absent; hashes/new nonempty fixtures need independent derivation; all Zig/runtime verification Pending.
