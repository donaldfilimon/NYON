# Proposed V3 independent wire formula corpus

Status: **PROPOSED / PARTIAL gap6**, 2026-10-02. This directory is separate from
`../living-v3-proposed`; neither corpus is a codec or implemented V3 authority.

Run from the repository root:

```sh
python3 docs/superpowers/specs/fixtures/living-v3-wire-proposed/derive.py generate
python3 docs/superpowers/specs/fixtures/living-v3-wire-proposed/derive.py verify
```

`derive.py` uses Python's standard library and frozen written formulas only.
`generatedvectors.json` is generated; never hand-edit it. Verify recomputes inputs,
canonical bytes, rejection checks, identity frames and literal hashes and compares
all generated output bytes. No expected output is read from Rust or the V2 engine.

Both frozen wire/schema SHA256 identities are recorded and checked before deriving.
The catalog copies the schema's documentary V2 input snapshot and appends the exact
Proposed additions in complete-schema §Catalog schema and society §§Added catalog
definitions/Research. Synthetic pack/generator labels and entity/slot inputs are
explicit author-chosen examples, not an available shipped V3 generator or pack.

Coverage is deliberately labelled by strength:

| Coverage | Current evidence |
| --- | --- |
| Complete document bytes |14 subset-valid canonical documents: empty catalog/state/genesis/archive/Notebook/transfer/coordinator; minimal populated state/genesis; systems64; three Unicode-name states; exact T0 bookmark |
| Wire rejection cases |32 cases: version/catalog/kind/enum/unknown/missing/order/null/decimal/u64/hex/order/reference/ID/capacity/integrity/name/Unicode/float/duplicate/UTF8/byte-cap rejection; wrong bookmark digest |
| Scalar/record subset |24 primitive/shape cases including u64 bounds, acyclic threat hash and wrong digest, Supply first unavailable/prior unexpired/prior expired/forged payment, exact creator capture, immutable paid/creator manifest shapes and waiting offer fact |
| Literal framed hashes |32 vectors containing input_hex, SHA256 and16-byte ID when applicable: catalog/wrapped State/genesis/root, all18 genesis kind tags, creator stellar, null/non-null fork, full Revision, lifecycle/actor namespace frame, Receipt/Event, threat, immutable Terms |
| Exact intermediate data |Canonical payload UTF8 beside hashes; genesis G bytes; CreatorCapture independent of future revision; original immutable commitment binding |

Negative expectations separate local predicates from normative transport tags.
Each has `must_reject:true`, `local_diagnostic`, nullable `expected_wire_error`
and `claim`. `exact_wire_tag` asserts a tag literally in the frozen closed enum;
`rejection_only` asserts rejection without prescribing a transport tag where the
frozen spec supplies no exact predicate-to-error mapping. Missing fields, local
shape/type/name/decimal/hex/sorting/malformed-byte and forged allocation diagnostics
are not new wire variants. Primitive negatives use the same distinction.
Format and rules versions have separate exact tags; a reference naming an existing
World where System is required is `wrong_reference_kind`, while an absent ID is
`missing_reference`. Unknown fields use `unknown_field`. Alternate escape/whitespace
bytes use `noncanonical_bytes`. Codec comparison must not compare local diagnostics
as transport outcomes, and must check only rejection for `rejection_only` cases.

`valid_subset` and document inclusion mean the listed validators accepted those
explicit inputs. They are not exhaustive V3 admission validation. Partial structural
validators reject unsupported nonempty families as `outside_subset`, rather than
silently accepting records they do not inspect. Record shape examples do not claim
executed allocation, visibility, offer capacity, arrival or treaty fulfillment.

Framing-only inputs are named as such: the empty CreatorBatch is not asserted to be
an admissible command, the standalone Event frame is not asserted to occur in the
empty Receipt, the standalone creator Stellar record's star is not in empty
Genesis, and actor autonomous framing does not assert a permitted observation
capture. They test exact width/order/domain/NUL/hash mechanics, not valid history.

Open coverage includes every command/Record/TypedKey union, all populated entity
families and enums, nonempty revision replay, anchors/checkpoints/evicted receipts,
bounded collision scans/collisions, wrong namespace/kind/sequence, nonempty event
facts, experiments/metrics, resource conservation and full scoring/diplomacy/
forecast/stellar application simulations. The frozen coordinator has no declared
byte cap; the corpus imposes none, and reports this written-contract omission.

Independent formula and corpus review must check both derivation code and literal
outputs before any V3 codec consumes them. Generation+verify proves deterministic
self-consistency only. It does not close fullgap6 or imply written/user acceptance,
platform/performance qualification or implemented behavior.
