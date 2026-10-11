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
