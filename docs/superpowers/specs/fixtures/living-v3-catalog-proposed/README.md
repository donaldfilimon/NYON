# Proposed V3 exact-core Catalog corpus

This separate family derives the fixed core Catalog from the frozen written package. It does not replace historical corpora or approve V3 rules. Full six-gap coverage remains **Partial**.

Run from the repository root:

```sh
python3 docs/superpowers/specs/fixtures/living-v3-catalog-proposed/derive.py generate
python3 docs/superpowers/specs/fixtures/living-v3-catalog-proposed/derive.py verify
```

Only `generate` writes generatedvectors.json and coverage.json. `verify` reconstructs expected bytes in memory and compares both files byte for byte. No engine code, engine serializer, engine generator or production dependencies are imported. Both commands reject a changed eight-spec source freeze.

The admitted fixture has the complete ordered Catalog envelope and recursively checked nested records. The validator checks all five IndustryEffect tags, the HazardEffect tag, all three TechnologyEffect tags, all three HullRole tags, the closed 22 PolicyBonus intent registry, u32 leaves, Name/Slug leaves, strict ordering/uniqueness, ResourceAmount references, technology references/cycles, collection caps and exact fixed core values. The PACK preimage is the literal UTF-8 domain `NYON_LIVING_V3_PACK`, one NUL, then complete canonical Catalog bytes. There is no Catalog self-integrity field. These rules are taken from complete-schema §§Scalar grammar, Catalog schema and definition registry, Documentary initial catalog snapshot, Hash framing; society §§Added catalog definitions and Research; and wire §§Wire contract and Catalog (source hashes are embedded in both outputs).

The first reconstruction parses the documentary initial V2 snapshot solely as a source table, adds the explicit V3 rows/fields and produces V3 bytes. The second transcribes the complete final core as literal tables. They must agree byte for byte. They share the canonical encoder and reviewed addition values; this is cross-reconstruction evidence within one independent tool, not a claim of external independent review.

One exact core Catalog is admitted; 158 document cases reject; 30 positives validate only Name/Slug/intent leaves; four cases show hash sensitivity. Raw cases include duplicate keys, alternate encodings, invalid UTF-8/JSON, whitespace, field order, integer forms, depth65 and cap-first malformed input. Semantic negatives cover every effect variant's unknown tag/missing/extra/wrong payload, nested fields/order/keys, references/cycles, fixed balance, resource omission, every definition cap+1 and ResourceAmount cap+1. Some rejection witnesses violate more than one predicate, as their diagnostics explicitly show. Parser depth counting occurs before general JSON parsing and respects quoted strings and escapes; its execution order is a tool choice, not a specified error priority.

`diagnostics` are local predicate evidence, **not** promised wire error tags. `expected_error` is null except the explicit >1MiB raw byte-cap case, whose exact `byte_capacity` precedes decoding/parsing. All remaining cases assert rejection only because the frozen package does not fully bind leaf error mappings or competing-error precedence. Graph and capacity witnesses list `invalid_graph`/`record_capacity` predicates without inventing a normative first error. Repeated Catalog slugs are not assigned entity `duplicate_identity`.

Exact-core admission here means the literal fixed balance and valid metadata instance supplied by this family. It does not mean its newly added title-case display names are the only possible names accepted by V3. Permitted custom metadata, definition roster variants, cross-family slug reuse, custom bonuses and differently balanced catalogs are not resolved by this tool. Leaf positives therefore do not become admitted custom packs. The source does not type the pack_version envelope width explicitly; this family admits inherited value1 and rejects0 without asserting any width-edge acceptance. General scale/quantity ranges beyond the fixed effects remain unbound. Coverage.json records these limits and absent State/Genesis/history/actor/mechanics coverage.

Review correction: case IDs are unique and checked during generation. Ten additional malformed ResourceAmount/resource/container cases exercise guarded reference membership. These are sampled malformed shapes; the tool is not a complete general Catalog decoder or exhaustive malformed-input proof. The exact admitted core bytes and PACK digest are unchanged.
