# Current Proposed V3 history corpus (HIST-C)

2026-10-02. **Proposed; bounded author corpus, awaiting independent review.**
This folder qualifies one empty-root history family against the frozen current
eight-document contract. It does not implement V3 or establish human approval,
runtime admission, a complete decoder matrix, or complete game coverage.

## Reproduce

From the repository root, with Python 3 and its standard library only:

```sh
python3 docs/superpowers/specs/fixtures/living-v3-current-proposed/history/derive.py generate
python3 docs/superpowers/specs/fixtures/living-v3-current-proposed/history/derive.py verify
```

Generation writes only the six JSON products in this folder. Verification
reconstructs them and compares exact bytes. Both modes first check immutable
dependencies and all eight live/captured specification bytes against their
frozen lengths/hashes; they repeat freshness before writing or comparing. A
changed source, review, or accepted CAT/GEN dependency stops the command rather
than silently updating an expectation. README is authored documentation, not a
generated product. No package installation, Rust serialization, prior history
generator, or CAT/GEN helper import supplies expected values.

## Complete admitted fixture family

The base path starts from the independently admitted complete empty Genesis and
fixed Catalog. At completed tick0, paused CreateSystem Alpha accepts sequence1;
paused EditSystem Beta accepts sequence2. Ordinary boundary1 then advances the
tick with no new Revision or Event. The final State has all30 declared fields,
one System, all other collections empty, branch_sequence0, next_sequence2 and
tick1. Names and Bookmark labels are fixture choices, not additional grammar
rules. No inhabited Hub receipt policy is selected.

Each paused step captures the pre-command completed parent/digest/phase1 and
has one complete direct Change. Creation uses the CreatorEntity flag2 frame;
editing retains that ID and issuance origin. Header orchestration changes do
not enter reviewed_effects or Facts.change. Each paused Receipt has precisely
one phase1/ordinal0 creator_intervention; ordinary1 has no applied Revision or
Event. Public Event IDs are calculated after the ID-free Receipt payload seal.
There is no initial Receipt and no same-receipt hash cycle.

The Archive carries the exact empty Genesis, two ID-sorted Revisions, root
Branch head/tip/marks, final wrapped State digest and three authentic whole-
Receipt Anchors. Replay follows parent/application chronology separately from
array ID order. Anchors sort by the full key and authenticate each original
published view and Receipt; they are not substituted with the current head.
The Notebook contains one State-digest Bookmark, empty experiments/evidence,
and exact Genesis/Archive linkage. Transfer carries all complete inner inputs,
a reachable selected view and its own seal. HistoryView has canonical bytes
and root/state bindings; no standalone VIEW domain or digest is invented.

Eight complete positive documents are checked:

| Positive family | Documents | Purpose |
| --- | ---: | --- |
| Alpha → Beta, Boundary One | Archive, Notebook, Transfer (3) | Entire base path and linkage |
| Alpha Ω → Beta β, Boundary Ω | Archive, Notebook, Transfer (3) | Grammar-valid UTF-8 metadata with every frame resealed |
| Notebook next_ordinal9, retained Bookmark ordinal1 | Notebook (1) | Legal common-mark holes, not an equality requirement |
| Transfer selected R2/tick0, Archive active R2/tick1 | Transfer (1) | Reachable retained selection may differ from active_view |

The base Transfer selects the final view by fixture choice. Its selection is
not a universal equality rule. The negative selected tick2 is unreachable and
has rejection-only strength. Embedded Archive/Notebook header constants are
checked before downstream history/link use; coherently resealed violations
also remain rejection-only because their full normative diagnostic ownership
is not strengthened from helper execution order.

## Files and framing

| File | Role |
| --- | --- |
| derive.py | Literal constructor, narrow replay/admission, freshness checks and vector derivation |
| archive.json | Complete canonical base Archive |
| notebook.json | Complete canonical attached base Notebook |
| transfer.json | Complete canonical base Transfer |
| trace.json | Non-wire audit artifact: envelopes, captures, Changes, States, public Receipts, Events and all preimages |
| vectors.json | Raw encoded complete positives/negatives and separate trace-binding cases |
| predicate-coverage.json | Pins, causal source locators, 380 declaration nodes, strength counts and explicit gaps |

Domains use ASCII `NYON_LIVING_V3_…` plus one NUL; fixed integers are
little-endian. Canonical JSON preserves declared field order. Length framing
uses u32le byte length; sequence/tick frames use u64le; optional Revision uses
0 or 1 plus32 bytes. State hashing includes the version3/PACK wrapper. Archive,
Notebook and Transfer exclude only their own final integrity member when
sealed. All inner links are finalized before enclosing seals.

The source/dependency pin ledger is in derive.py and generated coverage. Its
source manifest SHA256 is
`d0ab6b0d86808e6f9f3fabfeb503e1a143d7b108c1e49468bb94dd88ec206c8c`;
checkpoint2 manifest SHA256 is
`bab14622dfb118e45801f228e1153309641f72b3a50a8d84eb3cd0466a46bd3f`.
The exact Catalog dependency has4264 bytes and SHA256
`efc762006d5abfa3e3a00d3aa9ba46b001e9e1951833b1c655ab6518b18f3f10`;
PACK is `9720f69feae4b1b04cc3459b4a156153c284eae8658197d922a142e634191da8`.
The accepted empty Genesis has805 bytes and SHA256
`4c5db9ebe76e19b36e76cd7333934f8697b81303001d99c10c27f35448ed03a0`.
Every five CAT and six corrected GEN files is pinned without mutation.

## Diagnostic strength and coverage

There are **437 raw cases**:8 complete positives,396 exact-stage/error/path
negatives,3 exact-stage/error-only negatives, and30 rejection-only negatives.
The last group intentionally exports no private helper path or order as a
normative expectation. All semantic tampering that needs to reach replay or
inner linkage is coherently resealed. `outside_family` is not a wire error.

The exact-path cases cover377 required-member omissions in populated records
(including required Transfer /catalog, excluding the separately admitted
immutable Catalog interior under /catalog/),9 unsupported
top-level headers,3 outer integrity faults,3 scalar spelling/domain faults,
2 wrong PACK reference faults, and2 array sorting faults. Missing root headers
use stage4; other structural omissions use stage5. Exact exported negative
stage counts are stage4:18, stage5:373, stage6:3, stage8:2, stage9:3. Canonical
leading whitespace has only stage/error strength. No semantic multi-fault
precedence is inferred from this helper.

The additional **10 trace-binding rejection checks** mutate actual replayed
public State/Receipt/Event outputs and compare against independently replayed
Archive publications. They are audit-harness checks, not extra portable
fields, authoritative raw diagnostics, or runtime tests. There are no Receipt
bytes, queues, checkpoint/cache flags, or completion bits in the Archive.

Source locators in the ledger bind pre-command captures (schema2446–2458),
direct Changes (1723–1740/2438–2442), framing (2370–2423), aggregate publication
(2460–2485), Receipt/Event partition (2329–2340 and wire149–154), history and
Anchors (2575–2589/2630–2648/2692–2697), Notebook (2497–2511/2654–2659), and
Transfer (2614–2617). Those locators refer to the frozen sources, not a moving
file revision.

## Remaining coverage limits

Only the empty root, one creator System origin, two paused single-operation
edits and ordinary1 are qualified. Richer Genesis roots, inhabited autonomous
receipt policy (including T50), forks/rekeys, running queues, multi-operation
cascades/removals, transient sibling collisions, Event-specific Anchors,
Notebook experiments/evidence and comparison/causal completion are separate
families. Unsupported configurations are coverage boundaries, not new bans.

The author decoder has bounded-depth syntax scanning, duplicate-key retention,
schema/scalar/canonical checks and narrow linkage/replay. This corpus does not
qualify its whole raw/depth/duplicate/parser family, general stage7–10
multi-fault matrix, production memory/cancellation/job budgets, retained-history
collision performance or platform durability. No initial-profile validator is
applied to evolved States. No runtime, public API, source, eight-spec file or
accepted CAT/GEN file was modified for HIST-C.
