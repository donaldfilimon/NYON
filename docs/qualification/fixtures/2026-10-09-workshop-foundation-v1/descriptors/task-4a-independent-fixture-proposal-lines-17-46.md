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

