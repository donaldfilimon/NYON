# CAT-C: current Proposed complete Catalog corpus

This fresh corpus derives the complete finite V3 Catalog from the eight-spec
checkpoint2 freeze. It does not import an old validator, fixture generator,
V2 asset or Rust serializer output. `derive.py` assembles the source's literal
inherited values and required V3 additions, then independently reads candidate
raw bytes through the declared Catalog stages. All V3 mechanics remain Proposed;
human rules approval and runtime implementation are absent.

Run from the repository root:

```sh
python3 docs/superpowers/specs/fixtures/living-v3-current-proposed/derive.py generate
python3 docs/superpowers/specs/fixtures/living-v3-current-proposed/derive.py verify
```

Both commands check every live and immutable-copy source's SHA-256 and byte count
before deriving, and again before output publication/verification. They also pin
checkpoint2's manifest, the immutable-copy manifest and the integrated/Anchor
written reviews. A mismatch stops; neither command refreshes source pins. `verify`
compares regenerated complete output bytes, not just counts or stored digests.
Stdlib only; no dependencies, network, Cargo, product or historical-corpus access.

The fixed inputs are `.superpowers/sdd/2026-10-01-sandbox-completion-and-expansion/`:

- `v3-current-release-source-freeze/manifest.json`, SHA-256
  `d0ab6b0d86808e6f9f3fabfeb503e1a143d7b108c1e49468bb94dd88ec206c8c`.
- `v3-coordinated-amendment-current-source-manifest-2.json`, SHA-256
  `bab14622dfb118e45801f228e1153309641f72b3a50a8d84eb3cd0466a46bd3f`.
- Integrated review `cd485f38a06964734e435bb245025579a40ee59da176cc3f1c1c85a07a98ca65`
  and Anchor delta review `947111491e1467c6a122817c0a57c34d9afe384c4f3ae4ded8ff275608ca7ecf`.

The source manifest in every generated document names all eight current hashes.
Complete-schema owns scalar/record/key/cap/Catalog/decoder contracts; societies
owns new industry and technology numbers; wire owns PACK domain/framing. Source
section names are stable locators; source line numbers are not treated as inputs.

## Literal documents and strength

`core.catalog.json` is the complete compact canonical Catalog, 4,264 bytes with
no trailing newline. Its literal Name metadata follows the source snapshot and
new named definitions. Required V3 fields include Food, all eight industries,
explicit workforce/stress, three hull roles and three technologies. Policy bonuses
remain the literal frozen roster; adding an allowed IntentKind tag does not add
a bonus. ResourceAmount inputs sort by resource slug, so the lab is Alloy1 then
Energy2. Metadata Names do not infer roles or mechanics.

PACK preimage is exactly `NYON_LIVING_V3_PACK` plus one NUL and those complete
Catalog bytes, with no integrity field or length prefix. The freshly reconstructed
core digest is `9720f69feae4b1b04cc3459b4a156153c284eae8658197d922a142e634191da8`.
Equality to a historical numeric value does not indicate copied old outputs: the
constructor, canonical bytes and complete base64 preimage are available here.

`vectors.json` contains551 complete raw documents:41 positives and510 negatives.
Raw inputs are literal base64 bytes, including invalid UTF8, duplicate decoded
members and exact-cap whitespace. Positives are **complete Catalog admission**
under this finite profile, not Name leaves: each of28 existing Name paths varies
in a full pack; all allowed metadata varies together; slug1/64, version1/u32MAX,
Name1/64 scalars/256 bytes, literal replacement character, decomposed/composed
Unicode, permitted non-ASCII spaces/separators, quote/backslash are exercised.
They authenticate new exact PACK bytes, not metadata normalization.

Negative strength `exact_stage_error_path` pins only normatively decided stage,
closed error and JSON Pointer, plus declared capacities when applicable. The
reordered-root canonical case is `exact_stage_error_only`: it does not invent
a universal pointer for a differing key token versus containing object. Byte
offsets produced privately by the helper are not new persisted diagnostic fields
or claimed exact-offset vectors. Isolated key failures and explicit selected
multi-defect cases are supported; all malformed child/key combinations are not
claimed exhausted.

Two separate consumer-link checks bind the altered complete metadata pack to its
matching or wrong expected PACK. They are primitive external expected-hash checks,
not fabricated Genesis/State/Actor/Transfer envelopes or root/entity identities.
They pin stage8/catalog_mismatch, without exporting the helper `/catalog_hash`
path as a universal consumer pointer. A Catalog has no self-integrity field.

## Admission and coverage boundaries

`predicate-coverage.json` lists the full positive admission stage ledger,203
scalar paths, complete record/array field orders including explicit empty lists,
all industry/technology/hazard effect variants, three roles and22 IntentKind tags.
Every numeric mechanical scalar has a profile-mutation negative; scalar widths/
types, all eight roster/key/cap families and the28 Name metadata paths have their
scoped cases. Explicit references resolve resources and the technology DAG in
each admitted complete pack.

The reader uses an explicit container stack for syntax/depth and retains object
pairs until duplicate decoded keys are checked before map materialization. Schema
traversal, canonical equality and finite mechanics remain separate stages. Header
order precedes body checks; unknown object members precede missing fields; collection
caps precede children; Name/Slug/type/width predicates are structural5; exact
fixed mechanics/ratio domains are semantic7 after canonical6. Denominator0 is a
prerequisite failure at its denominator path. Fixed Catalog2/4 cannot replace1/2.
Zero deposit/damage/empty solar input stay legitimate; listed actual fixed recipe
cost/output entries cannot be changed to zero.

An absent resource or technology mutation changes the finite profile and therefore
fails semantic7 before stage8: these are deliberately **not** advertised as
missing_reference/invalid_graph negatives after a bypassed earlier guard. Positive
Catalog-local references and graph checks are complete for the finite roster;
generic typed-reference collision/history precedence belongs to later families.
Stage7 scalar/profile comparisons use declaration/index order; missing/extra
definition-ID rosters point at the collection, unequal submitted values at fields.

Machine-readable `uncovered_or_stopped` distinguishes these earlier-guard cases,
unknown stronger diagnostics, general admission and runtime scope. No
outside_family authoritative error is added. This corpus proves neither general
State/Genesis admission, Actor32/replay/forecast transitions, entity/root frames,
durable host transfer nor native/wasm product codec agreement. It cannot close
the full semantic matrix or six gaps. Later corpora must reconstruct those
families under their own explicit releases; historical folders remain unchanged.

This fixture oracle has no production responsiveness/ownership certification.
Native/wasm, cancellation, segment work, store generation and publication tests
are separate runtime evidence. Independent corpus review remains required.
