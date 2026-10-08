# Derivation provenance and boundaries

All current expected documents are formed here from written fields/formulas.
No Rust serializer, runtime output, generated V2 asset, old expected digest or
historical vectors file supplies the current expected bytes.

The independent historical helpers were inspected as references. Their code was
copied into this separate namespace, then changed against the corrected source.
They are not silently imported and their old pins/outputs remain unchanged:

- `living-v3-current-proposed/derive.py`, old SHA256
  `c8d772c48420b1dcf1c3598ceba35bc6ab59baa08e87fb7bd5e2fb917757ae51`:
  the Catalog literal builder, canonical encoder, nonrecursive tokenizer,
  finite-mechanics/schema checks and finite case generation are retained in
  `catalog.py`. Compared against complete-schema8–194,1225–1288,1679–1835,
  4197–4487 and society catalog/research tables. The required tag was changed to
  `carrying_capacity_penalty_scale`; the new source authority checker replaces
  all old release pin logic. This is substantial reused code, not a claim that
  the entire Catalog module was freshly authored.
- `living-v3-current-proposed/genesis/derive.py`: frames, root sealing, initial
  field skeleton and pure visibility/observation math were copied into
  `genesis.py`. Compared against schema194–509,984–1288,3742–3822,3961–4008,
  4088–4143,4197–4230,4760–4792 and wire state/identity/cap clauses.
  Old expected PACK and source pins are removed. Admission uses tokenizer nodes
  retaining number spellings, required voyage_origin and defense_request are
  added, fixed eleven-ID count equality is removed, Catalog linkage is computed
  from supplied valid metadata, installed traversal includes each nested Hull
  at its actual schema position, and general explicit base rows are generated
  by the new `profiles.py`. The old helper is not general admission evidence.

The frozen source says Lane endpoints are distinct (schema4101); this helper
therefore does not silently impose the old helper's unsubstantiated ascending
endpoint predicate. Lane arrays still sort by issued ID. A reversed distinct
endpoint pair is retained as an explicit review case. If a further inherited
constraint is intended, its precise immutable owning clause must be identified
before changing these expected bytes; runtime behavior is not its oracle.

All byte/identity claims should be independently reviewed. The parser and builder
share canonical encoding; their mutual agreement alone cannot prove correctness.
The new predicate matrix preserves this limitation. FIX1 adds richer-record shape declarations and direct-reference traversal from
the written schema in separate stdlib modules. Exhaustive semantic enforcement,
per-predicate witnesses and all-eight semantic decomposition remain unfinished,
distinct from genuinely unadopted initialization semantics. The original stopped
package remains immutable evidence of the preceding partial state.
