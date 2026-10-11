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
