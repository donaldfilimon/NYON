# Standalone diplomacy cursor and Corridor publication review

Review boundary: `LivingDiplomacyJobV2`, its test-only rules reference wrapper,
and the authority's new ephemeral Corridor witness publication. Read-only source
review; no producer or executor edits were made during this review.

## Verdict

No blocking correctness finding in the reviewed standalone diplomacy slice or
Corridor publication. The standalone cursor is qualified against the existing
reference phase; this does not certify resident phase integration, a complete
bounded step, device performance or the complete Living game.

## Diplomacy evidence

The cursor performs relation decay before constructing its normalized decision
snapshot hash. The same immutable post-decay state supplies active actors,
relations, owners, lane connectivity, observations and assembled fleet strength.
All bilateral agreement and unilateral war proposals are collected before
ApplyAgreements/ApplyWars mutate their candidate maps. This preserves the original
shared proposal snapshot rather than letting an earlier declaration change a later
actor's target. Duplicate declarations are resolved in stable actor order.

Source comparison checked active/exclusive intervals, bilateral thresholds,
reachable colony pairs, fresh observed targets, immediate adjacency, actual docked
fleet ownership/energy/order/build reservations, maximum single-fleet assembled
escort strength, observed operational defenses, stable target ordering, agreement
removal on declaration and relation penalties. Decision keys preserve actor/action/
target/canonical-action ordering before assigning per-actor ordinals. The rules
wrapper only delegates to the original phase under cfg(test).

Hash and search work consume the shared visit budget. Search distance/frontier
entries and retained graph/index records retire incrementally before Ready;
finish rejects cancellation, sticky faults and non-ready states. The tests check
budget-one and mixed schedules, equal total visits, major-stage cancellation and
deadline arithmetic failure. Scalar stage transitions do not charge fictitious
record visits.

Independent execution: `cargo test -p nyon-workshop-core --lib diplomacy_tests`
completed with **exit 0, six passed, zero failed**. Log:
`/tmp/nyon-independent-diplomacy-review-tests.log`. Coverage includes Trade/
Nonaggression/war/shared snapshot parity, decay/truce/existing/stale axes, zero/
one/many actors, saturated path cost/disconnection, cancellation/sticky faults,
and fleet assembly/observed defense permissions. State, ordinary payloads,
explanations and decisions are compared against the reference across budgets.

## Corridor publication evidence

The pending vector is extracted only from the local candidate after state
validation and ordinary receipt sealing; each witness tick must equal the sealed
receipt tick, and its digest is bound to that receipt. Decision sealing remains
fallible afterward, so any later error still discards the local candidate rather
than installing partially bound authority state. The prepared boundary carries
the vector alongside the exact state/receipt metadata.

Publication passes the existing exact capture fence. Successful commit moves the
old vector into the retired boundary and installs the new vector together with
the new receipt/state. Failed or stale preparation cannot replace resident
evidence. Retaining the previous last-valid receipt and its matching witnesses
after a latched step fault is consistent with the authority boundary contract.

Paused submission and paused replay clear Corridor evidence when installing their
creator receipt. Fork/replay-fork clear it; running replay installs regenerated
receipt-bound witnesses. Constructors initialize an empty vector. No new canonical
receipt fields, ordinary event identity inputs or publicly constructible prepared
authority bypass were introduced.

This publication review is source evidence. Root-owned actual public fixture,
atomic-failure/stale-capture, replay and reload tests remain the runtime evidence
boundary; standalone producer tests do not establish the full Corridor workflow.

## Subsequent exact-fault and staged-integration review

The seventh standalone regression changes only invalid normalized snapshot fault
classification to the reference's exact Decision(Snapshot) error. Its predicate
rejects zero tick and any non-successor of the source state tick. Due decay still
runs first, preserving the counted tick-zero path. Independent narrow execution
passed **one test, exit 0** in `/tmp/nyon-independent-diplomacy-delta.log`, including
zero, unchanged and skipped tick cases and sticky subsequent fault behavior.

The staged phase-seven integration source review also found no blocker. The
completed marker distinguishes observations from diplomacy. Due boundaries at
the existing 100-tick cadence hand the exact post-observation candidate/events to
the standalone job; non-due boundaries mark completion without invoking it. The
synchronous remainder skips already-completed diplomacy while retaining later
intent/dispatch/commit phases. Cancellation, sticky fault and stale capture checks
continue to guard publication. Independent execution passed **18 staged tests, exit 0**, in
`/tmp/nyon-independent-staged-phase7.log`. This establishes the focused regression
boundary, without whole-step performance qualification.

Chronicle restoration source review found no blocker: the latest boundary is
seeded only when tick/state digest match, decision metadata must match that same
receipt/state, and successful history restoration clears previous presentation
caches before projecting the authenticated restored boundary. Older discarded
Chronicle rows are not invented from the archive.
