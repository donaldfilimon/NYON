# Living V2 bounded intent continuation

Date: 2026-10-01. Status: **Proposed implementation sequence**.
Scope: complete the existing intent semantics without changing numerical rules,
wire shapes, identities or actor disclosure. Depends on scheduled forecast jobs.

## 2026-10-02 intent-input checkpoint

Slices 1 and 2 now exist as private components. Owned scheduled forecasts retain
catalog/source ownership; `LivingIntentInputsJobV2` composes bounded snapshot
copying, normalized decision hashing, one active actor forecast at a time,
original-stock/colony ownership indexes, receiver demands and index retirement.
Every child shares the remaining 1..=1024 actual-record budget. The original
snapshot is immutable; ordered demands retain the reference stock <20 rule.

Ready-only borrowed finish moves output once while premature/error finish retains
partial owners. The normalized hash owner moves into output for future bounded
retirement. Cancellation/fault preserves active children and accumulated outputs;
eventual job/output reclamation remains the composing caller's responsibility.
No resident switch, proposal/reservation integration or synchronous-marker removal
is part of this checkpoint. Slice 3 is the next implementation dependency.

Four focused regressions, final format and strict core Clippy passed. Independent
specification and code-quality review approved the frozen task diff with no
findings. Current full qualification is recorded separately in
`LIVING-INTENT-INPUTS-EVIDENCE-2026-10-02.md`; component parity alone establishes
neither bounded resident execution nor latency/memory acceptance.

## 2026-10-02 economic checkpoint

Slice 3 exists as a private owned cursor. Nested economic scans and exact path
search share the remaining poll budget. Both stable orders, adjacent full-action
deduplication, checked explanation ordinals and reference explanations are
preserved. Ready indexes retire incrementally; cancellation/fault and output
owners await the composing executor's bounded reclamation.

Independent path and economics reviews approved specification and quality with
no findings. Full qualification passed 1,384 workspace tests across 101 suites
and all repository native/fuzz/paired-web/wasm/vector gates. Evidence and frozen
identity are in `LIVING-ECONOMIC-EVIDENCE-2026-10-02.md`. Fleet slice 4 is underway;
reservation and resident integration remain unimplemented by these components.

## 2026-10-02 fleet checkpoint

Slice 4 exists as a private owned component. Immutable four-alternative selection,
counted state copying, replacement-aware assembly/rekeying, exact decisions and
nested delta descriptors preserve the reference. Fifteen focused regressions and
independent specification/quality review passed. Full qualification passed1,399
workspace tests across101 suites and all native/fuzz/paired-web/wasm/vector gates;
see `LIVING-FLEET-EVIDENCE-2026-10-02.md`. Retained output/no-decision owners await
bounded composing retirement. Validation is next, then reservation and complete
intent composition; resident execution remains synchronous.

## 2026-10-02 validation checkpoint

Incremental validation exists with exact oracle error precedence, accepted
omissions, effective-tick observation checks, counted nested traversal and
explicit retained-index retirement/owner handoff. Independent spec/quality
review and full combined-checkout qualification passed; evidence is in
`LIVING-VALIDATION-EVIDENCE-2026-10-02.md` (1,415 tests/101 suites).

General state retirement is next before reservation. A separate authorized task
has installed transitional resident polling with a synchronous completion tail;
whole bounded execution and responsiveness remain Partial. This supersedes
older resident-unchanged status, while retaining the explicit completion markers.

## Ownership and reference

`living/rules.rs` owns the proposal cursor and retained test reference;
`living/step_job.rs` owns reusable phase/catalog/search primitives;
`living/simulation.rs` composes the job before dispatch. Session integration
follows a complete boundary executor; no resident fallback is accepted.

Keep the existing `generate_and_reserve_intents_with_disclosure` algorithm as a
test reference while porting. Its snapshot digest, actor order, economic-before-
fleet category order, proposal ordinals, split identities, exact decision facts,
reservation fallback and validation behavior are normative.

## Sequential implementation slices

1. Give scheduled forecasts the existing `LivingPhaseCatalogV2` borrowed/owned
   holder and an owned constructor. Owned production/dispatch child constructors
   receive cloned catalog Arcs; avoid self-references or lifetime extension.
   Compare complete owned/borrowed forecasts and cancellation at all child phases.
2. Capture one immutable post-observation state owner, hash its decision snapshot
   incrementally, and schedule one active actor forecast at a time. Copy the
   snapshot through the state-copy job. Build receiver demand requests by visiting
   each deficit and indexed owned stock fact. Keep source knowledge immutable.
3. Port economic enumeration into nested candidate cursors. Explicitly budget
   colony/facility/recipe/resource scans, source/destination pairs, route legality,
   shortest paths and full-slot replacement. Accumulate candidates in a stable
   ordered structure so no final full sort hides inside one visit. Preserve exact
   candidate score, action bytes and explanation ordinal from the reference.
4. Port settlement/explore/attack/guard proposals using owned candidate copies.
   Budget nested hull selection, world/observation scans, paths, split assembly,
   proposal-delta extraction and created-fleet identity adjustment. Retain rejected
   alternatives needed for reservation fallback; retire unused candidate owners
   through a bounded queue rather than dropping a complete state inside poll.
5. Reserve one proposal at a time into a private copy of the current reserved
   candidate. Budget sorted deltas for worlds/jobs/facilities/fleets/routes and
   their nested records. Preserve all-or-nothing reservation and actor/category
   wins. Incremental candidate validation must exist before this slice can claim
   bounded completion; do not call `state.validate` as one work unit.
6. Publish no state from these jobs. Return the complete candidate and ordered
   decisions/events into the parent executor, then poll dispatch with its exact
   remaining budget. Remove the due-intent synchronous marker only when every
   nested stage is bounded. Keep `step()` on the same completed executor later.

## Required regressions

Compare full candidate, all three receipt surfaces, pending queue, allocator marks
and history at budgets 1/64/1024 against the reference, including a 50th boundary.
Cover two actors competing for stock/slots/hulls; full-capacity replacement;
ownership loss; hidden and stale observations; repair versus industry funding;
trade demand provenance; guard versus attack fallback; rejected proposal identity
consumption; fault/cancel/stale queue-tail behavior; and deterministic arithmetic
failure. Assert actual nested visit totals rather than whole-phase counters.

Focused parity does not qualify resident input response or memory. Full native,
fuzz, paired browser, wasm, vectors and artifact gates follow each completed
implementation slice; live platform and endurance measurements follow integration.
