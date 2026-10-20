## Design Document Review: Rich-city plan workload fix1

### Summary

**PASS for the scoped planning correction.** The unreachable simultaneous-capacity requirement is removed, and its direct witness, transcript, ownership and performance consumers now use independently accepted legal workloads while retaining separate synthetic envelope stress. No Important/major or Critical finding remains in this correction; product readiness still requires the actual Task 0 and Task 1 exit artifacts.

### Scope and exact revisions

This review covers only Issue 1 from the [original plan review](2026-10-11-rich-city-plan-review.md) and direct regressions in its consumers. It does not reopen the complete game design or approve moving external owner documents. The original review and all source, specifications, plans, ledgers and instructions were preserved.

| Artifact | Reviewed SHA256 |
| --- | --- |
| [Rich-city implementation plan](../superpowers/plans/2026-10-11-nyon-zig-rich-city.md) | `d4306c80d70576f5bdd5fb44dcb77728b82792017f117d8aa6d8d282aa8ab832` |
| [R: rich-city design](../superpowers/specs/2026-10-11-nyon-zig-rich-city-design.md) | `c9443864b336bf415b6dc38537118c14438343f9d4ebb0a820025adb5b1a4709` |
| Original plan review | `df12dbbb3893cd7278f3726c86206361e0e53127c4225a8abc92d620583fca63` |

The underlying four-neighbor topology and current/proposed architecture evidence remains the actual source inspection recorded in the original review, particularly `zig/src/surface.zig:4-5,47-86`. This documentation correction supplies no new executable behavior requiring a source change or a build. R's unchanged placement, claim and freight rules remain the basis of the impossibility argument.

### Resolved finding: Reachable qualification workloads versus conservative capacity envelopes

**Original issue:** Task 17 required a reachable `4096 cities / 16384 buildings / 4096 cargo / 36864 claims` state. Full claims at those city/building counts forces every building to be a farm and consumes every hall attachment slot; no legal hall/road freight path remains.

**Disposition: resolved in the reviewed plan revision.** Task 1 lines234-235 explicitly disallows that tuple and requires independently derived fixtures in `FX/custody/quota-workloads.json`, hash-linked from `FX/manifest.json`. It separates legal per-quota extrema and next-admission attempts, reachable joint workloads, and synthetic allocator/codec stress. It does not substitute an invented alternative maximum.

The revised acceptance records the actual first rejection. A configured boundary that an earlier placement/resource rule prevents reaching is tested separately as a synthetic predicate/storage boundary; it cannot be reported as a public gameplay quota rejection. Every legal fixture must pin manifest, qualified cells, owners, footprints, completed routes, stocks/reservations, departures/positions, public command sequence, phase counts and oracle hashes. Independent review precedes consumers, and the Task 1 stop at line255 requires these witnesses before Task 2.

### Direct consumer checks

| Consumer | Scoped result |
| --- | --- |
| Task 1, lines229-255 | Owns the independently written workload oracle and `verifyQuotaWorkloads()` alongside complete-story and layout witnesses. Missing schedules/hashes remain blockers, with no claimed successful construction. |
| Task 15, line598 | Retains maximum-size syntactic emitters and separately consumes the exact Task 1 reachable workloads. Synthetic envelope, admission peak and committed state are labeled distinctly. |
| Task 17, lines638-645 | Consumes the pinned fixture file; verifies actual counts, routes, phase hashes and the first rejecting guard. The 83,216-row/95,504-page envelope remains conservative synthetic storage stress, forbidden from qualifying an accepted gameplay tick. Public cancellation/retirement uses reachable workloads; synthetic ownership stress still requires valid storage ownership. |
| Task 17, line641; Task 18, line668 | Admission/pre-freight peaks are separated from committed cargo after arrival retirement. Destination reservations must match surviving cargo, and timed samples include any scheduled replenishment commands. This closes a direct risk of reporting peak admission counts as sustained loaded state. |
| Task 18, lines664-669 | Requires 10,000 full loaded ticks **for each** accepted legal loaded-workload schedule, exact fixture hashes/counts, applicable extrema and interacting city/road/freight/training/tactical work. Arbitrary sparse fixtures and synthetic all-maxima storage cannot satisfy the timing gate. Maximum storage/encoding stress remains mandatory separately. |
| Global constraints, lines26-30; Task 18, lines667-669 | City/building/claim/shipment/training/roster quotas are unchanged. Full-tick p95≤5ms, separate 256-unit poll p95≤2ms/max≤8ms, zero poll allocations, 100,000 retained ticks and maximum-byte stress remain required. |

### Approval boundary

This PASS closes the original report's one workload finding for the exact revised plan. It is conditional planning acceptance, not implementation authorization or evidence that a maximum workload has already been constructed.

Task 0 lines215-223 still requires exact composed R/E/T/full-scale/provider/session/hostility contracts, registered profiles, concrete ownership/lifetime signatures and independent acceptance. Task 1 still requires the **actual** independently reviewed complete public journey, exact per-hour/final hashes, legal workload fixtures and compiled layout/ownership witness before product work. The correction strengthens those prerequisites without claiming they have passed. Later external revisions need their own exact-pin integration review.

### Strengths

- Preserves configured capacities while reporting mathematical reachability honestly.
- Gives the independent witness a concrete file, schema responsibilities, hashes and downstream consumers.
- Distinguishes candidate peaks, committed state and sustained timed workloads, including freight arrival/replenishment effects.
- Retains full performance, custody, ownership and durability obligations without treating synthetic stress as gameplay evidence.

### Verification record

**Current:** scoped text/contract review and direct-consumer consistency checks. Documentation validation checked structure, whitespace, local links, exact plan/R pins and preservation of the raw review; exit **0**. Separate scoped `git diff --check` exited **0**. This new untracked review was validated directly rather than relying on tracked-diff coverage.

**Not run:** compiler invocations, builds, product tests, repository executable gates, allocation/layout measurements, performance workloads or provider/session/native/browser operations. Only this new review file was created. No implementation, complete-story hash, reachable maximum, storage-fit or runtime success is asserted.
