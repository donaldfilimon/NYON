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
