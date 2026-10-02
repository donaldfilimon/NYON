# Living observed alloy supply completion

Status: implementation detail of the accepted optional two-system alloy supply
experiment. This derives observations from existing replayed receipts and typed
facts. It adds no gameplay, canonical field, recipe, command, reward or causal
cargo identity. Focused and platform qualification remain separate.

The accepted notebook asks players to establish a two-system alloy supply chain;
the current briefing calls for actual alloy production and actual inter-system
transfer endpoints and amounts. Two observed resource topologies satisfy those
stages without adding a mandatory extraction witness or baseline:

- Alloy production on a world followed by an actual Alloy delivery from that
  world to a world in another system.
- Actual Ore delivery between systems followed by Ore-consuming Alloy production
  on the delivered-to world. Arrival precedes production within a boundary, so
  both witnesses can be at the same tick when their ordinary ordinals prove order.

## Evidence and derived interface

Both observations use the existing `supply_chain` experiment records. A successful
production reference must be autonomous FacilityProduced, with its exact receipt
and sidecar authenticated against authority replay. Facts record positive Ore
input and Alloy output, and an existing facility entity. Replay resolves that
facility's actual world and system and checks the production actor equals the
world owner at that boundary. Merely observing a foundry, stockpile, blocked
recipe or future construction does not establish actual production.

A delivery reference must be autonomous ShipmentDelivered with typed positive
accepted units, matching intended/actual recipient, distinct world endpoints,
Ore or Alloy resource, and known distinct source/destination systems. Waiting,
capture, return, Energy freight, missing explanations and capacity omissions
cannot certify this observation. Supply does not claim negotiated Trade, so an
Internal shipment or an unknown immutable dispatch classification is not given
invented diplomatic meaning; its actual accepted resource transfer can still be
observed when all required physical facts are known.

Derived joins require the same exact validated retained branch and appropriately
ordered actual creator-revision prefixes. Alloy-produced-before-delivery compares
`(tick, event ordinal)` and requires the production actor/world/system to match
shipment sender/source/source system. Ore-delivered-before-production requires
the production actor/world/system to match actual recipient/destination/
destination system. A missing production owner cannot complete a join. Relocated
worlds, changed owners, reversed prefixes, siblings and unrelated endpoints
cannot be joined by display name or a later inventory measurement.

`ValidatedLivingNotebookV2.supply_completions()` exposes private-constructor,
read-only `LivingNotebookSupplyCompletionV2` values. Each has production/delivery
record and bookmark ordinals, exact views and event references, production
actor/facility/world/system, observed Ore input and Alloy output, the actual
accepted delivery facts, and a `LivingNotebookSupplyTopologyV2` classification:
`AlloyDelivered` or `OreDeliveredToAlloyProducer`. There is no persisted completion
bit. No proof enters canonical Notebook or archive bytes.

The replay job retains compact typed witnesses, not extra full replay states.
One candidate comparison consumes one replay work unit. At most one stable first
matching production is retained per delivery reference. Storage stays bounded
by the existing 128 evidence records with at most 32 references each. Cancellation
or validation failure clears unpublished derived witnesses. Removing either
supporting record and validating again removes its derived completion.

`supports_living_supply_observation_v2(authority, receipt, event)` shares the exact
per-observation admission predicate with Session. It requires the authority's
current receipt, exact event membership and authenticated producer facts. It
returns a boolean for one observation, not a complete joined proof. Local records
remain observations until validated Notebook replay establishes a derived join.

## Meaning and UI boundary

The resulting label is "Alloy production and inter-system delivery observed".
It displays the actual facility/world/system, input/output quantities, transferred
resource/units, direction, branch and both exact receipt/bookmark links. Before
both stages validate, the notebook shows the actual observation and the missing
stage, with no reward or failure timer.

This does not prove that delivered Ore units were the units consumed by the
foundry, or that Alloy delivered later came from the named production event.
Inventories are fungible; delivery facts do not contain a shipment identity and
historical departure witness. Genesis cargo may have departed earlier. No
shipment-to-facility causal link or resource conservation proof is inferred from
coincident quantities or present inventories. Unknown dispatch causation stays
explicit even when the two actual observed stages are linked.

Session may cache only copies of derived proofs from validated metadata attached
to the exact resident retained history. Save/Continue and import recompute joins;
failed/cancelled validation preserves resident notes and cannot publish partial
success. Current-build live native/browser, IndexedDB persistence, pointer/
keyboard and assistive speech journeys are independent acceptance evidence.

## Focused regression contract

Cover both real topologies, including same-boundary Ore arrival then Alloy
production; actual actor/world/system/resource matching; single observations
remaining incomplete; same-system and Energy rejection; wrong endpoints and
chronology; reversed creator prefix and sibling branches; unknown systems and
actor mismatch; stale/forged receipts through the shared public helper; canonical
byte preservation; support removal; cancellation; and coherent save/reopen/import
through the visible UI. Host tests alone do not certify the live platform journey.

Current host qualification: five focused core integration tests pass, including
both topologies, exact matching/ordering, sibling and reversed-prefix negatives,
same-system/Energy rejection and stale/forged public helper inputs. One controlled
predicate unit test rejects unknown systems and actor/world mismatches. One
public Runtime/native-store journey records both actual stages, runs the visible
asynchronous verification action, inspects both bookmark links, saves and destroys
the client, then reopens exact Continue with the replay-derived detail retained.
Notebook bytes remain unchanged. These results do not establish real IndexedDB,
live pointer/keyboard execution, speech access or named-device performance.
