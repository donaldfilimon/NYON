# Proposed environmental boundary fragments

Independent written-formula diagnostics for complete-schema I1 scope selection,
event identity selection before expiry, and positive-roster capacity. The script
imports no authority code and pins three source hashes.

Run `python3 docs/superpowers/specs/fixtures/living-v3-environment-boundaries-proposed/derive.py verify`.

There are 18 cases: nine each for stellar and hazard facts. They cover 128/129
positive IDs, supersession and expiry before the capacity check, original-observer
and canonical-byte ties, and a newer expired identity without fallback. Every case
also checks reversed source order and duplicate source assertions.

The 129 case faults with `record_capacity`; it never truncates or publishes a
partial roster. Same-tick, same-observer empty scope wins the literal canonical
byte tie against a positive scope because `]` precedes `{` at the first differing
byte. These rules follow the written tuple rather than a priority for own sources.

All sources are synthetic scope/child fragments with symbolic topology and hazard
definitions. They are **not admitted full State, Observation, history or actor
capture documents**. Output SHA256 values hash diagnostic fragment bytes only;
they are not normative actor, trace, Receipt or State hashes. The retained raw
scopes include children that have expired since observation.

Author generate/verify consistency is separate from independent review, full
admission, budget/cancellation proof and runtime qualification. Full V3 remains
Partial and the rules remain Proposed.
