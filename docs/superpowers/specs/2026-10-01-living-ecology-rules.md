# Living V3 ecology rules

Date: 2026-10-01. Status: **Proposed numerical contract for review**.
Parent: `2026-10-01-living-expansion-architecture.md`.
These rules do not amend V2 hazards, resources, recipes or canonical state.

## Boundary order shared with society and diplomacy

V3 uses the following deterministic order:

1. Apply queued creator revisions in their accepted order.
2. Expire agreements/wars/hazards; advance due ecology and warnings.
3. Resolve fleet arrivals and freight/passenger delivery/return/capture.
4. Resolve combat, retreat, occupation and settlement.
5. Complete jobs; establish/recompute due Food/energy holds, then execute guarded
   local production, repair, research and due military supply.
6. Pay due population needs, resolve demographics and update workforce.
7. Refresh limited civilization observations; on multiples50 capture one immutable
   actor-limited planning snapshot per actor from this common authority input.
8. On multiples100 update diplomacy and negotiate using those phase7 captures,
   protecting global category0 phase8/9 material reservations before ordinary
   pending-offer consideration.
9. On multiples50 generate proposals and reserve using the SAME phase7 captures
   and central ledger. Negotiation/earlier spending cannot recapture/rescore.
10. Dispatch eligible standing orders and freight.
11. Validate, seal receipts and atomically publish the boundary; release transient
    planning/declaration holds. All guarded resource debits remain actual facts.

Local production uses the workforce committed by the prior assignment update;
the phase-six update applies to subsequent production. Creator population or
specialization edits, ownership changes and arrivals/settlement recompute affected
workforce immediately using the same priority rule, without advancing a needs
clock. Facility completion, enable/disable, creation/removal, passenger boarding
or unloading and technology grant/revocation also recompute the affected colony's
assignments immediately. These recomputations never consume resources or move
needs/operation clocks. Assign only enabled completed staffed facility kinds;
passive habitation/defense and reserved jobs consume no worker units. A newly
completed facility can receive workers immediately but its first operation
remains completion plus its recipe period. After each due demographic update,
recompute against resulting population. No actor sees another actor's later
reservation in its proposal snapshot.

## World environment and conservation

Each world stores base habitability, environmental stress, renewable fertility,
maximum fertility and a renewal/decay next-due clock. Values use integer basis
points, 0..=10000. maximum fertility is immutable catalog/genesis data unless an
explicit creator edit changes it with a complete preview. Current fertility may
not exceed maximum. There is no floating-point environment state.

At the world's ecology boundary, every 100 ticks: renew fertility by min(50,
maximum-current) and reduce stress by min(10, stress). Advance next_due by exactly
100, even at either bound. There is no catch-up renewal after pause or import.
Finite mineral deposits do not replenish through this rule.

Effective habitability = max(0, base_habitability - floor(stress/2) - summed active
habitability impacts capped at 10000). Effective carrying capacity =
floor(base society carrying capacity * effective habitability / 10000).
Resilient habitation halves the combined habitability penalty, rounding down,
before subtraction. It never raises habitability above its base.

Farm Food output = floor(base recipe output * effective habitability / 10000),
then limited by renewable fertility. Each actual Food unit consumes one fertility
unit. The schema Farm evaluated prefix checks staffing before environment; a
staffed q0 consumes no energy and reports only Environmental limit/Food0. Positive
q physical blockers precede needs holds; failed input/room/worker consumes neither
energy nor fertility. Farms run in stable facility-ID order and cannot spend fertility twice.
Agronomy changes only the base output, before environmental scaling.

Actual successful extractor operation adds 1 stress; successful foundry operation
adds 2 stress. Blocked attempts add zero. Apply stress immediately, capped at
10000; it can affect later ID-ordered farms on the same production boundary.
This order is normative and must be shown in operation/forecast explanations.
Other existing recipes add zero unless a new catalog definition explicitly names
a stress contribution in a later reviewed rules version.

The world inspector distinguishes mineral remaining, renewable fertility,
renewal rate, stress/decay, effective habitability, capacity and active events.
Do not label a finite deposit as renewable or present nominal recipe output as
actual achievable output.

## Stellar activity and disasters

Initial automatic event: Solar flare. Each star stores its next activity boundary,
initialized to genesis +1000. At a due boundary E, independently derive
SHA256(domain literal `NYON_LIVING_V3_STELLAR_ACTIVITY`, seed32, star_id16,
E little-endian u64); interpret bytes 0..2 as little-endian u16. A value less than
6554 selects a flare. Advance the clock by1000 regardless of selection.

A selected flare receives an identity from the reviewed V3 autonomous entity
formula, using that star, E and its stable activity ordinal. It targets the star's
system, warns at E, starts at E+100 and ends at E+200, end-exclusive. Active flare
impact is 2000 habitability basis points on that system's worlds and a 1/2
freight batch multiplier on lanes with that system as an endpoint, rounding down.
The existing ion-storm lane event is a separate type; concurrent modifiers apply
in event-ID order to the current integer batch. No hidden ambient RNG or wall
clock influences activity.

Hazards and stellar_events each have at most128 pending/active records, separately.
Automatic star creation checks only stellar_events occupancy128;128 hazards do not
consume a stellar slot. At stellar capacity, reject automatic event creation with an explicit capacity-omission event while advancing
the activity clock; do not fail unrelated world updates or overwrite an existing
event. The omission fact names star/E/proposed type, so replay reproduces it.
Expired records remain in history receipts and leave active state at expiry.

Creator may schedule/edit/cancel a flare or supported lane hazard, including
exact target and warning/start/end boundaries. Validation enforces warning <=
start < end and rejects missing targets, overflow and duplicate identities
atomically. An event already active can be cancelled only as an explicit creator
revision with its boundary and changed effects recorded.

## Autonomous adaptation

Actor32 selects exact owned deposits, public World id/system and own claims/occupation records, while remote dynamic inputs remain observed. Scoring §Conditional private B/F operator holds unsupported claim/combat/storage effects and never reads seed for ranks/flare foresight. Complete-schema §Closed scale domains permits Proposed creator nonamplifying0..denominator/positive denominator, retaining unreduced spelling; automatic flares and fixed Catalog stay literal1/2. Exact zero-freight transition scales nominal batch after blockade and before stock cap; zero issues no Shipment/debit/delivery credit and never bypasses earlier source floor. Optional bootstrap is schema B0–B7, not a global richer-Genesis ban.


Actors know environmental data only for currently visible worlds/lanes or their
timestamped prior observations. Their snapshot's known_hazards and
known_stellar_events carry exact observed lane/system/star target, warning/start/
end, freight fraction and original observer/tick (schema correction I1). Warnings
become observable only with warning<=capture tick<end and authorized direct/shared
visibility; knowing distant topology alone grants no live schedule. Captured stale
facts survive unseen edits until refreshed and remain labelled counterfactual.
Apply captured freight factors in event-ID order; never hard-code1/2 for a
creator-edited fraction or read hidden root event placement. No star RNG clock or
seed is revealed by these records. Their scheduled forecasts apply known warnings,
habitat penalties, needs and actual known immutable manifests; unmanifested
contracted promises never count as arrivals. They cannot read unseen changes or
future seeded disasters before warning. Scoring §§1–8 fixes their exact200-boundary
counterfactual, visibility, checked utility and complete admission/tie ordering.

Adaptation candidates use existing economic/fleet reservation lanes: add staffing
and Food production, fund resilience research, increase habitation, adjust supply
or move/settle through an eligible ark. Imminent paid-needs shortage outranks
discretionary construction. Qualifying category0 reduces an owned needs result
within100 ticks (or the explicitly declared public-request opportunity); helping
only later needs is recovery category1. Adaptation shares the same physical/
industrial/hull/fleet reservations as diplomacy and economy. Due production obeys
the society guard before phase6; actual stress/fertility effects still use the
unchanged ID/atomic recipe order. Research is a gradual investment, not an emergency
instant unlock. Passengers obey society conservation; no free evacuation population
or globally teleported supply is introduced.

## Creator/history and acceptance

Typed environment edits expose base values, actual fertility, stress and clocks.
Previews show affected worlds/routes/needs/workforce and ownership-independent
impacts. Creator overrides are free but cannot bypass ranges, identity or capacity.
Hazard layer and text warnings expose the same facts as semantics and Chronicle.

Pin boundary E/E+99/E+100/E+199/E+200, multiple stars, equal-ID ordering,
concurrent flare/ion storm, empty galaxies, zero/full fertility, depleted minerals,
stress caps, Food/storage atomicity, actor-hidden changes, event capacity omission,
creator cancellation/removal and undo/replay. Derive activity/hash fixtures outside
the Rust implementation. Save immediately before a warning/start/end and compare
next results to uninterrupted authority; test native/wasm and polling parity.


Independent correction fixtures must compare announced versus hidden/cancelled/
expired/creator-edited records, conflicting provenance, exact system/lane freight
targets, multiple factor ID order and equal permitted input with different hidden
root changes. Eligible trace omission/clock/cost handling follows schema I4.
These written contracts do not imply those future vectors already exist.


## Integrated round2 environmental scope correction (Proposed)

Complete-schema §Integrated correction I1 is the normative environmental knowledge
registry. Complete timestamped system/lane assertions include empty negative
coverage; choose newest scope across all permitted source worlds before flattening
positive event IDs. Select newest per-event child over retained winning positives
INCLUDING expired children, then filter the chosen child by expiry, then check128-
fact capture limits; never resurrect a losing older copy on expiry. Scope history
is unchanged by filtering; source refresh/eviction is separate knowledge loss. Equal ticks use lowest
original observer then canonical scope bytes, without own-source priority. Missing,
partial or unrelated scope data grants no absence evidence. Shared/request copies
preserve scope bytes and timestamps. Forecasts use only winning permitted scopes;
hidden cancellations retain stale facts until authorized supersession or expiry.
Fresh schedule observation requires an incident directly visible world; own fleet
traversal alone does not refresh schedules. This explicitly Proposed visibility
refinement replaces round1's fleet-only read and changes observation/capture wire
bytes, not balance tables, combat sight or movement. Empty winners enter actor
snapshot hashes; complete observation carriers and scopes obey schema bounds.
Old-baseline corpora remain historical, and full integrated gap6 remains Partial.
