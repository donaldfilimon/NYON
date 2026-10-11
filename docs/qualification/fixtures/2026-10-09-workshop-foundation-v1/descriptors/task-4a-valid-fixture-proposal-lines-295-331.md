## Finite scalar constructor and decoded-ID corpus

These inputs name their actual scalar consumer. String notation is decoded bytes; `ASCII A repeated N` means exactly N bytes0x41 and no implicit newline, while hex inputs state complete exceptional bytes. Constructor errors and pure helper booleans below follow source predicates; no runtime constructor or deserializer ran. Unknown raw JSON lexeme behavior is explicitly excluded. [crates/nyon-workshop-core/src/ids.rs:43,94,145,195,269]

| Consumer | Exact finite inputs | Proposed outcomes |
| --- | --- | --- |
| CatalogId::new | `a`; `a0_-`; `abcdefghijklmnopqrstuvwxyz012345` (32 bytes) | success with unchanged content |
| CatalogId::new | empty; `abcdefghijklmnopqrstuvwxyz0123456` (33); `0a`; `A`; `a.b`; `a b`; UTF-8 hex `c3a9` | `IdentifierError::CatalogId` |
| ObjectName::new | `A`; `A B`; `!~`; semantic bytes `51225c` (Q, quote, backslash); ASCII A repeated64 | success with unchanged content; quoted JSON spelling already independently reviewed |
| ObjectName::new | empty; ASCII A repeated65; ` A`; `A `; `A  B`; decoded bytes `410042`, `411f42`, `417f42`, UTF-8 `41c3a9` | `IdentifierError::ObjectName` |
| is_pack_id helper | `a.b`; ASCII a repeated48 | true; dot allowed here |
| is_pack_id helper | empty; ASCII a repeated49; `0a`; `A`; `a/b` | false |
| is_description helper | empty; ` A  B `; ASCII A repeated512 | true; whitespace restrictions for names do not apply |
| is_description helper | ASCII A repeated513; hex `00`, `1f`, `7f`, `c3a9` | false |
| GalaxyCoord::new | -134217728,-1,0,1,134217728 | success, exact signed values retained |
| GalaxyCoord::new | -134217729,134217729,-9223372036854775808,9223372036854775807 | `IdentifierError::CoordinateOutOfRange`; i64-bound inputs still fail coordinate bounds |
| GalaxyPointV1::new | (-134217728,134217728) | success; x then y in canonical bytes |
| GalaxyPointV1::new | (134217729,0), (0,-134217729) | CoordinateOutOfRange; first failing axis wins at construction |

Source checks are byte-length based with no trimming, case folding or Unicode normalization. Constructor accepted strings are owned; content/lifetime/allocation witnesses belong the Zig implementation. Pack-description helpers do not imply complete pack admission. [crates/nyon-workshop-core/src/ids.rs:94,145,227,269,277,285,293]

For **each** of EntityId and BranchId, test the following decoded string rows as its own nominal consumer. For **each** of RevisionId, CatalogHash and StateDigest, use the second group. The raw JSON wire shape alone does not distinguish EntityId from BranchId or the three32-byte types. Nominal cross-type rejection is a future Zig compile-time/API property; do not fabricate a runtime parser rejection for identical valid wire strings. [crates/nyon-workshop-core/src/ids.rs:11,26,71]

| Decoded string group | Exact bytes / strings | Proposed decoded-validator outcome |
| --- | --- | --- |
| 16-byte valid | `00000000000000000000000000000000`; `ffffffffffffffffffffffffffffffff`; `000102030405060708090a0b0c0d0e0f` | success, raw16 bytes retained |
| 16-byte length failures | `0000000000000000000000000000000` (31); `000000000000000000000000000000000` (33) | custom `invalid canonical identifier` |
| 16-byte character failures,32-byte-length strings | `A0000000000000000000000000000000`; `g0000000000000000000000000000000`; `/0000000000000000000000000000000` | same custom error |
| 16-byte non-ASCII case | hex decoded bytes `c3a9` followed by30 bytes0x30 (32-byte string) | same custom error despite correct byte length |
| 32-byte valid | `0000000000000000000000000000000000000000000000000000000000000000`; `ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff`; `000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f` | success, raw32 bytes retained |
| 32-byte length failures |63 bytes0x30;65 bytes0x30 | same custom error |
| 32-byte character failures | ASCII A/g/slash respectively followed by63 bytes0x30 | same custom error |
| 32-byte non-ASCII case | hex `c3a9` followed by62 bytes0x30 (64-byte string) | same custom error |

Each group contains nine decoded inputs including three valid controls. The source first checks exact byte length and then ASCII digit/a–f classes. Error display prefix/JSON location depends on the consuming deserializer and was not characterized. Escaped decoded IDs, negative-zero signed/unsigned lexemes, quoted numeric forms, JSON numeric overflow mapping, duplicate raw JSON keys and unknown-field handling through aggregate consumers remain separate Pending ingress cases. [crates/nyon-workshop-core/src/ids.rs:43,71,209; task-4a-foundation-vector-seams.md, § Raw schema, validation and capacity boundary]

Additional checked-distance finite pair cases: (0,0)→(255,0) gives255; (0,0)→(256,0) gives256; (0,0)→(257,0) gives257. The later route consumer derives1,1,2 travel ticks respectively using `(distance+255)/256` and minimum1. These supplement the reviewed diagonal/floor-distance cases; no new legal-coordinate overflow case is claimed. [crates/nyon-workshop-core/src/ids.rs:234; crates/nyon-workshop-core/src/simulation.rs:375]
