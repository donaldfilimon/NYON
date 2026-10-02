# Living supply and trade receipt facts

Status: Proposed additive implementation detail of the accepted supply-chain and
trade-delivery experiments. No new gameplay, commands, ordinary event kinds,
state/receipt/archive identity formulas or success rewards are introduced.
Existing independent Living bytes remain unchanged when the new facts are absent.

## Additive wire shape

Append these fields to `LivingEventFactsV2`, after `deposit_input`, in this order:

1. `agreement: Option<LivingAgreementV2>` with serde default and skip-None.
2. `delivery: Option<LivingDeliveryFactV2>` with serde default and skip-None.

An absent optional field means unknown. Explicit null for either absent field
is noncanonical because canonical re-encoding omits it. Both fields are covered
by the existing event-facts integrity payload and bounded 2 MiB envelope; the
ordinary receipt's payload, digest and event identities are unchanged.

`agreement` reuses the existing value shape, in order: `kind`, `participant_a`,
`participant_b`, `start_tick`, `end_tick`. Parties must be strictly increasing,
and start must precede the exclusive end. It is valid only on AgreementFormed;
paired receipt validation requires start_tick equal the emission tick. All
existing agreement kinds retain their meanings. Trade experiment classification
requires Trade, not an arbitrary agreement.

`LivingDeliveryFactV2` has fields in this exact canonical order:

- `source_world`: immutable shipment source-world identity.
- `destination_world`: immutable shipment destination-world identity.
- `dispatch_owner`: immutable sender identity accepted at dispatch.
- `intended_receiver`: immutable intended recipient identity accepted at dispatch.
- `recipient`: actual owner accepting inventory at delivery.
- `resource`: actual manifest resource.
- `units`: actual fully accepted quantity, a positive u32 at most 10,000.
- `dispatch_kind`: existing optional LivingRouteKindV2; null explicitly means the
  legacy manifest did not retain a dispatch classification.
- `source_system`: optional system of the source world at delivery; null means
  unavailable, including a source world removed before delivery.
- `destination_system`: optional system of the destination world at delivery;
  null means unavailable. A current producer normally knows it, but a decoder
  must not fill unknown values from a later state.

The last three optional fields are required nested keys, including explicit null
when unknown. They do not use sparse omission: the enclosing delivery field is
already sparse, and explicit unknown retains a single canonical nested shape.
Same-world manifests remain representable, preserving existing valid state.
Trade and two-system experiment predicates require distinct endpoints.
`recipient == intended_receiver` is
required because delivery facts attach only to ShipmentDelivered, not captures
or returns. Internal, Aid, Trade and unknown dispatch classifications retain
separate meanings; Internal requires the existing equal-participant invariant; the facts do not require different dispatch/recipient actors
for internal freight. Neither a route identity nor historic departure-system
identity is inferred: shipments intentionally retain neither.

No generic actor/entity/resource list is required to duplicate the nested typed
payload. Producers may also populate existing fields for display, but consumers
use the authenticated typed payload rather than guessing which generic entry
represents delivery. Agreement and delivery cannot coexist on one event, since
their allowed ordinary event kinds differ.

## Producers and publication

Agreement facts copy the exact bilateral proposal accepted into state, at its
actual formation event. They are not copied from a later agreement lookup, which
could have expired, changed or been replaced. Creator SetAgreement remains a
creator intervention and cannot masquerade as autonomous negotiation.

Delivery facts are recorded only after successful addition of the entire cargo
quantity to destination inventory. The producer takes immutable sender, intended
recipient, world endpoints, resource, accepted units and dispatch classification
from that shipment; recipient and optional system identities come from actual
world records at that delivery boundary. ShipmentWaiting, dispatch, arrival,
ShipmentCaptured, ShipmentReturned and discarded cargo do not produce delivery
facts. A storage-full attempt cannot fabricate zero or partial accepted units.
Existing ordinary kind, provenance, ordinal and event ID stay unchanged.

Facts are validated before publication. Wrong kind,
recipient mismatch, zero/oversized units, reversed/equal agreement parties or
invalid intervals reject the candidate atomically. Cancellation or a fact-bound
failure preserves the last valid state, queue, history and resident metadata.

### Approved compact capacity admission contract

The additive sparse `omitted` object uses the exact keys `reason` and `ranges`.
The only initial reason is `capacity`; `ranges` contains inclusive `[start,end]`
u16 ordinal pairs. Absence preserves previous bytes. Empty, reversed, adjacent,
overlapping, unordered, out-of-receipt ranges and explicit null are rejected.
Ranges participate in integrity and bind to the exact ordinary receipt digest;
they are disjoint from retained details. An omission cannot create an event.

Admission retains the maximal stable prefix of producer-supplied explanations
whose complete canonical envelope fits 2 MiB, including integrity and suffix
omission framing. Remaining candidate ordinals are coalesced into ranges; bare
ordinary events are not marked as requested explanations. Omitted ordinals occur
strictly after retained details. Invalid shapes still reject atomically, but
valid presentation details alone do not fault a valid ordinary boundary. The
separate mandatory intent-decision reason channel is unaffected.

The independent known delivery fixture uses 577 bytes per explained entry;
4,096 deliveries exceed the cap without admission. Individual omission records
also fail at the 65,536 ordinary-event maximum. Compact coalesced ranges fit:
the worst alternating candidate pattern has 32,768 pairs, under 459 KiB even
at maximum ordinal width. No ordinary receipt, event identity or cap changes.

Coverage must independently calculate at least one mixed details/omissions
fixture and preserve every old fixture. Tests must compare the ordinary receipt
against a bare collector, retain all 4,096 shipment events, exercise exact-size
boundaries and changing ordinal widths, prove deterministic admission under
different executor slice budgets, reject overlap/duplicates/wrong identities/
unknown reasons and rehashed fabricated imports during replay, and expose an
explicit unavailable explanation in Chronicle and Notebook. A capacity omission
cannot satisfy a Trade or supply-chain evidence predicate.

## Notebook and presentation acceptance boundary

An authenticated autonomous AgreementFormed with Trade parties is a negotiation
observation. An authenticated autonomous ShipmentDelivered with dispatch_kind
Trade, different dispatch owner and recipient, matching intended recipient and
positive accepted units is a delivery observation. One event does not establish
both stages or broad completion. Legacy unknown-kind delivery is honestly a
transfer observation and cannot prove negotiated trade. Merely finding an active
agreement or inventory increase in the current state is insufficient.

Supply-chain observations can reference authenticated actual production facts
and a delivered resource with exact world endpoints. A two-system observation
requires recorded known source/destination systems that differ. A foundry's
actual input/output facts can document ore consumption and alloy production on
the delivered-to world. Fungible inventories cannot establish that a particular
shipment's individual units caused that production; no such causal claim or
shipment-to-facility link is invented. Incomplete stages stay explicitly
incomplete. The existing Notebook supports exact references at separate named
bookmarks; replay must authenticate each against its own receipt and sidecar.

## Independent vectors and regression contract

Verify the original independent corpus and all existing additive corpora before
any vector edit. New Some fixtures are calculated independently in Python from
this declared key order, not emitted by Rust. For a fixed ordinary receipt at
tick 50/state digest 44 repeated 32 times, use phase-3 ShipmentDelivered ordinal0
and phase-7 AgreementFormed ordinal1, retaining ordinary phase order. Derive ordinary digest from the unchanged
receipt domain and payload; derive each event ID from its unchanged formula;
derive fact integrity from the existing event-facts domain and payload. Fixture
agreement: Trade, parties 11/22 repeated 16 times, interval [50,1250). Fixture
delivery: worlds 33/44, sender11, intended/actual recipient22, Ore, units10,
Trade, systems55/66 (all identities repeated 16 times). Include unknown optional
manifest/system values in a separate independently calculated fixture. Preserve
all old absent-field fixtures byte-for-byte.

Real Commerce must authenticate a matching bilateral Trade formation by tick100
and an actual cross-civilization delivery by tick200 with exact recorded endpoints,
resource and accepted quantity. Public genesis/command fixtures must distinguish
full-storage waiting, later real delivery, captured recipient, return-to-source,
legacy unknown manifest kind and positive zero-inference supply observations.
Mutate units, parties, interval, recipient, kind, ordinal, event ID, receipt digest
and integrity; invalid shapes and unrehashed corrupt pairs reject without
authority mutation. Integrity hashes are not signatures: a recomputed hash can
frame fabricated but structurally valid facts. Consumers must replay authority
and compare its producer facts to prove an actual event, rather than accepting
an imported sidecar solely because its hashes agree. Test
explicit noncanonical null for sparse fields, nested unknown preservation,
replay/native save/reload and cancellation. Native/wasm digest parity, whole-path
work budgeting and live platform journeys retain their separate proof boundaries.

## Linked Trade notebook derivation

This slice reuses named bookmarks and existing `trade_delivery` evidence
records. It adds no canonical field, stored completion flag, mission, reward or
authority command. A baseline bookmark is optional context; it is not required
for the accepted experiment. Completion derives from two retained observations.

### Exact proof conditions

A candidate selects a negotiation witness F and a delivery witness D. Each
has an existing validated Trade evidence record. The derived proof requires:

1. F and D resolve against the same validated archive and name the same branch.
   Actual replayed creator-revision prefixes are nondecreasing, and
   `F.tick < D.tick`. Same-tick events, sibling futures and a reversed creator
   prefix never join merely because their names or participants match.
2. F references an autonomous AgreementFormed with typed Trade facts, sorted
   distinct parties A/C, start equal to F.tick and valid exclusive end. Replay
   at F contains that exact agreement. Creator overrides alone do not qualify.
3. D references autonomous ShipmentDelivered facts: Trade classification,
   distinct worlds and actors, intended recipient equal to actual recipient,
   positive accepted units and unordered sender/recipient pair exactly A/C.
   Either direction is valid; a third party, matching name or guessed route is
   insufficient.
4. Replay at D contains the exact F agreement kind, parties, start and end,
   active at D.tick in `[start,end)`. Expired, removed or replaced agreements
   do not complete this pair. Present inventory is not a delivery witness.
5. Missing/omitted facts, unknown classification, invalid imports, cancelled
   replay or stale history attachment cannot produce a proof. Each independently
   valid record remains an observation when another stage is missing.

`ValidatedLivingNotebookV2.trade_completions()` returns read-only private-field
proofs. Each provides formation/delivery evidence record and bookmark ordinals,
exact views and event references, and authoritative agreement/delivery values.
Proofs never enter notebook bytes. The replay job retains compact typed witnesses
and actual revision depths, without extra full replay states. One candidate pair
comparison consumes one replay work unit. At most one stable first matching F
is retained per D reference, bounding proof storage by the existing 128 records
and 32 references per record. Removing support and revalidating recomputes proofs.

This proves an observed negotiation and a matching real delivery later while
that exact agreement was active. It does not prove that the agreement caused
cargo dispatch, that cargo departed after negotiation, or that particular cargo
units entered a production recipe. Delivery facts lack exact shipment identity
and departure witnesses. Genesis cargo dispatched before formation can arrive
later and still support these two observed stages. Stronger causal claims remain
unsupported and cannot be inferred from arrival time or route existence.

### UI integration contract

The notebook displays separately recorded agreement and delivery observations.
With F only it reads "Agreement observed; matching delivery pending"; with D
only, "Delivery observed; negotiation witness pending". A capacity omission
remains explicitly unavailable evidence. Optional baselines may be kept as notes
without blocking proof. After replay validation, linked proof reads "Agreement
and matching delivery observed", with its exact branch, F/D ticks, parties,
resource and accepted units, and links to both receipts and bookmarks.

Current authoritative local records remain observations until asynchronous
notebook replay returns the derived proof. Session may cache only copies of
these private-constructor proofs from validated metadata. Save/Continue and
portable import retain the existing evidence shapes; completion is recomputed
after installation. Removing support or changing retained history invalidates
the cache. Failed/cancelled verification preserves resident metadata and cannot
publish a partial derived completion. No claim of dispatch causation appears.

### Qualification boundary

Individual typed events, capacity admission and observation persistence have
host automated evidence. Linked core derivation is implemented in this slice,
with focused qualification reported separately after actual results. Session
cache, visible linked proof, save/destroy/reopen, portable transfer, native/browser
pointer and keyboard journeys, actual IndexedDB reopen and assistive speech are
separate checks. Their completion cannot be inferred from core compilation.

Regression coverage must include real Commerce without a mandatory baseline;
F-only and D-only notes; reversed direction; wrong-party, expired, removed or
replaced agreement, creator-only, same-tick, sibling-branch and reversed-prefix
negatives; legacy/omitted facts; forged references; supporting-record removal;
cancellation; and save/reopen plus portable replay producing the same proof.
