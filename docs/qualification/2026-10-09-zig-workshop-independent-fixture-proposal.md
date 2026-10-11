# Task 4A–B independent fixture proposal

2026-10-09. **Current:** manual byte derivations and ordinary SHA-256 calculations documented below, against fingerprinted source. **Proposed:** corpus cases and admission/ownership tests. **Pending:** independent review, immutable fixture freeze, Workshop Zig implementation and runtime parity. This is evidence preparation, not a completed 4A or 4B slice.

Only this report was written. No source, existing document, fixture, tool, dependency, build/profile or commit changed. No Rust/Zig runtime encoder, source generator or broad test ran. The earlier [foundation report](task-4a-foundation-vector-seams.md) was read and remains unchanged. Other production work remains with its existing owners.

## Provenance and derivation rule

The accepted normative design is `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:7,176-235,363-390`. It establishes typed identities, checked integer geometry, hard caps, strict pack input and sorted minified canonical pack hashing. Exact state-record field order/domain and exact pack variant declaration order are **source compatibility supplements**, respectively `crates/nyon-workshop-core/src/model.rs:38-163,187-238` and `crates/nyon-workshop-core/src/pack.rs:24-57,144-161,190-203,316-342`. The design does not enumerate all these exact bytes. The Library addendum preserves V1 formats and authority (`docs/superpowers/specs/2026-09-04-nyon-workshop-library-addendum.md:3,8,14-18`); platform admission is outside this proposal. Plans and existing qualification reports are targets/evidence indexes, not new wire-format authority.

Method: choose literal semantic values; apply the cited declarations, validated scalar spelling and map/array ordering by hand; type the expected minified ASCII JSON directly into this report. The calculator only hashes those literal byte strings. It never serializes objects to obtain expected bytes and never imports/calls NYON code. Thus the calculation is independent of both implementations, while the exact byte contract is source-contract-derived where the accepted spec is incomplete. Existing Rust literal comparisons remain a distinct evidence class; matching does not retroactively establish those literals' original provenance.

## Literal 4A cases

Each inline JSON value is exact ASCII bytes without the Markdown delimiters or a trailing newline.

| Case | Input / exact expected output | Derivation |
| --- | --- | --- |
| I16-nibbles | Raw hex `000102030405060708090a0b0c0d0e0f` → JSON `"000102030405060708090a0b0c0d0e0f"` for EntityId or BranchId | Per-byte high then low lowercase nibble, `crates/nyon-workshop-core/src/ids.rs:26-75`. Nominal types remain distinct. |
| I32-nibbles | Raw hex `000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f` → JSON `"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"` for RevisionId, CatalogHash or StateDigest | Same formula, 32 raw bytes/64 decoded characters, `crates/nyon-workshop-core/src/ids.rs:71-75`. |
| I16-zero / FF | `"00000000000000000000000000000000"` / `"ffffffffffffffffffffffffffffffff"` | Neither an invalid sentinel nor an uppercase representation, `crates/nyon-workshop-core/src/ids.rs:43-65`. Repeat at 64 characters for 32-byte wrappers. |
| I16-order | Raw hex A=`00010000000000000000000000000000`, B=`01000000000000000000000000000000`; A < B | Byte0 decides; little-endian-integer interpretation would invert this relation. Derived array `Ord` at `crates/nyon-workshop-core/src/ids.rs:14-18`. |
| U64 / U16 | Tick `0`, `9007199254740993`, `18446744073709551615`; Local `0`, `65535` | Transparent numeric wrappers, no quotes/floating intermediate, `crates/nyon-workshop-core/src/ids.rs:5-9,85-87`. |
| Coord limits | `{"x":-134217728,"y":134217728}` | `131072 × 1024 = 134217728`, inclusive bound and ordered x,y, `crates/nyon-workshop-core/src/ids.rs:3,191-231`; accepted geometry `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:203`. |
| Coord negative/zero | `{"x":-1,"y":0}` | Signed decimal scalar and field order, `crates/nyon-workshop-core/src/ids.rs:191-224`. |
| Ref local / existing | `{"kind":"local","id":65535}` / `{"kind":"existing","id":"000102030405060708090a0b0c0d0e0f"}` | Tagged kind/content schema, `crates/nyon-workshop-core/src/ids.rs:250-255`; raw ref annotation does not promise global strict-document validation. |
| Name escapes | Semantic ASCII bytes `51 22 5c` (Q, quote, backslash) → JSON `"Q\"\\"` | Printable-name rule admits both bytes, `crates/nyon-workshop-core/src/ids.rs:285-291`; JSON quote/backslash escaping is part of `model.rs:187-188` canonical serializer compatibility. Review the literal escape bytes before freeze. |
| Inventory ordering/zero | Insert `a_→18446744073709551615`, `a0→9007199254740993`, `a-→1`, `a→0` in any order → `{"a":0,"a-":1,"a0":9007199254740993,"a_":18446744073709551615}` | Catalog lexical bytes: prefix before extension, then `-`(2d) < `0`(30) < `_`(5f), `crates/nyon-workshop-core/src/ids.rs:89,277-283`; ordered inventory `crates/nyon-workshop-core/src/model.rs:70`. Zero is retained by subtraction `crates/nyon-workshop-core/src/simulation.rs:522-535`. |

Negative fixtures: fixed IDs with wrong lengths 31/33/63/65, uppercase A, g, slash, non-ASCII; unsigned values -1, max+1,1.5, quoted numbers; coordinates±134217729 and i64 extrema; catalog 0/33 bytes, forbidden dot/uppercase/leading digit; pack 49 bytes, name 0/65, edge/doubled spaces, control 00/1f/7f/non-ASCII; description 513 bytes. Exact scalar/domain errors derive from `crates/nyon-workshop-core/src/ids.rs:43-65,195-215,257-295`. Consumer parser error mapping, escaped ID acceptance and numeric -0 handling remain **Pending characterization**; do not invent a stricter standalone raw-state decoder.

### Integer floor-distance witnesses

Use checked integer squared differences and the condition `r² ≤ square < (r+1)²`, not a floating square root (`crates/nyon-workshop-core/src/ids.rs:234-247,297-312`).

| Points | Square | Expected floor distance | Hand proof |
| --- | ---: | ---: | --- |
| (0,0),(0,0) | 0 | 0 | 0²≤0<1² |
| (0,0),(1,1) | 2 | 1 | 1≤2<4 |
| (0,0),(2,2) | 8 | 2 | 4≤8<9 |
| (0,0),(3,4) | 25 | 5 | 25≤25<36 |
| (0,0),(5,5) | 50 | 7 | 49≤50<64 |
| (-134217728,-134217728),(134217728,134217728) | `2 × 268435456²` | 379625062 | Exact square/neighbor calculation recorded below. |

Symmetry swaps endpoints without changing the square. Legal coordinates cannot produce a checked i128 overflow: each difference's absolute value is at most 268435456; the sum is at most 2×268435456². Overflow injection through invalid internal representations is a separate test, not a valid-coordinate fixture. Existing source constants independently agree at `crates/nyon-workshop-core/tests/simulation.rs:135-151`; no production distance routine ran here.

## Exact state bytes

`STATE_EMPTY` is hand-derived from ordered state fields and empty defaults (`crates/nyon-workshop-core/src/model.rs:131-163`).

```STATE_EMPTY
{"tick":0,"factions":{},"systems":{},"stars":{},"worlds":{},"lanes":{},"deposits":{},"industries":{},"routes":{},"shipments":{},"hazards":{},"owners":{}}
```

`STATE_ALL_RECORDS_RAW` is a deliberately **raw-schema stress case**, not a valid authority genesis or replayable save. It covers all ten record kinds plus owners, nested RGB/point/inventory, both signed coordinate limits, u32/u64 maximums, zero inventory, strings with escapes, booleans and a present optional reference. IDs are literal16-byte values with only their last byte nonzero. Record/state field order comes directly from `crates/nyon-workshop-core/src/model.rs:38-144`; collection keys are sorted bytewise, not field-name alphabetical.

```STATE_ALL_RECORDS_RAW
{"tick":18446744073709551615,"factions":{"00000000000000000000000000000001":{"name":"Q\"\\","color_rgb":[0,127,255]}},"systems":{"00000000000000000000000000000002":{"name":"S","position":{"x":-134217728,"y":134217728}}},"stars":{"00000000000000000000000000000003":{"system":"00000000000000000000000000000002","name":"T","archetype_id":"yellow-dwarf"}},"worlds":{"00000000000000000000000000000004":{"system":"00000000000000000000000000000002","primary":"00000000000000000000000000000003","name":"W","archetype_id":"rocky-world","orbit_radius_milli_au":4294967295,"orbit_period_ticks":18446744073709551615,"phase_millidegrees":4294967295,"inventory":{"a":0,"a-":1,"a0":9007199254740993,"a_":18446744073709551615}}},"lanes":{"00000000000000000000000000000005":{"a":"00000000000000000000000000000002","b":"00000000000000000000000000000002","distance_units":18446744073709551615}},"deposits":{"00000000000000000000000000000006":{"world":"00000000000000000000000000000004","resource_id":"ore","reserve_units":18446744073709551615}},"industries":{"00000000000000000000000000000007":{"world":"00000000000000000000000000000004","definition_id":"extractor","linked_deposit":"00000000000000000000000000000006","enabled":true}},"routes":{"00000000000000000000000000000008":{"source":"00000000000000000000000000000004","destination":"00000000000000000000000000000004","resource_id":"ore","batch_units":18446744073709551615}},"shipments":{"00000000000000000000000000000009":{"route":"00000000000000000000000000000008","source":"00000000000000000000000000000004","destination":"00000000000000000000000000000004","resource_id":"ore","units":18446744073709551615,"dispatched_tick":0,"arrival_tick":18446744073709551615}},"hazards":{"0000000000000000000000000000000a":{"lane":"00000000000000000000000000000005","hazard_id":"ion-storm","start_tick":0,"duration_ticks":18446744073709551615,"cancelled":true}},"owners":{"00000000000000000000000000000004":"00000000000000000000000000000001"}}
```

The intentionally invalid authority values include phase ≥360000, a lane joining a system to itself, a route joining a world to itself, reserve/batch quantities above 1e9, unsupported inventory resources and hazard duration exceeding the catalog maximum. Later command admission rejects these constraints at `crates/nyon-workshop-core/src/command.rs:420-424,442-476,540-581,616-650`. Model serialization/digest is independent of that admission: current public records and canonical functions have no standalone whole-state validator (`crates/nyon-workshop-core/src/model.rs:129-144,165-238`). A hash of this case proves exact encoding only. Do not install it as valid authority.

Two additional full-record literals exercise optional null and false without altering record order:

```INDUSTRY_NULL_FALSE
{"world":"00000000000000000000000000000004","definition_id":"solar-array","linked_deposit":null,"enabled":false}
```

```HAZARD_NOT_CANCELLED
{"lane":"00000000000000000000000000000005","hazard_id":"ion-storm","start_tick":10,"duration_ticks":2,"cancelled":false}
```

Freeze later variants with one field removed/reordered/renamed, inventory zero removed, every map insertion reversed and a second entity key exercising the lexical/endian-order distinction. Each should compare to literal expected bytes or a named admission error; round-trip equality alone is insufficient. Structural equality must compare content, not allocation addresses; allocation-failure, clone independence and caller-input lifetime tests remain Proposed (foundation report:24-29,78-92).

## Exact 4B pack bytes

The root declaration order is `kind,format_version,pack_id,pack_version,title,star_archetypes,world_archetypes,resources,industry_definitions,hazard_definitions` (`crates/nyon-workshop-core/src/pack.rs:190-203`). Simple definitions are `id,name,description`; quantity is `resource_id,quantity`; tagged industries emit kind followed by each variant's declaration order; hazard definitions follow their declaration order (`crates/nyon-workshop-core/src/pack.rs:24-57,144-161`). Definitions sort by ID and solar/processor quantities sort by resource ID (`:128-139,322-333`). Unknown fields are not added, descriptions are retained even when empty, and declarations are not alphabetically reordered.

`PACK_EMPTY` is a compact structurally valid candidate: current validation has no minimum collection cardinality, requires kind/format, a valid pack ID/name and nonzero pack_version, then maximums/definitions/references (`crates/nyon-workshop-core/src/pack.rs:356-445`). Proposed expected admission is success; this inference has not been exercised by a Rust or Zig runtime here.

```PACK_EMPTY
{"kind":"NYON_WORKSHOP_DATA","format_version":1,"pack_id":"a","pack_version":1,"title":"A","star_archetypes":[],"world_archetypes":[],"resources":[],"industry_definitions":[],"hazard_definitions":[]}
```

`PACK_CORE_MANUAL` copies semantic built-in input values from `assets/workshop/core-pack-v1.json:1-59`, then **manually** applies source sorting and declaration order: resources alloy,energy,ore; industries extractor,foundry,solar-array; foundry inputs energy,ore. It exercises all three industry variants, two simple-definition classes, resource arrays, recipes and hazard ratio fields. No runtime codec produced this literal.

```PACK_CORE_MANUAL
{"kind":"NYON_WORKSHOP_DATA","format_version":1,"pack_id":"nyon.core","pack_version":1,"title":"NYON Core","star_archetypes":[{"id":"yellow-dwarf","name":"Yellow Dwarf","description":"A stable main-sequence star."}],"world_archetypes":[{"id":"rocky-world","name":"Rocky World","description":"A solid world suitable for industry."}],"resources":[{"id":"alloy","name":"Alloy","description":"Refined structural material."},{"id":"energy","name":"Energy","description":"Electrical energy for industry."},{"id":"ore","name":"Ore","description":"Unrefined mineral feedstock."}],"industry_definitions":[{"kind":"extractor","id":"extractor","name":"Extractor","description":"Uses energy to extract linked ore reserves.","duration_ticks":1,"energy_input":{"resource_id":"energy","quantity":1},"output":{"resource_id":"ore","quantity":2}},{"kind":"processor","id":"foundry","name":"Foundry","description":"Converts energy and ore into alloy.","duration_ticks":1,"inputs":[{"resource_id":"energy","quantity":2},{"resource_id":"ore","quantity":3}],"outputs":[{"resource_id":"alloy","quantity":1}]},{"kind":"solar","id":"solar-array","name":"Solar Array","description":"Produces four energy units per tick.","duration_ticks":1,"outputs":[{"resource_id":"energy","quantity":4}]}],"hazard_definitions":[{"id":"ion-storm","name":"Ion Storm","description":"Halves route capacity on its lane using integer floor division.","duration_ticks":36000,"route_capacity_numerator":1,"route_capacity_denominator":2}]}
```

Proposed input variants: reverse all definition arrays, reverse processor inputs and solar outputs where multi-entry; include whitespace/escaped ASCII input spelling. Expected export is the manually canonical bytes, not the input spelling. Import need not be canonical; export must be canonical (`crates/nyon-workshop-core/src/pack.rs:316-354`; accepted `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:388`). Input cardinalities 32/64/32/64/32 and max+1, quantities 1/1e9 versus0/1e9+1, durations 1/36000 versus0/36001, duplicate quantity resource, missing resource and duplicate IDs across definition kinds remain Proposed strict admission fixtures (`crates/nyon-workshop-core/src/pack.rs:371-443,471-499`).

## Strict input, duplicate/dangling and capacity distinction

| Corpus proposal | Expected boundary |
| --- | --- |
| Pack prefix BOM bytes efbbbf; invalid UTF8 ff; raw size 1048577 before JSON parse | Bom / InvalidUtf8 / PackTooLarge respectively, `crates/nyon-workshop-core/src/pack.rs:503-513`; accepted 1MiB cap `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:372-373`. |
| Pack root duplicate `{"kind":"NYON_WORKSHOP_DATA","kind":"NYON_WORKSHOP_DATA"}`; nested duplicate `{"x":{"id":"a","id":"b"}}`; escape-equivalent duplicate `{"id":"a","\u0069d":"b"}` | DuplicateKey at strict parser before wire admission, decoded key equality at `crates/nyon-workshop-core/src/pack.rs:636-657`. These deliberately incomplete root shapes still reach duplicate detection first. Runtime result Pending. |
| Pack floats `1.0`/`1e0` inside a field; trailing second JSON value; unknown field at root/variant | Float / Json / Json according to strict visitor, trailing-value check and deny_unknown_fields at `crates/nyon-workshop-core/src/pack.rs:610-614,652-665,31-32,190-192`. Numeric negative-zero mapping requires separate characterization. |
| Pack depth 32/33 with braces/brackets inside quoted strings | Depth counts actual nesting, not punctuation inside strings; depth 33 → DepthExceeded before structural parsing (`crates/nyon-workshop-core/src/pack.rs:514-541`). Freeze concrete complete input/error order later, rather than claim every depth 32 malformed document is accepted. |
| Duplicate same key in a raw-state JSON object | Not an ordered map's duplicate resident entry; standalone raw-state serde behavior remains unknown here. Do not infer strict pack behavior. |
| Same EntityId in faction and system maps; dangling world.primary/system/owner IDs; invalid catalog references | Hash/serialize can still operate on raw records; entity_kind returns first matching collection. Authority admission must reject invalid installation/replay. `crates/nyon-workshop-core/src/model.rs:165-238`; creator collision `crates/nyon-workshop-core/src/command.rs:703-717`; foundation report:82-92. |
| Hazard stored historical population >128 while scheduled/active population ≤128 | Must remain representable/persisted; no old-record eviction. Cap concerns scheduled or active, accepted `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:226,235`; current count predicate `crates/nyon-workshop-core/src/command.rs:755-768`. |

Concrete hazard-count cases at authoritative tick 10, same literal record fields except start/duration/cancelled: (start0,duration10,false) contributes 0 because end==tick; (0,11,false) contributes 1; (20,1,false) contributes 1 although future; (20,1,true) contributes 0. 128 qualifying records plus one historical expired record is 129 stored records but counted 128; the next qualifying addition exceeds 128. Distinct IDs and full canonical byte fixture still need authoring/review. Start=18446744073709551615,duration1 is checked end overflow and yields Arithmetic even if cancelled, because the source computes end before the cancelled check (`crates/nyon-workshop-core/src/command.rs:755-765`). Execution and atomic-preservation witnesses belong 4D; 4A container design must not impose a total 128-record limit.

## SHA-256 calculation protocol and actual evidence

Algorithm: standard `hashlib.sha256(domain_bytes + payload_literal_bytes).hexdigest()`; no line terminator, BOM, byte-length prefix or trailing bytes. State uses ASCII `NYON-WORKSHOP-STATE-V1` plus00 (`crates/nyon-workshop-core/src/model.rs:218-238`). Pack uses ASCII `NYON-WORKSHOP-PACK-V1` plus00 (`crates/nyon-workshop-core/src/pack.rs:335-342`; normative `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:388`). Every displayed JSON block is minified ASCII, read as literal bytes. Hash calculation results, exact input hex and recorded execution follow below; these are preparation evidence, not implementation test results.

## Source fingerprints and freeze steps

Files were fingerprinted with `shasum -a 256`; hashes bind this inspection and are not domain-separated Workshop hashes.

| Source | SHA-256 |
| --- | --- |
| Accepted `docs/superpowers/specs/2026-09-02-nyon-v2-design.md` | `f386a8c074ad87c431a1066f7a55a8a96270c84692554b9c4a919c2d543e195d` |
| Accepted `docs/superpowers/specs/2026-09-04-nyon-workshop-library-addendum.md` | `784781b8f7c23eabbe3dbed0c2fd90fbf1a7a9bd7701802bc5ced759f3c898ae` |
| `crates/nyon-workshop-core/src/ids.rs` | `489895dc85c97f524a3727677fb63be68e3678897e811a48c7013789af82637d` |
| `crates/nyon-workshop-core/src/model.rs` | `e69a03a72cd278e3159858a281ef71526b3683218a3fa5d7955ce4814f77a6c9` |
| `crates/nyon-workshop-core/src/pack.rs` | `acc1c3d19b80a051461e0628bde62f66dfafca4cd846a874a36b829b7fe5bb3c` |
| `crates/nyon-workshop-core/src/command.rs` | `576716226d01e5f23855456e27f1d88aa1d39c2192612a26114a59725024f782` |
| `crates/nyon-workshop-core/src/simulation.rs` | `c4076de2542d2688546da88d33eedde1c85e35b9407684b7651e040c5bbc819b` |
| Semantic input `assets/workshop/core-pack-v1.json` | `5565bd115dcc53858d47e9e712c270d5ad460a4bd441625ff9ffaf59d65a6db5` |
| Rust external literal witnesses `crates/nyon-workshop-core/tests/creator.rs` | `06c8a488c7b301903d955856a01b6b371d8de1899a09f7d1fedca0002d26cc3f` |
| Rust external literal witness `crates/nyon-workshop-core/tests/pack.rs` | `619cf3390a01077cc9fcce0d590507dffb49905a92b036d29f40e8fba2769e0a` |
| Earlier foundation report | `5ee6c1cadf997c3e730e53a2e08837718b6e5de8e8e9a46b643a89be3d05c954` |

Next steps: independently review literal bytes/escaping/order and the source/spec supplement; fill the Pending ingress/error/multiple-key/permutation/historical-population cases; independently review calculator inputs/results; assign immutable case IDs and provenance entries, write and checksum fixtures only after that review; then dispatch 4A/4B implementations and retain separate Rust differential evidence. No fixture freeze, Workshop source dispatch or slice completion is authorized by this proposal itself.

Gaps: raw-state ingress quirks; valid nonempty genesis graph separate from raw stress case; complete exact-byte permutations/negative corpus; admitted 129-record historical-hazard fixture; allocator failure tests; independent review/freeze and every Rust/Zig runtime result remain Pending.

## Recorded literal-input calculations

Executed `python3` standard-library calculation in the canonical checkout; command completed with exit 0. It read only the four named literal blocks above, constructed the two explicitly stated domains and hashed concatenated bytes. It used no JSON encoder, production code, source generator or fixture writer. Only this proposal report received the calculations.

The following read-only recheck command was also executed verbatim from `/Users/donaldfilimon/dev/active/NYON`. It confirmed each recorded complete input hex equals the independently typed literal payload plus its domain. All four rows printed the byte lengths/digests in the table below. The command completed with exit 0. Source-fingerprint commands `shasum -a 256` also completed with exit 0; neither is an implementation gate.

```sh
python3 - <<'PY'
from pathlib import Path
import hashlib
import re
report = Path('.superpowers/sdd/2026-10-09-nyon-zig-migration/task-4a-independent-fixture-proposal.md').read_bytes()
for name, domain in [('STATE_EMPTY', b'NYON-WORKSHOP-STATE-V1\0'), ('STATE_ALL_RECORDS_RAW', b'NYON-WORKSHOP-STATE-V1\0'), ('PACK_EMPTY', b'NYON-WORKSHOP-PACK-V1\0'), ('PACK_CORE_MANUAL', b'NYON-WORKSHOP-PACK-V1\0')]:
    payload = report.split(b'```' + name.encode() + b'\n', 1)[1].split(b'\n```', 1)[0]
    section = report.split(b'### Exact ' + name.encode() + b' SHA-256 input hex\n', 1)[1]
    hex_text = section.split(b'```text\n', 1)[1].split(b'\n```', 1)[0]
    preimage = bytes.fromhex(hex_text.decode('ascii'))
    assert preimage == domain + payload
    print(name, len(payload), len(preimage), hashlib.sha256(preimage).hexdigest())
PY
```

| Literal | Payload bytes | Complete input bytes | Domain hex including terminal 00 | SHA-256 lowercase hex |
| --- | ---: | ---: | --- | --- |
| STATE_EMPTY | 153 | 176 | `4e594f4e2d574f524b53484f502d53544154452d563100` | `ce4b1e272e8d13891c4f8ef175c79bb46ae8fb5ebcd23241701afab7ec654f0d` |
| STATE_ALL_RECORDS_RAW | 1970 | 1993 | `4e594f4e2d574f524b53484f502d53544154452d563100` | `f0c1dc0c71db4253e4f3ba8b31b5f9323efe8b5d52565d46dcd0e9a643fd8da5` |
| PACK_EMPTY | 199 | 221 | `4e594f4e2d574f524b53484f502d5041434b2d563100` | `e5275e76792449c78553244bf536b6fa579140f19a7b7b9c2a5c24bf49239437` |
| PACK_CORE_MANUAL | 1490 | 1512 | `4e594f4e2d574f524b53484f502d5041434b2d563100` | `11e68b4a4b290f6bf14126f8ccf3378e1ec332962c697d6f958e76fed3ef4842` |

Independent calculation comparison: STATE_EMPTY matches the existing literal at `crates/nyon-workshop-core/tests/creator.rs:190-195`: **True**. PACK_CORE_MANUAL matches the existing literal at `crates/nyon-workshop-core/tests/pack.rs:11-15`: **True**. This confirms agreement for these separately hand-derived inputs; the original Rust literals' provenance remains unknown. No comparison was made to Rust runtime-generated bytes.

Exact extreme-distance inequality, using Python arbitrary-precision integer multiplication only: `144115187698503844 <= 144115188075855872 < 144115188457753969`. Therefore floor distance is 379625062. No NYON geometry implementation ran.

### Exact STATE_EMPTY SHA-256 input hex

Concatenate these hex lines without whitespace; they encode the complete domain+NUL+literal JSON input.

```text
4e594f4e2d574f524b53484f502d53544154452d5631007b227469636b223a302c2266616374696f6e73223a7b7d2c22
73797374656d73223a7b7d2c227374617273223a7b7d2c22776f726c6473223a7b7d2c226c616e6573223a7b7d2c2264
65706f73697473223a7b7d2c22696e6475737472696573223a7b7d2c22726f75746573223a7b7d2c22736869706d656e
7473223a7b7d2c2268617a61726473223a7b7d2c226f776e657273223a7b7d7d
```

### Exact STATE_ALL_RECORDS_RAW SHA-256 input hex

Concatenate these hex lines without whitespace; they encode the complete domain+NUL+literal JSON input.

```text
4e594f4e2d574f524b53484f502d53544154452d5631007b227469636b223a3138343436373434303733373039353531
3631352c2266616374696f6e73223a7b2230303030303030303030303030303030303030303030303030303030303030
31223a7b226e616d65223a22515c225c5c222c22636f6c6f725f726762223a5b302c3132372c3235355d7d7d2c227379
7374656d73223a7b223030303030303030303030303030303030303030303030303030303030303032223a7b226e616d
65223a2253222c22706f736974696f6e223a7b2278223a2d3133343231373732382c2279223a3133343231373732387d
7d7d2c227374617273223a7b223030303030303030303030303030303030303030303030303030303030303033223a7b
2273797374656d223a223030303030303030303030303030303030303030303030303030303030303032222c226e616d
65223a2254222c226172636865747970655f6964223a2279656c6c6f772d6477617266227d7d2c22776f726c6473223a
7b223030303030303030303030303030303030303030303030303030303030303034223a7b2273797374656d223a2230
30303030303030303030303030303030303030303030303030303030303032222c227072696d617279223a2230303030
30303030303030303030303030303030303030303030303030303033222c226e616d65223a2257222c22617263686574
7970655f6964223a22726f636b792d776f726c64222c226f726269745f7261646975735f6d696c6c695f6175223a3432
39343936373239352c226f726269745f706572696f645f7469636b73223a313834343637343430373337303935353136
31352c2270686173655f6d696c6c6964656772656573223a343239343936373239352c22696e76656e746f7279223a7b
2261223a302c22612d223a312c226130223a393030373139393235343734303939332c22615f223a3138343436373434
3037333730393535313631357d7d7d2c226c616e6573223a7b2230303030303030303030303030303030303030303030
30303030303030303035223a7b2261223a22303030303030303030303030303030303030303030303030303030303030
3032222c2262223a223030303030303030303030303030303030303030303030303030303030303032222c2264697374
616e63655f756e697473223a31383434363734343037333730393535313631357d7d2c226465706f73697473223a7b22
3030303030303030303030303030303030303030303030303030303030303036223a7b22776f726c64223a2230303030
30303030303030303030303030303030303030303030303030303034222c227265736f757263655f6964223a226f7265
222c22726573657276655f756e697473223a31383434363734343037333730393535313631357d7d2c22696e64757374
72696573223a7b223030303030303030303030303030303030303030303030303030303030303037223a7b22776f726c
64223a223030303030303030303030303030303030303030303030303030303030303034222c22646566696e6974696f
6e5f6964223a22657874726163746f72222c226c696e6b65645f6465706f736974223a22303030303030303030303030
3030303030303030303030303030303030303036222c22656e61626c6564223a747275657d7d2c22726f75746573223a
7b223030303030303030303030303030303030303030303030303030303030303038223a7b22736f75726365223a2230
30303030303030303030303030303030303030303030303030303030303034222c2264657374696e6174696f6e223a22
3030303030303030303030303030303030303030303030303030303030303034222c227265736f757263655f6964223a
226f7265222c2262617463685f756e697473223a31383434363734343037333730393535313631357d7d2c2273686970
6d656e7473223a7b223030303030303030303030303030303030303030303030303030303030303039223a7b22726f75
7465223a223030303030303030303030303030303030303030303030303030303030303038222c22736f75726365223a
223030303030303030303030303030303030303030303030303030303030303034222c2264657374696e6174696f6e22
3a223030303030303030303030303030303030303030303030303030303030303034222c227265736f757263655f6964
223a226f7265222c22756e697473223a31383434363734343037333730393535313631352c2264697370617463686564
5f7469636b223a302c226172726976616c5f7469636b223a31383434363734343037333730393535313631357d7d2c22
68617a61726473223a7b223030303030303030303030303030303030303030303030303030303030303061223a7b226c
616e65223a223030303030303030303030303030303030303030303030303030303030303035222c2268617a6172645f
6964223a22696f6e2d73746f726d222c2273746172745f7469636b223a302c226475726174696f6e5f7469636b73223a
31383434363734343037333730393535313631352c2263616e63656c6c6564223a747275657d7d2c226f776e65727322
3a7b223030303030303030303030303030303030303030303030303030303030303034223a2230303030303030303030
30303030303030303030303030303030303030303031227d7d
```

### Exact PACK_EMPTY SHA-256 input hex

Concatenate these hex lines without whitespace; they encode the complete domain+NUL+literal JSON input.

```text
4e594f4e2d574f524b53484f502d5041434b2d5631007b226b696e64223a224e594f4e5f574f524b53484f505f444154
41222c22666f726d61745f76657273696f6e223a312c227061636b5f6964223a2261222c227061636b5f76657273696f
6e223a312c227469746c65223a2241222c22737461725f61726368657479706573223a5b5d2c22776f726c645f617263
68657479706573223a5b5d2c227265736f7572636573223a5b5d2c22696e6475737472795f646566696e6974696f6e73
223a5b5d2c2268617a6172645f646566696e6974696f6e73223a5b5d7d
```

### Exact PACK_CORE_MANUAL SHA-256 input hex

Concatenate these hex lines without whitespace; they encode the complete domain+NUL+literal JSON input.

```text
4e594f4e2d574f524b53484f502d5041434b2d5631007b226b696e64223a224e594f4e5f574f524b53484f505f444154
41222c22666f726d61745f76657273696f6e223a312c227061636b5f6964223a226e796f6e2e636f7265222c22706163
6b5f76657273696f6e223a312c227469746c65223a224e594f4e20436f7265222c22737461725f617263686574797065
73223a5b7b226964223a2279656c6c6f772d6477617266222c226e616d65223a2259656c6c6f77204477617266222c22
6465736372697074696f6e223a224120737461626c65206d61696e2d73657175656e636520737461722e227d5d2c2277
6f726c645f61726368657479706573223a5b7b226964223a22726f636b792d776f726c64222c226e616d65223a22526f
636b7920576f726c64222c226465736372697074696f6e223a224120736f6c696420776f726c64207375697461626c65
20666f7220696e6475737472792e227d5d2c227265736f7572636573223a5b7b226964223a22616c6c6f79222c226e61
6d65223a22416c6c6f79222c226465736372697074696f6e223a22526566696e6564207374727563747572616c206d61
74657269616c2e227d2c7b226964223a22656e65726779222c226e616d65223a22456e65726779222c22646573637269
7074696f6e223a22456c656374726963616c20656e6572677920666f7220696e6475737472792e227d2c7b226964223a
226f7265222c226e616d65223a224f7265222c226465736372697074696f6e223a22556e726566696e6564206d696e65
72616c206665656473746f636b2e227d5d2c22696e6475737472795f646566696e6974696f6e73223a5b7b226b696e64
223a22657874726163746f72222c226964223a22657874726163746f72222c226e616d65223a22457874726163746f72
222c226465736372697074696f6e223a225573657320656e6572677920746f2065787472616374206c696e6b6564206f
72652072657365727665732e222c226475726174696f6e5f7469636b73223a312c22656e657267795f696e707574223a
7b227265736f757263655f6964223a22656e65726779222c227175616e74697479223a317d2c226f7574707574223a7b
227265736f757263655f6964223a226f7265222c227175616e74697479223a327d7d2c7b226b696e64223a2270726f63
6573736f72222c226964223a22666f756e647279222c226e616d65223a22466f756e647279222c226465736372697074
696f6e223a22436f6e766572747320656e6572677920616e64206f726520696e746f20616c6c6f792e222c2264757261
74696f6e5f7469636b73223a312c22696e70757473223a5b7b227265736f757263655f6964223a22656e65726779222c
227175616e74697479223a327d2c7b227265736f757263655f6964223a226f7265222c227175616e74697479223a337d
5d2c226f757470757473223a5b7b227265736f757263655f6964223a22616c6c6f79222c227175616e74697479223a31
7d5d7d2c7b226b696e64223a22736f6c6172222c226964223a22736f6c61722d6172726179222c226e616d65223a2253
6f6c6172204172726179222c226465736372697074696f6e223a2250726f647563657320666f757220656e6572677920
756e69747320706572207469636b2e222c226475726174696f6e5f7469636b73223a312c226f757470757473223a5b7b
227265736f757263655f6964223a22656e65726779222c227175616e74697479223a347d5d7d5d2c2268617a6172645f
646566696e6974696f6e73223a5b7b226964223a22696f6e2d73746f726d222c226e616d65223a22496f6e2053746f72
6d222c226465736372697074696f6e223a2248616c76657320726f757465206361706163697479206f6e20697473206c
616e65207573696e6720696e746567657220666c6f6f72206469766973696f6e2e222c226475726174696f6e5f746963
6b73223a33363030302c22726f7574655f63617061636974795f6e756d657261746f72223a312c22726f7574655f6361
7061636974795f64656e6f6d696e61746f72223a327d5d7d
```
