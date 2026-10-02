# Living notebook durable receipt references

Status: Proposed implementation detail of the accepted seven optional
experiments and portable evidence contract. No additional gameplay, reward,
time limit, authority command, or simulation identity is introduced.

The current notebook validates exact state references and matching-tick
measurements, but discards suggested-experiment classifications and supporting
ordinary receipt references on save/reload. This is an acceptance gap.

Add an optional ordered `evidence` collection to the notebook payload and
integrity payload, omitted when empty. Missing means empty for old documents.
Existing empty-evidence notebook bytes, archives, state identities, and frozen
Living vectors remain unchanged. Maximum 128 entries within the existing
1 MiB total notebook cap; no receipt sidecar or checkpoint export.

Each entry contains a never-reused ordinal, an existing bookmark ordinal,
a typed experiment identifier from the accepted seven, an ordinary receipt
digest, and one or more ordered event references (event ordinal, full stable
event ID, and event kind). Evidence denotes a verified observation, not
completion of an experiment. Multiple entries can support a multi-stage
experiment; incomplete required witnesses remain explicitly incomplete.

Notebook validation reconstructs the referenced exact bookmark against its
validated archive, checks state digest, and checks the reconstructed ordinary
receipt digest and referenced event IDs, ordinals, and kinds. A wrong receipt,
missing event, unrelated creator intervention, unavailable view, duplicate
ordinal, invalid experiment kind, or cap overflow rejects the whole candidate.
Receipt facts needed to claim parties, endpoints, quantities, or causal impact
are checked against authoritative replay producers; until those exist, the UI
may show a verified event reference but may not certify that broader outcome.

Session save snapshots include these references; install restores them only
after notebook validation and exact resident retained-history attachment.
Old notebooks remain readable and honestly display missing receipt evidence.
Cancelled validation and failed save/install preserve all resident metadata.

Independent review precedes implementation. Independently calculated canonical
bytes and integrity fixtures must cover absent/empty preservation and a populated
reference. Regressions must cover reload, native/browser attachment, forged
digest/event/kind/index, stale history, cancellation, and mixed paused/running
revision boundaries. Whole-path visited-entity budgeting remains a separate
required implementation; logical replay polls are not its proof.

## Pinned proposed shape after independent review

The optional final payload field is `evidence`, immediately after `experiments`
and before outer `integrity_sha256`. It is absent on old notebooks. Its section
contains `next_ordinal` (u64, using the existing notebook ordinal encoding) and
`records`, in that order. Once created, the section is retained even if a future
operation removes all records, preserving its allocation high-water mark.
Explicit null for absent evidence is noncanonical and rejects by re-encoding.

Each record has fields, in order: `ordinal` (u64), `bookmark` (u64),
`experiment` (tag string), `receipt_digest` (existing 32-byte digest type),
and `events`. Exact experiment tags are `supply_chain`, `autonomous_settlement`,
`trade_delivery`, `corridor_impact`, `war_alternative`, `dormant_recovery`,
and `portable_reproduction`. Records are strictly increasing by ordinal; every
ordinal is below `next_ordinal`, which never decreases within the document.

Each event reference has `ordinal` (u16), `id` (existing 16-byte event ID), and
`kind` (existing event-kind registry), in that order. A record has 1..=32
references in strictly increasing event ordinal order, with no repeated ID.
The section has at most 128 records; total canonical notebook size stays1MiB.
Unknown tags, invalid counters, duplicate references and noncanonical empty
representations reject before replay.

Replay must match receipt tick AND receipt state digest to the exact bookmark,
then digest and each event identity, ordinal and kind. A post-production paused
edit at T cannot use the previous ordinary T receipt for its new state: require
the existing exact pre-edit bookmark/cursor or report unavailable evidence.
Provenance and producer facts determine experiment support after identity
verification; a creator event can be an intervention observation but cannot
prove autonomous settlement or a negotiated agreement. Creator recovery
requires an exact replay-derived whole-revision transition proof.
Any absent participant/quantity/endpoint/causal fact remains explicitly unknown.
Verified observations alone never certify all experiment stages or causation.


Import identity boundary: evidence ordinals belong to the source notebook
(document metadata), not the galaxy authority's revision or branch namespace.
Replacing a notebook retains each validated imported record's source ordinal.
The resident Session carries forward the maximum allocator highwater so that
later local records cannot reuse an ordinal allocated before replacement.


Creator recovery observations preserve the existing CreatorIntervention event.
They require the authority replay-produced whole-revision transition proof,
bound to the exact revision and receipt, and a surviving final owned colony.
This does not certify an autonomous recovery or introduce a new status rule.
See `2026-10-01-living-recovery-evidence.md` for admission and exclusions.
