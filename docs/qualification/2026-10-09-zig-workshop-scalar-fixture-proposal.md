# Task 4A remaining scalar, nominal and value-comparison fixture proposal

2026-10-09. **Current:** source inspection, handwritten inputs and the independent descriptor/arithmetic/literal-hash calculations recorded in the scoped evidence directory. **Proposed:** the finite future consumers and expected outcomes below. **Pending:** fresh independent review, immutable corpus freeze, every actual Rust/Zig constructor/decoder/equality/ownership result and Workshop implementation. No product source, approved fixture, existing proposal, profile or asset was written. No Rust/Zig build, test, serializer, authority or generator ran.

This proposal supplements the existing six-literal proposal and the valid-graph proposal. The latter is bound to SHA-256 `9bdb9290fe84f046a13cc485e9b50435ffc462292023511ab2bcb3a0c7c7b920`; its independent review was running at dispatch. The prior subset review requested complete nominal/scalar coverage and per-field equality. The valid-graph proposal already supplies basic decoded-ID/string boundaries, map permutations, absent/zero inventories and an all-four dynamic-view example; those are retained rather than relabeled as new runtime evidence. [task-4a-independent-fixture-review.md:59; task-4a-valid-fixture-proposal.md:73,295,333]

## Boundary and exact notation

4A owns distinct scalar wrappers, validated owned names/catalog IDs, geometry, `ObjectRefV1`, all ten entity records, value equality/clone/free, ordered maps and raw canonical state bytes/digest. 4B owns full catalog/strict pack decoding, validation and hash. 4C owns creator wire, revision records and namespace functions; command admission, capacities, simulation, authenticated history and archive installation remain later consumers. Raw state encoding does not validate an authority graph. [task-4a-foundation-vector-seams.md:22,31,33,35; crates/nyon-workshop-core/src/model.rs:165,187]

Notation fixes each input without an implicit newline or BOM:

- `R(h,n)` is exactly `n` repetitions of the single byte whose two-digit hex spelling is `h`. `||` concatenates bytes. `HEX(h)` is the bytes named by the complete even-length hex string. These are input descriptors, not output from either implementation.
- `ASCII(text)` means exactly the displayed ASCII bytes. JSON literals include their visible quotes/backslashes. Decoded string rows explicitly say **decoded**; do not feed their unescaped control bytes into a JSON parser.
- `J(s)` means quote byte `22`, the decoded ASCII string `s` with only quote/backslash escaped as `5c22`/`5c5c`, then quote byte `22`. It is used only with control-free ASCII ID/name controls. Unicode/control JSON cases below instead specify their complete escaped wire input.
- `I1` through `Ia` are raw16-byte IDs `R(00,15) || HEX(01)` through `R(00,15) || HEX(0a)`. `I10 = R(00,15) || HEX(10)`. Thus I4's JSON spelling is `"00000000000000000000000000000004"`. IDs remain distinct nominal values even where wire bytes match.

The accepted design fixes integer authority, typed IDs, coordinate bounds and atomic limits. Exact scalar encoding/errors here are compatibility supplements from the cited current source, not additional words attributed to the design. [docs/superpowers/specs/2026-09-02-nyon-v2-design.md:176,178,203,211,235; crates/nyon-workshop-core/src/ids.rs:26,257]

## Five fixed-ID consumers

Run each width-group row separately for every nominal type in that group. This expands a group descriptor into named consumers; success on EntityId does not establish BranchId, nor does RevisionId establish CatalogHash or StateDigest. Serialization emits lowercase high-nibble/low-nibble spelling; deserialization first obtains a String, checks exact decoded UTF-8 byte length and accepts only bytes `0-9a-f`. [crates/nyon-workshop-core/src/ids.rs:11,26,43,71]

| Nominal type | Raw byte width N | Decoded hex length L | Current consumer |
| --- | ---: | ---: | --- |
| EntityId | 16 | 32 | Serialize/Deserialize for EntityId |
| BranchId | 16 | 32 | Serialize/Deserialize for BranchId |
| RevisionId | 32 | 64 | Serialize/Deserialize for RevisionId |
| CatalogHash | 32 | 64 | Serialize/Deserialize for CatalogHash |
| StateDigest | 32 | 64 | Serialize/Deserialize for StateDigest |

| Case for each type | Exact raw/decoded input | Exact expected bytes or source-derived outcome |
| --- | --- | --- |
| ZERO | Raw `R(00,N)` | JSON `22 || R(30,L) || 22`; decoded zeros succeed, no nonzero sentinel rule |
| FF | Raw `R(ff,N)` | JSON `22 || R(66,L) || 22` |
| NIBBLES16 | Raw `000102030405060708090a0b0c0d0e0f` | JSON `"000102030405060708090a0b0c0d0e0f"` for each16-byte type |
| NIBBLES32 | Raw `000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f` | JSON `"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"` for each32-byte type |
| DECODED_LENGTH | Decoded `R(30,k)`, separately k=0,1,L-1,L+1 | custom deserializer error `invalid canonical identifier` |
| DECODED_BAD_ASCII | Decoded `HEX(c) || R(30,L-1)`, separately c=`41,46,47,67,2f,20,09,00,7f` | same custom error; uppercase A/F rejected, G/g rejected, no case folding |
| DECODED_BAD_UTF8_CHARACTER | Decoded `HEX(c3a9) || R(30,L-2)` | same custom error despite correct byte length; é is a valid UTF-8 character but outside ID alphabet |
| ESCAPED_ZERO | Wire `22 || ASCII(\u0030) || R(30,L-1) || 22` | decoded zero ID succeeds; re-encode exactly ZERO canonical bytes |
| ESCAPED_A | Wire `22 || ASCII(\u0041) || R(30,L-1) || 22` | uppercase A remains rejected with the custom ID error |
| WRONG_JSON_TYPE | Wire `0`, `null`, `[]`, `{}`, separately | serde JSON type error while acquiring String, not the custom ID-validator error |

The first nine decoded cases per width group in the valid-graph proposal are overlapping controls, not newly independent provenance. The cases above add empty/one-character lengths, uppercase F/G, whitespace/control classes, escaped spelling and wrong wire types with named boundaries. JSON string escapes are decoded before the source validator; the local pinned serde string parser maps `\u0030` to0 and `\u0041` toA. This is source-derived characterization, **not execution** of Rust or Zig ingress. [task-4a-valid-fixture-proposal.md:316; crates/nyon-workshop-core/src/ids.rs:48; local serde_json-1.0.151/src/read.rs:874,900; task-4a-scalar-fixture-evidence/source-excerpts.log]

**Order witnesses:** for16 bytes, A=`00010000000000000000000000000000`, B=`01000000000000000000000000000000`; for32 bytes, A=`0001000000000000000000000000000000000000000000000000000000000000`, B=`0100000000000000000000000000000000000000000000000000000000000000`. For every matching-width nominal type, expect A<B, A!=B, A==a detached A copy, and A<FF. First byte0<1 determines A<B; little-endian integer A=256, B=1 would give the opposite. No conversion to an integer is part of the ordering API. [crates/nyon-workshop-core/src/ids.rs:11,14,17,20,23]

**Nominal rejection specification:** the future Zig compile fixture defines `accept(T, x:T)` for each type and constructs `x` through that source type's public constructor using the relevant zero bytes. Compile each directed ordered pair below as a separate isolated invocation, passing source value to target receiver. Expect compile rejection before runtime; do not claim a compiler diagnostic string or a presently existing Zig API. [docs/superpowers/specs/2026-09-02-nyon-v2-design.md:178; task-4a-foundation-vector-seams.md:26,131]

| Target receiver | Source types each rejected, independently |
| --- | --- |
| EntityId | BranchId, RevisionId, CatalogHash, StateDigest |
| BranchId | EntityId, RevisionId, CatalogHash, StateDigest |
| RevisionId | EntityId, BranchId, CatalogHash, StateDigest |
| CatalogHash | EntityId, BranchId, RevisionId, StateDigest |
| StateDigest | EntityId, BranchId, RevisionId, CatalogHash |

These are20 negative compile consumers, including eight directed same-width pairs. Each receiver also has a successful same-type control compiled by itself. A combined file stopping at the first error does not demonstrate20 rejections. Add six directed pair checks among WorkshopTick, BatchLocalId, GalaxyCoord (all zero controls), and both directions CatalogId/ObjectName (`a` constructed separately), to preserve the remaining nominal wrapper distinctions. All28 negative and ten positive controls are **Proposed**; none ran. The source API exposes `CatalogId::new/as_str`, `ObjectName::new/as_str`, `GalaxyCoord::new/get` and public numeric wrappers; Zig equivalents are not implemented. [crates/nyon-workshop-core/src/ids.rs:9,87,93,144,195]

## Numeric scalars, points and actual consumer errors

The current repository locks serde/serde_core1.0.229 and serde_json1.0.151. Core manifests request serde derive and the standard serde_json dependency; the directly typed JSON integer deserializers call the inspected number parser. Error categories below refer to direct `serde_json::from_slice::<Type>` consumers, not arbitrary serde formats, a strict pack parser or an authority validator. Exact line/column suffixes are deliberately not goldens. [Cargo.lock:2506,2516,2536; crates/nyon-workshop-core/Cargo.toml:10; local serde_json-1.0.151/src/de.rs:319,1500]

| Type / exact ASCII JSON input | Proposed outcome / canonical success bytes |
| --- | --- |
| WorkshopTick `0`, `1`, `9007199254740993`, `18446744073709551615` | exact u64 values; serialize to the identical decimal bytes, no quotes or float |
| BatchLocalId `0`, `1`, `65535` | exact u16 values; serialize identically |
| BatchLocalId `65536`; WorkshopTick `-1` | serde JSON data error from integer-range conversion, not IdentifierError |
| WorkshopTick `18446744073709551616` | number parser switches to F64 on u64 overflow; integer visitor rejects with serde JSON data/type error |
| Both unsigned wrappers, each of `-0`, `0.0`, `1.0`, `1.5`, `1e0` | F64 visit rejected; an integral mathematical value does not make the JSON lexeme an accepted integer |
| Both unsigned wrappers, each of `"0"`, `null`, `true`, `[]`, `{}` | serde JSON type/data error; no coercion |
| Both unsigned wrappers, each of `00`, `+1`, `0 1` | syntax/trailing-input error; no accepted complete typed value |
| GalaxyCoord `-134217728`, `-1`, `0`, `1`, `134217728` | checked coordinate succeeds, serialize exact signed decimal |
| GalaxyCoord `-134217729`, `134217729`, `-9223372036854775808`, `9223372036854775807` | i64 parses, then custom `galaxy coordinate is outside the supported range` from CoordinateOutOfRange |
| GalaxyCoord `9223372036854775808` | u64 parses, serde conversion to i64 fails before coordinate constructor; integer-range data error |
| GalaxyCoord `-9223372036854775809` | signed underflow becomes F64 and is rejected before coordinate constructor |
| GalaxyCoord each of `-0`, `0.0`, `1e0`, `"0"` | serde integer type error before coordinate constructor |

For `-0`, `parse_number(false,0)` chooses F64; serde's i64/u16/u64 visitors do not supply F64 coercion and inherit a failing default. For finite max+1 values above, rejection follows parser/visitor class, not a made-up Workshop overflow enum. **Current source inspection** supports these outcomes; actual feature-resolved ingress execution and exact diagnostics remain Pending. [local serde_json-1.0.151/src/de.rs:462,509; local serde_core-1.0.229/src/de/impls.rs:129,399,420,436; local serde_core-1.0.229/src/de/mod.rs:1491; task-4a-scalar-fixture-evidence/source-excerpts.log]

For each coordinate constructor boundary run direct `GalaxyCoord::new` separately from JSON. Direct i64 max/min yield CoordinateOutOfRange, not serde errors. Point constructors `(134217729,0)`, `(0,-134217729)` fail the corresponding axis; `(-134217728,134217728)` succeeds. Point canonical wire is ordered x,y, including `{"x":-1,"y":2}`. Direct typed integers do not have a negative-zero representation. [crates/nyon-workshop-core/src/ids.rs:195,209,219,226,257]

Point direct serde cases: `{"x":0,"y":0}` succeeds; `{"y":0,"x":0}` succeeds and canonical output is `{"x":0,"y":0}`; `{"x":0}`, `{"x":0,"y":0,"z":0}` and `{"x":0,"x":1,"y":0}` fail missing/unknown/duplicate struct-field handling. These are aggregate serde consumer proposals. The known `deny_unknown_fields` annotation and derived struct machinery support the first two failure classes; **duplicate-field exact diagnostics are not independently derived here and remain Pending characterization**. Do not convert these into a new global raw-state parser guarantee. [crates/nyon-workshop-core/src/ids.rs:219; task-4a-foundation-vector-seams.md:168]

**Ordering:** typed tick values inserted in order `[10,2,9007199254740993,0,18446744073709551615,1]` expect `[0,1,2,10,9007199254740993,18446744073709551615]`. Local input `[65535,10,2,0,1]` expects `[0,1,2,10,65535]`; coords `[134217728,0,-1,-134217728,1]` expect `[-134217728,-1,0,1,134217728]`. Numeric10 follows2 although ASCII `10` precedes `2`. [crates/nyon-workshop-core/src/ids.rs:5,85,191]

**Distance extension:** legal opposite corners have `dx=dy=268435456`, square `144115188075855872` and floor379625062. Verify the two neighboring square inequalities with integer arithmetic. Existing six reviewed pairs and255/256/257 route-boundary pairs remain their original candidates. Add `(1,-1),(-2,3)`→5 and reverse→5 (square25) and both coordinate-limit axis pairs→268435456 (square72057594037927936). All legal inputs fit checked i128; there is no legal-coordinate DistanceOverflow witness. Route ceil travel remains a later simulation consumer. [task-4a-independent-fixture-proposal.md:32; task-4a-valid-fixture-proposal.md:331; crates/nyon-workshop-core/src/ids.rs:234,297]

`ObjectRefV1` controls are exact JSON `{"kind":"existing","id":"00000000000000000000000000000000"}` and `{"kind":"local","id":0}`, plus local65535. Existing/Local identity participates in equality: Existing(I1) differs from Existing(I2), Local(0) differs from Local(1), Existing(zero16) differs from Local(0). Local65536 and Existing uppercase-A reject at their respective scalar consumers. Unknown kind `"other"`, missing kind or missing id reject the tagged enum consumer. Exact unknown-field/duplicate-tag permissiveness is **not frozen** here: this enum lacks `deny_unknown_fields`, unlike points/state records. [crates/nyon-workshop-core/src/ids.rs:250; task-4a-foundation-vector-seams.md:51,168]

## Bounded strings, ASCII classes and ownership

The following decoded constructor/helper descriptor sets are finite. Apply each row independently; no trimming, case folding, whitespace collapse or Unicode normalization is allowed. `CatalogId::new` errors are IdentifierError::CatalogId (`catalog identifier is invalid`); ObjectName errors are IdentifierError::ObjectName (`object name is invalid`). Pack-ID/description are crate-private booleans, not extra IdentifierError variants. [crates/nyon-workshop-core/src/ids.rs:93,144,257,269,277,285,293]

| Consumer | Exact accepted controls | Exact rejected boundaries |
| --- | --- | --- |
| CatalogId::new | `a`; `a0_-`; `R(61,32)` | empty, `R(61,33)`; `0a`, `A`, `_a`, `-a`, `.a`, `a.b`, `a b`, ` a`, `a ` |
| is_pack_id | `a`; `a0._-`; `R(61,48)` | empty, `R(61,49)`; `0a`, `A`, `_a`, `-a`, `.a`, `a/b`, `a b`, ` a`, `a ` |
| ObjectName::new | `A`; `A B`; `!~`; `HEX(51225c)`; `R(41,64)` | empty, `R(41,65)`; ` A`, `A `, `A  B` |
| is_description | empty, `HEX(20)`, ` A  B `, `R(41,512)` | `R(41,513)` |

**Complete ASCII position classes:** for every byte c from00 through7f, instantiate decoded one-byte `HEX(c)` independently for CatalogId and pack-ID helper; accept exactly61..7a, reject every other first byte. For every c00..7f, instantiate `HEX(61) || HEX(c)` independently; catalog accepts exactly30..39,61..7a,2d,5f; pack also accepts2e. These four finite sets include controls, uppercase, punctuation and edge spaces, with all accepted data unchanged. Their one/two-byte inputs avoid length masking a character failure. [crates/nyon-workshop-core/src/ids.rs:269,277]

For every printable c20..7e, ObjectName::new on `HEX(21) || HEX(c) || HEX(21)` succeeds, including single interior space, quote and backslash. For every c00..1f and7f, the same three-byte input fails; is_description on that three-byte input likewise fails. Independently, description on one byte c20..7e succeeds, including a solitary space. Name tests `A  B`, ` A`, `A ` isolate doubled/edge-space rejection. Every described input is valid UTF-8 even when it contains NUL/control/DEL; no invalid-UTF-8 byte is mislabeled as an admissible Rust &str argument. [crates/nyon-workshop-core/src/ids.rs:285,293]

Unicode class inputs, each decoded and supplied independently to all four consumers: `HEX(c3a9)` (é), `HEX(65cc81)` (e plus combining acute), `HEX(f09f9880)` (😀), and `ASCII(a) || HEX(c2a0)` (nonbreaking space suffix). All reject. Name/description use printable ASCII predicates; catalog/pack use ASCII alphabets. The two accent spellings remain distinct rejected byte sequences; no normalized equality is implied. Invalid raw UTF-8 `HEX(ff)` is a **4B preflight** input expecting InvalidUtf8, not a valid direct string constructor call. [crates/nyon-workshop-core/src/ids.rs:269,277,285,293; crates/nyon-workshop-core/src/pack.rs:503]

Direct serde string controls: CatalogId wire `"\u0061"` succeeds as content `a`, canonical output `"a"`; `"\u0041"` and `"a\u0000"` fail with the CatalogId custom message. ObjectName wire `"A\u0020B"` succeeds as `A B`, canonical output `"A B"`; `"\u0020A"`, `"A\u0020\u0020B"`, `"A\u007f"` fail with ObjectName custom message. Exact quote/backslash accepted bytes `HEX(51225c)` serialize to JSON hex `22515c225c5c22`. Numeric/null/array inputs fail acquiring String, before the string-domain validator. These spellings are source-derived consumer candidates, with decoder execution Pending. [crates/nyon-workshop-core/src/ids.rs:130,181; local serde_json-1.0.151/src/read.rs:874,900; task-4a-independent-fixture-proposal.md:27]

String equality/order controls: separately allocate CatalogId `a`,`a-`,`a0`,`a_`; reverse insertion expects that exact lexical order, since prefix then2d<30<5f. Independently allocated ObjectName `A` equals `A`; `A`<`A B`<`B`; `A B` differs from `A  B` because the latter must fail construction, so it must never be supplied as a validated name. Accepted string creation and clone must own their bytes after caller input is overwritten/freed. Mutating/freeing a detached clone may not alter the original. Creation/clone OOM must return a resource error and unwind once; this is a **Proposed Zig ownership contract**, not a current Rust IdentifierError variant or executed failure witness. [crates/nyon-workshop-core/src/ids.rs:89,91,140,142; task-4a-foundation-vector-seams.md:137]

### Pack bridge, with the different consumer named

The exact baseline below is a handwritten minimal pack input; by source predicates it has no missing minimum definition requirement. It is a proposed4B control, not an executed admission or new complete pack corpus. [crates/nyon-workshop-core/src/pack.rs:190,316,356,371]

```PACK_SCALAR_BRIDGE_BASE
{"kind":"NYON_WORKSHOP_DATA","format_version":1,"pack_id":"a","pack_version":1,"title":"A","star_archetypes":[],"world_archetypes":[],"resources":[],"industry_definitions":[],"hazard_definitions":[]}
```

Each bridge case makes exactly the named whole-field fragment replacement once in this literal. `"pack_id":"a"`→`"pack_id":"A"` gives InvalidField{field:"pack_id"}; `"title":"A"`→`"title":" A"` gives InvalidField{field:"title"}. To exercise definition metadata, replace `"star_archetypes":[]` with `"star_archetypes":[{"id":"a","name":"A","description":""}]`; this is the metadata control. Then its `"name":"A"`→`"name":" A"` gives InvalidField{field:"name"}; description empty→JSON string containing `R(41,513)` gives InvalidField{field:"description"}. Definition `"id":"a"`→`"id":"A"` fails CatalogId deserialization and maps to PackValidationError::Json **before** validate_identity. It does not reach InvalidField{field:"id"}. ObjectName is not the pack metadata string type. [crates/nyon-workshop-core/src/pack.rs:145,192,320,356,448]

4B owns strict BOM/UTF-8/depth/duplicate-key/float handling, quantities, all definition limits, recipes/references and canonical export/hash. For example an unknown-resource recipe error cannot be inferred by running only the CatalogId helper. Strict number preflight/visitor errors cannot be substituted for direct serde scalar errors. Full strict corpus remains Pending; this bridge only binds string-domain mapping, not4B completion. [crates/nyon-workshop-core/src/pack.rs:286,471,503,636,652; task-4a-foundation-vector-seams.md:31]

## Every-field equality matrix

Baseline S is exactly `STATE_ALL_RECORDS_RAW` from the reviewed six-literal proposal, source declaration order preserved. It is intentionally **invalid authority** and is never installed. Each row below defines a typed detached-copy comparison; mutate only the specified path with the displayed before→after values. Compare original S to the changed copy: expect structural inequality and different canonical bytes. Hash inequality is not used as an equality oracle. All unchanged clone controls must compare equal despite different allocation addresses/capacities/insertion histories. [task-4a-independent-fixture-proposal.md:55,57,61; crates/nyon-workshop-core/src/model.rs:38,129,187; task-4a-foundation-vector-seams.md:80]

Paths use the full typed entity key alias defined above; these aliases specify exact IDs, not list indices. A grouped row with multiple suffixes means separate detached copies, never combined mutation. This is48 leaf/option/inventory cases covering all41 top-level record fields, including each RGB channel, both position axes and inventory key/presence/value distinctions. [crates/nyon-workshop-core/src/model.rs:38-127]

| Path in S | Exact before→after, each independent case |
| --- | --- |
| factions[I1].name | decoded `HEX(51225c)`→`R` |
| factions[I1].color_rgb[0,1,2] | index0:0→1; index1:127→128; index2:255→254 |
| systems[I2].name | `S`→`R` |
| systems[I2].position.x | -134217728→-134217727 |
| systems[I2].position.y | 134217728→134217727 |
| stars[I3].system | I2→I10 |
| stars[I3].name | `T`→`R` |
| stars[I3].archetype_id | `yellow-dwarf`→`x` |
| worlds[I4].system | I2→I10 |
| worlds[I4].primary | I3→I10 |
| worlds[I4].name | `W`→`R` |
| worlds[I4].archetype_id | `rocky-world`→`x` |
| worlds[I4].orbit_radius_milli_au | 4294967295→4294967294 |
| worlds[I4].orbit_period_ticks | 18446744073709551615→18446744073709551614 |
| worlds[I4].phase_millidegrees | 4294967295→4294967294 |
| worlds[I4].inventory["a"] value | resident0→resident1 |
| worlds[I4].inventory["a"] presence | resident0→absent, preserve all other entries |
| worlds[I4].inventory["a_"] value | 18446744073709551615→18446744073709551614 |
| worlds[I4].inventory key | rename a0→a1, preserving9007199254740993 |
| lanes[I5].a | I2→I10 |
| lanes[I5].b | I2→I10 |
| lanes[I5].distance_units | 18446744073709551615→18446744073709551614 |
| deposits[I6].world | I4→I10 |
| deposits[I6].resource_id | `ore`→`x` |
| deposits[I6].reserve_units | 18446744073709551615→18446744073709551614 |
| industries[I7].world | I4→I10 |
| industries[I7].definition_id | `extractor`→`x` |
| industries[I7].linked_deposit | Some(I6)→None; separately Some(I6)→Some(I10) |
| industries[I7].enabled | true→false |
| routes[I8].source | I4→I10 |
| routes[I8].destination | I4→I10 |
| routes[I8].resource_id | `ore`→`x` |
| routes[I8].batch_units | 18446744073709551615→18446744073709551614 |
| shipments[I9].route | I8→I10 |
| shipments[I9].source | I4→I10 |
| shipments[I9].destination | I4→I10 |
| shipments[I9].resource_id | `ore`→`x` |
| shipments[I9].units | 18446744073709551615→18446744073709551614 |
| shipments[I9].dispatched_tick | 0→1 |
| shipments[I9].arrival_tick | 18446744073709551615→18446744073709551614 |
| hazards[Ia].lane | I5→I10 |
| hazards[Ia].hazard_id | `ion-storm`→`x` |
| hazards[Ia].start_tick | 0→1 |
| hazards[Ia].duration_ticks | 18446744073709551615→18446744073709551614 |
| hazards[Ia].cancelled | true→false |

The remaining24 independent state-container comparisons are: tick max→max-1 (one); owners[I4] I1→I10, rename owner key I4→I10 preserving I1, remove owners[I4] (three); for each of the ten single-entry entity maps in S, rename its key to I10 preserving the record, and separately remove its only entry (twenty). All expect inequality. S has12 top-level state fields (tick plus11 maps); owners keys and values both contribute. No key rename silently rewrites references; these are raw equality witnesses, not valid command mutations. [crates/nyon-workshop-core/src/model.rs:129-144]

Together there are72 independent changed-copy cases. Also run equality reflexivity, symmetry (both argument orders for all72), equal detached deep clone, and equality under each already proposed order permutation. Deep copy independence uses the faction name and World inventory mutations above, then frees the clone and verifies original exact S bytes; free original first in a separate case and verify clone bytes. Allocation-failure enumeration for name/key/map insertion, deep clone and encode/digest remains a future implementation witness, not supplied by this matrix. [crates/nyon-workshop-core/src/model.rs:38,129; task-4a-valid-fixture-proposal.md:77; task-4a-foundation-vector-seams.md:137]

## Complete dynamic replacements, isolated from static maps

The source dynamic view replaces **whole** tick/worlds/deposits/shipments, borrowing every other map from the receiver. It does not merge old and candidate maps and cannot retain slices after the synchronous call. Its map/field order matches ordinary canonical state encoding. The valid-graph all-four example remains unchanged; these additional raw sentinel cases detect omitted individual overrides, accidental merging and accidentally replacing static content. [crates/nyon-workshop-core/src/model.rs:195-238; task-4a-valid-fixture-proposal.md:75]

Use receiver S and define exact field fragments below, all copied as literal reviewed bytes. Each named replacement is one top-level fragment occurrence, with no whitespace inserted. Full input/preimage hex and SHA-256 for each byte-defined dynamic expected state are in `derive-literals-01.log`; the calculator performs literal byte substitutions only, never a JSON serializer. These are **independent standard hash calculations**, not an implemented dynamic-view call.

| Replacement symbol | Exact old fragment | Exact replacement |
| --- | --- | --- |
| T | `"tick":18446744073709551615` | `"tick":0` |
| W | `"worlds":{"00000000000000000000000000000004":{"system":"00000000000000000000000000000002","primary":"00000000000000000000000000000003","name":"W","archetype_id":"rocky-world","orbit_radius_milli_au":4294967295,"orbit_period_ticks":18446744073709551615,"phase_millidegrees":4294967295,"inventory":{"a":0,"a-":1,"a0":9007199254740993,"a_":18446744073709551615}}}` | `"worlds":{}` |
| D | `"deposits":{"00000000000000000000000000000006":{"world":"00000000000000000000000000000004","resource_id":"ore","reserve_units":18446744073709551615}}` | `"deposits":{}` |
| H | `"shipments":{"00000000000000000000000000000009":{"route":"00000000000000000000000000000008","source":"00000000000000000000000000000004","destination":"00000000000000000000000000000004","resource_id":"ore","units":18446744073709551615,"dispatched_tick":0,"arrival_tick":18446744073709551615}}` | `"shipments":{}` |
| F | `"name":"Q\"\\"` | `"name":"R"` |

| Future dynamic consumer | Receiver, override inputs | Exact expected bytes |
| --- | --- | --- |
| DV_IDENTITY | S; S.tick/S.worlds/S.deposits/S.shipments | S |
| DV_TICK_ONLY | S; tick0, all three maps from S | apply T only to S |
| DV_WORLDS_EMPTY | S; S.tick, empty worlds, S.deposits/S.shipments | apply W only |
| DV_DEPOSITS_EMPTY | S; S.tick/S.worlds, empty deposits, S.shipments | apply D only |
| DV_SHIPMENTS_EMPTY | S; S.tick/S.worlds/S.deposits, empty shipments | apply H only |
| DV_ALL_EMPTY | S; tick0, all three maps empty | apply T,W,D,H |
| DV_POISONED_RECEIVER | receiver bytes S after T,W,D,H; replacements restore S.tick and complete three maps from detached S | S, because receiver's four dynamic fields are ignored |
| DV_STATIC_CHANGED | receiver S after F; overrides S.tick and S's three complete maps | apply F only, because receiver faction must remain |
| DV_DISTINCT_ALLOCATION | S; equal clones of three S maps, same tick | S, independent of addresses/capacities |

For DV_ALL_EMPTY the expected worlds/deposits/shipments maps have zero entries despite nonempty receiver maps. This catches a merge implementation. DV_POISONED_RECEIVER catches use of receiver tick/maps. DV_STATIC_CHANGED catches use of a whole replacement state rather than receiver static maps. Caller data must remain alive throughout encoding; snapshot before/after unchanged, then release temporary replacements and verify receiver bytes to establish no ownership transfer. Comparing dynamic digest with ordinary encoding of a detached state using the same substitutions is a **second** consumer witness; both must compare against the literal expected bytes/preimages, not just each other's output. Every actual lifetime/allocator/runtime observation remains Pending. [crates/nyon-workshop-core/src/model.rs:195,220; task-4a-foundation-vector-seams.md:28,137]

## Exact residuals by owner

4A preparation remaining after this proposal: fresh review and any corrections; source-contract compatibility acceptance where source adds detail to design; immutable complete fixture manifest with source/payload hashes; selected Zig semantic APIs for constructors, sorted map ownership and fallible clone/encode/digest; actual isolated nominal compile witnesses, scalar ingress if a decoder is included, equality/geometry/lifetime/OOM execution and Rust differential witnesses. Aggregate raw JSON duplicate keys/enum unknown fields remain Pending ingress characterization. No public installable raw-state API is proposed. [task-4a-foundation-vector-seams.md:24,125,162]

4B still needs its full strict JSON, size/depth/cardinality, quantity/duration, duplicate definition/quantity, recipe and missing-reference corpus, canonical catalog ordering/export/hash and immutable catalog ownership. The scalar bridge does not cover those groups. 4C still needs creator batch/operation wire strictness, revision parent-none/parent-some and other namespace preimages, receipt/order/size rules; nominal ID representation alone does not implement revision identity. Full atomic admission, hazard active/scheduled capacities, simulation, branches/replay/archive, Forge18 and platform/session acceptance remain later task scopes. [task-4a-foundation-vector-seams.md:31,33,35,92,136; task-4a-valid-fixture-proposal.md:337]

Those later operational results are **not prerequisites for independently reviewing/freezing the finite4A literal byte inputs**. A reviewed valid-graph construction can supply source-derived encoding expectations before its command/replay consumer runs. Freeze readiness for4A means its independently derived IDs/state/canonical-byte cases are reviewed and bound; it does not imply that4B/4C/4D/4F already executed. This preserves the migration plan's4A→4B→4C→4D→4E order and avoids a dependency cycle. The actual immutable4A freeze decision remains with the parent, outside this proposal's scope. [docs/superpowers/plans/2026-10-09-nyon-zig-migration.md:148,152; task-4a-valid-fixture-proposal.md:29]

## Actual evidence and freeze record

Only this proposal and `task-4a-scalar-fixture-evidence/` are written. `source-before.json` binds19 inspected source/report/dependency identities. `source-excerpts.log` preserves the exact numbered source passages, including locally installed pinned serde files; these local source reads are not external research. The separate derivation script and its child-exit receipt record descriptor widths/counts, numeric ordering, square inequalities, complete matrix field coverage and SHA-256 of the literal dynamic payloads. SHA-256 is standard `hashlib.sha256(ASCII("NYON-WORKSHOP-STATE-V1") || HEX(00) || literal_payload)` with no trailing newline, BOM or length prefix. It never invokes a production serializer or generator. [crates/nyon-workshop-core/src/model.rs:218]

The actual source-only derivation command, run from `/Users/donaldfilimon/dev/active/NYON`, was:

```text
python3 .superpowers/sdd/2026-10-09-nyon-zig-migration/task-4a-scalar-fixture-evidence/derive-literals.py
```

Its child exit was **0**. The receipt records start1791552040.936376/end1791552041.145205 Unix seconds, script SHA-256 `337e3dbf1cdf562fda4c2bdaf6b434d7c20ccd07d6c6e77dcdde1f5d7de30887`, and complete log SHA-256 `9ca659ded0755ea9ca9a0622baf08d5a9052f677dbd786b49e599c537b766514`. It checked85 decoded fixed-ID descriptors plus10 escaped and20 wrong-wire-type descriptors (115 total),28 future negative compile-pair descriptors and10 same-type controls, numeric-width/order/square arithmetic, ASCII descriptor counts, all41 declared entity fields and48 record+24 state equality cases, all nine dynamic expected payloads, the199-byte pack bridge literal, and19 unchanged source identities. Compile-pair and equality checks validate **proposal descriptors/coverage**, not compiled rejection or a production equality implementation. The log's PASS label is explicitly for descriptor/arithmetic/literal hashing only. [task-4a-scalar-fixture-evidence/derive-literals-01.log; task-4a-scalar-fixture-evidence/derive-literals-01-receipt.json]

The nine literal dynamic expected payload hash results are:

| Case | Payload bytes | Domain+payload bytes | Standard SHA-256 |
| --- | ---: | ---: | --- |
| DV_IDENTITY | 1970 | 1993 | f0c1dc0c71db4253e4f3ba8b31b5f9323efe8b5d52565d46dcd0e9a643fd8da5 |
| DV_TICK_ONLY | 1951 | 1974 | 9b13a2d2a7e7951ed448f466c4efa5878e00a5b2d09c4a551e0c3804425421a9 |
| DV_WORLDS_EMPTY | 1621 | 1644 | 9e80f17c4766fbe86ecbe67c5469ae23a31903b0ec6a011599f8ee87f9e9e0a3 |
| DV_DEPOSITS_EMPTY | 1834 | 1857 | af2c2cbeecd92d05e6463532f109164c7c3f88c848d3db40641ca8fa262bb79d |
| DV_SHIPMENTS_EMPTY | 1693 | 1716 | a2ce1ccdd23373291ea67f6ae7b22666e2e12ff3fae90a4acfc4ee5c745b6e97 |
| DV_ALL_EMPTY | 1189 | 1212 | ca4d56f73f7049e0551e1bbee6a7c8f7f75ab8a1362a187fe74a3bce29ec1e3e |
| DV_STATIC_CHANGED | 1966 | 1989 | d430b81c3ca9523df9954f3055acf90663d32fa3a2be933c37d33f83f076890c |
| DV_POISONED_RECEIVER | 1970 | 1993 | f0c1dc0c71db4253e4f3ba8b31b5f9323efe8b5d52565d46dcd0e9a643fd8da5 |
| DV_DISTINCT_ALLOCATION | 1970 | 1993 | f0c1dc0c71db4253e4f3ba8b31b5f9323efe8b5d52565d46dcd0e9a643fd8da5 |

These results derive only from the byte-defined inputs above; no runtime view produced them. Full payload/preimage hex is retained in the bound log. Closing `source-after.json` verified all19 bound input identities unchanged. The final structural/identity check receipt separately binds the final proposal and evidence fingerprints. Proposal review input is bound by the externally returned final proposal SHA, avoiding a self-referential hash. A report fingerprint is not a fixture freeze, admission result, test pass or4A/4B completion. [task-4a-scalar-fixture-evidence/derive-literals-01.log; task-4a-scalar-fixture-evidence/source-after.json]
