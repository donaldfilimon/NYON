# Corrected-source V3 foundation — Partial

This namespace derives Proposed Catalog and explicit B0–B7 bootstrap inputs from
`v3-ia2-completion-lane-freeze-2/manifest.json`. It is not a completed full-corpus
submission, a production decoder, or human/runtime approval. Independent review
must examine derivation and literal files; a green helper is consistency evidence.

Run from the canonical repository with Python's standard library:

```
PYTHONDONTWRITEBYTECODE=1 python3 docs/superpowers/specs/fixtures/living-v3-complete-proposed/derive.py generate
PYTHONDONTWRITEBYTECODE=1 python3 docs/superpowers/specs/fixtures/living-v3-complete-proposed/derive.py verify
```

`generate` writes only declared generated outputs below this directory. `verify`
rederives in memory and compares bytes without writing files, refreshing pins,
accepting source drift, or updating expectations. Both check exact eight frozen
and live source identities and all 66 historical protected files before and after
derivation. `helper_hashes.json` makes helper changes detectable by verification.

`catalog/core.catalog.json` is the freshly formed 4,269-byte Proposed Catalog.
Its corrected carrying-capacity effect tag creates PACK digest
`52a64c125d4be640b6887c014fa620b390764a870ecbb8e52c8d7015c69521bd`.
Catalog mechanics retain the finite roster; all permitted metadata is general.
No old golden or old pin is changed or imported as a current expected output.

`documents/` contains complete explicit Genesis documents and matching State
hash wrappers. These include empty/inhabited inputs, allowed metadata/catalog
changes, noncontiguous/u64-limit allocations, active/dormant/population variants,
Unicode, each base collection maximum and global hull/observation maxima. The
admission helper checks submitted derived fields; it never fills or repairs them.
`initial-state-and-root-identities.json` stores canonical bytes and exact frame
preimages, full SHA256, truncated IDs, root/view and initial marks. All18 frame
rows are **framing-only**; they do not manufacture richer Genesis or history.

`genesis-cases.json`, `structural-cases.json`, `foundation-predicate-coverage.json`,
`visibility-leaf-cases.json`, `root-binding-cases.json` and `raw-cap-cases.json`
retain distinct evidence strength. Structural/visibility/context cases do not
claim full-document admission. Rejection-only expectations never publish a
helper-selected semantic error as an exact interoperable diagnostic. No genuine
SHA256 collision is claimed.

`foundation-predicate-coverage.json` decomposes implemented schema paths and
initial semantic predicates. FIX1 declares the previously omitted richer record
and union shapes; declaration and recursive shape controls do not close their
local, cross-field, reference, history-origin or initialization semantics. `full-package-coverage.json` includes a lossless all-eight source
line index and named pending packages. That index is **not** the requested
exhaustive semantic decomposition; producing it and completing every predicate's
positive/negative evidence remains foundation work. The entire schema was not
read end-to-end in this implementation pass. Later replay/Actor32/200-boundary
forecast/diplomacy/War/paid Supply/Defense/Peace/history/annex obligations remain
pending, with no claimed vector or runtime coverage.

The source itself leaves richer initial contracts unadopted at complete-schema
4088–4143. It specifically requires separate chronology/source policies for
facilities/jobs/routes/hazards/stellar events/diplomacy/claims/occupations. That
is distinct from unfinished helper coverage. `Unsupported` is an **unexamined
coverage result**, never successful admission or an invented wire error. General
production wire acceptance cannot be inferred from this bootstrap helper.

See `provenance.md` for reuse and comparison details and the stopped implementation
report in the expansion SDD directory for exact remaining work and receipts.

FIX1 adds exact reference-family ordering and diagnostic-detail controls in
`fix1-regression-cases.json`, and the complete metadata State wrapper.
`needs_reserved` has declared shape and separate local-domain controls; its
local validator is not yet connected to every nested full-document occurrence.
These controls never permit prior-attempt bootstrap facts. See the FIX1 Partial
report for remaining F2/F5 integration and exhaustive evidence obligations.
