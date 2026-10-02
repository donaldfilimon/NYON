# Living V3 specification review

Date: 2026-10-01. Status: **Partial; implementation prerequisites remain open**.
Scope: architecture, society, ecology, diplomacy, experience and wire proposals.
This is an internal consistency review, not independent implementation acceptance.

## Resolved in this review

- Society originally described a colony Food inventory while the wire design
  placed Food on the world. The society contract now names the world inventory as
  the sole account for needs, recipes and freight. Carrying capacity is derived.
- Research preserves progress across target changes. The introductory society
  definition now consistently describes per-technology progress.
- The wire environment roster omitted maximum fertility required by renewal and
  creator validation. It now includes maximum fertility and uses base habitability
  consistently. Renewal and decay amounts remain fixed rules, not independent
  mutable rates with unspecified effects.
- Workforce recomputation now explicitly covers facility edits/completion,
  passenger transfer, technology edits and demographics. It assigns completed
  enabled staffed kinds without advancing recipe or needs clocks; newly completed
  facilities retain their completion-plus-period first operation.

## Open before expansion code

1. Freeze every inherited and new entity, catalog, command, event and history
   field schema. A record addition table cannot define a codec or reject malformed
   records. Include all enum variants, capacities, optional fields, sorted keys,
   defaults, removal dispositions and allocation marks.
2. Finish entity, fork and revision identity framing. Specify stable kind tags,
   actor/creator namespaces, sequence consumption, collision rejection and exact
   snapshot references. Derive vectors before writing codecs.
3. Define new economy candidate utility and tie order numerically. “Recovery
   outranks discretionary spending” does not rank two competing recovery choices.
   Include local Food/energy, workforce, research, treaty and military reservations.
4. Complete diplomatic request/commitment state. Demand requests are specified
   in rules but absent from the state roster. Specify exact transport visibility,
   request capacity, acceptance order, obligation status, renewal and peace records.
   A responder's objective must be represented explicitly to evaluate peace.
5. Specify history checkpoints, authenticated receipt retention and bounded
   reclamation. State the maximum work/bytes for comparison and export, and how
   reference anchors survive receipt eviction and reconstruct by replay.
6. Extend independent vectors to full valid documents and negative cases, stellar
   schedules, diplomacy, scarcity, conservation and replay. The existing proposed
   subset qualifies its own arithmetic and framing only.

V2 completion remains authorized and proceeds while these prerequisites are
settled. No V3 mechanic is implemented or accepted by this review.
