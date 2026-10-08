# GEN-C current Proposed Genesis corpus

Status: **Proposed selected-profile fixture qualification; independent review next.**
This folder is new. Its Catalog dependency and all eight rules are frozen;
accepted V1/V2, historical corpora, and the accepted parent CAT-C remain unchanged.
No runtime, full game, general reader, or human mechanics approval follows.

From the repository root:

```sh
python3 docs/superpowers/specs/fixtures/living-v3-current-proposed/genesis/derive.py generate
python3 docs/superpowers/specs/fixtures/living-v3-current-proposed/genesis/derive.py verify
```

Both commands check source/dependency freshness before constructing outputs and
again before publishing or comparing bytes. A changed dependency stops; pins
are never silently refreshed. `verify` recomputes and compares all four generated
files exactly. README is authored, not generated. Only Python standard libraries
are used. No old generator/validator, Rust serializer/output, or CAT-C helper is
imported. The accepted Catalog **bytes** are an immutable interpreted dependency,
not an expected-value oracle for Genesis identities, observations or hashes.

## Complete documents and independent framing

`empty.genesis.json` is the explicit empty control, 805 bytes. Every required
State field is present, including all26 empty arrays and counters `{}`.
`inhabited.genesis.json` is the 4518-byte named
GEN-C-MIN11-HUB-DEPOSIT-DOCKED-SCOUT document: two Systems/Stars/Worlds, one Lane,
Civilization, Deposit, Colony, docked Fleet/nested scout Hull, and direct home
Observation. Its other17 State arrays are explicit empty arrays. No top-level
Hull collection exists. There is no omitted default, self-integrity field,
initial Receipt/Anchor/Capture, paid Shipment, or portable command queue.

The fixed names/topology/stocks/environment/population are **fixture choices**.
The validator does not require Name equality or sample stock/population values.
The six complete positives are empty control, named inhabited sample, valid
Unicode Name, zero home stocks, population zero with a still-owned Colony and
Active actor, and fresh identities with noncontiguous positive ordinals. The
latter recomputes every reference/sort/projection, rather than replacing metadata.
All six receive independently derived sealing records in `vectors.json`.

Catalog core4264 bytes is pinned by file SHA256
`efc762006d5abfa3e3a00d3aa9ba46b001e9e1951833b1c655ab6518b18f3f10`;
its independently rechecked PACK is
`9720f69feae4b1b04cc3459b4a156153c284eae8658197d922a142e634191da8`.
Source-copy manifest is
`d0ab6b0d86808e6f9f3fabfeb503e1a143d7b108c1e49468bb94dd88ec206c8c`;
checkpoint2 is
`bab14622dfb118e45801f228e1153309641f72b3a50a8d84eb3cd0466a46bd3f`.
The exact integrated, Anchor, CAT-C review, preflight and all five accepted CAT
files are checked. All eight live documents and immutable copies are compared
by length and hash. No historical expected ID/hash is read during construction.

Output includes G, all11 complete GENESIS_ENTITY frames and full digests,
State canonical bytes, version3/catalog-bound STATE wrapper/preimage/digest,
complete GENESIS preimage/digest, ROOT_BRANCH frame/full digest/first16-byte ID,
Branch, HistoryView canonical bytes, initial zero high-water marks and empty
history context. All18 kind tags are recorded, but only the eleven relevant
issuances are exercised. Colony/Observation keys have no allocation row.

There is **no declared standalone VIEW domain**. HistoryView bytes and exact
root/state binding are exported without inventing a View digest. Root context
is outside Genesis, with no circular branch reference in Genesis preimages.
`queue_harness` is explicitly nonportable context, never a State/Genesis field.
Full Archive admission and authenticated retained history are separate work.

## Admission and causal source predicates

The selected fixture reader validates required/order/scalar/cap/key properties,
complete global installed-ID uniqueness including nested Hulls, positive unique
per-kind ordinal declaration/live bijection, exact frames, typed entity and
Catalog references. It checks B2 zero State clocks/counters and initial
activity/environment/hub/needs clocks, local10000/storage bounds and fertility
bounds. It checks Active iff owning a Colony, initial empty Research, recomputed
empty workforce with no facilities, full Catalog scout HP, zero passengers,
no credit/repair/order/blockade, empty blocked/Needs/Supply attempt history.

B5/B6 use only validated explicit base assets. Own World, own Colony system,
docked own Fleet world, or any role=scout Hull in an own docked Fleet system
can authorize sight. Submitted observations, travelling endpoints and public
Lane endpoints cannot authorize remote dynamic World data. The pure projection
copies owner, aggregate deposit, docked Hull counts, environment and required
zero defense/threat facts. It includes the exact empty stellar-system assertion
and each incident public Lane scope with observer/tick0/events[]. Missing or
inconsistent submitted projection is rejected, never filled or repaired.
References/frames are checked before reading B5 source operands. This profile
workflow is not an assertion that every production stage7/8/9 multi-defect
semantic ordering has already been decided.

`predicate-coverage.json` records all30 State field names, fresh structural
record declarations/array keys/caps, 132 submitted scalar paths, all18 kind tags,
B0–B7 successful predicates, source/dependency pins, strength counts and gaps.
Nonempty unsupported nested records and arbitrary simultaneous defects are not
promoted to general admission. No new balance is selected.

## Exported test strengths

`vectors.json` contains275 complete raw-document cases:

| Strength | Count | Meaning |
| --- | ---: | --- |
| complete_genesis_profile_admission |6| Complete selected-profile positives, not scalar leaves |
| exact_stage_error_path |212| Decided raw/encoding/member/header/required/scalar/key/global duplicate witnesses |
| exact_stage_error_only |1| Whitespace canonical mismatch; no strengthened path/offset |
| rejection_only |56| Source-governed profile/frame/reference/projection violations; exact semantic diagnostic selection not exported |

Every required member of every populated record, including explicit null/false/
empty members, has a missing-field witness. Other cases cover zero counters,
all initial clock owners, status/research/workforce, no prior Needs/Supply/order,
HP/passengers/credits, ownership, absent allocation/live correspondence, reserved
kind/zero ordinal, foreign/missing references, Catalog/seed frame changes,
missing/forged/remote observations and wrong copied/empty scope evidence.
Installed cross-kind and nested Hull duplicates have exact second-occurrence
stage7 paths. Raw8MiB+1 beats invalid UTF8. There are no arbitrary expected
outside_family wire errors or errors inferred by bypassing an earlier owner.

The separate18 `derived_root_context_rejection_only` cases reject changed
Branch/root State binding/View/tick/head/initial marks against computed context;
they are not fake malformed Genesis or full Archive admission. Seven
`visibility_leaf_only` fixtures exercise each visibility term, mixed scout,
non-scout world-only sight, travelling absence and observation self-authorization.
Their synthetic operands are not admitted alternative Genesis documents.

Seventeen `unsupported_family_unexamined` payloads in the ledger name each
unimplemented richer State collection. These deliberately unexamined payloads
are neither valid nor invalid wire claims. Additional asset-count/docked-role
configurations outside the supported empty/eleven-ID family return an explicit
fixture coverage result, never a new wire rejection. A production reader cannot
infer a blanket richer-Genesis prohibition from this corpus.

## Limits

General richer bootstrap chronology, jobs, facilities, environmental event
provenance, routes, paid initial Shipment, War/treaty/settlement/arbitration,
full stage7/8/9 matrix, authenticated history, Actor32, forecasts, Transfer,
store/platform and bounded job ownership remain Partial or outside this slice.
The explicit-stack syntax parser imposes depth64; subsequent bounded-depth
Python schema/materialization recursion is a fixture convenience, not a
production no-recursion/poll-budget/cancellation/memory proof. Exact private
helper diagnostic paths for rejection-only cases are not exported as authority.
No offsets become persisted diagnostics. No Receipt or consumer bonus is made
up to support initial state. Human adoption of all V3 mechanics remains absent.
