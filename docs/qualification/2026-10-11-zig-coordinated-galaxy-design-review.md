## Design Document Review: NYON Zig coordinated multi-planet transactional authority

### Summary

**Needs revision before written-spec approval and implementation planning.** The selected separate-authority architecture is coherent, including all-world phase barriers, hidden installation and a single committed-descriptor publication point. Three Important contract gaps and one Minor semantic ambiguity remain; none requires adding the explicitly deferred full-game mechanics.

Reviewed frozen Proposed specification `docs/superpowers/specs/2026-10-11-nyon-zig-coordinated-galaxy-design.md`, SHA-256 `97b9479092faa40fa80b55563d492521152505df703ad4d974210646d4d8f18f`. Design-direction approval does not approve this written specification. Ownership of this review is limited to this file; no specification or product source was changed.

### Issue 1: Fault shutdown cannot satisfy ordinary destruction preconditions

- **Severity**: major
- **Priority**: Important, blocking
- **Section**: Architecture, ownership and interfaces; Abort, cancellation and reclamation
- **Description**: Lines 198–199 require “`deinit` requires no jobs and no leases”; lines 318–319 retain ownership and Busy until every cleanup cursor drains. Lines 324–326 then promise that after `CleanupFault` the caller can “destroy the whole allocation after fault shutdown,” without specifying how a faulted, undrainable cleanup lane becomes destroyable. A reference-count or directory defect can prevent ordinary retirement from ever completing. The current island has no applicable escape hatch: `zig/src/planet_economy/job.zig` `Authority.deinit` asserts `lane == null`, while `state.zig` `Candidate.advanceDiscard` propagates release failures. The proposed behavior therefore needs its own explicit shutdown transition rather than inheriting normal deinit.
- **Suggestion**: Specify a faulted terminal state and an explicit destruction operation/precondition: mutation permanently closed, no live leases, no further cleanup traversal required, each independently owned reservation freed exactly once using allocation ownership rather than damaged page-ref state. Specify whether view acquisition remains allowed after the fault, how it is closed for shutdown, and what poll/cancel return for the faulted JobId. Include a fault-injection witness that cannot drain normal retirement, preserves published queries until lease release, and then successfully destroys all allocations without claiming idle success.
- **Status**: open

### Issue 2: Global history capacity is canonical but absent from creation configuration

- **Severity**: major
- **Priority**: Important, blocking
- **Section**: Limits, identities and memory; Canonical history and replay
- **Description**: The admission table allows global history capacity 1..4,096. Lines 97–99 enumerate creation fields but provide only per-planet history capacities; the global genesis tag explicitly serializes `global_history_capacity:u16`. No rule supplies this value, its default or its selection API. Two implementations can choose different genesis bytes or admission behavior while following the listed creation config. This affects real behavior: capacity 1 admits genesis but no complete subsequent tick, and a one-planet empty tick needs three global records in addition to genesis. The fixed reservation size does not determine the admitted logical capacity. Existing island config/source only provides local capacities and cannot answer this new coordinator choice.
- **Suggestion**: Add global history capacity to the normative creation config, specify its default and admitted range, and define genesis occupancy and whole-tick capacity checking using that limit. Bind replay target construction to the exact serialized capacity. Require independent lower-bound/exact-fit/one-short global-history fixtures alongside the already required maximum and local exhaustion tests.
- **Status**: open

### Issue 3: Genesis policy is not defined despite canonical policy comparison

- **Severity**: major
- **Priority**: Important, blocking
- **Section**: Architecture, ownership and interfaces; Canonical history and replay
- **Description**: Lines 184–186 say the committed descriptor contains a “trusted policy,” but creation config and global/local genesis omit any initial policy, and no initial descriptor-policy value is specified. Tick-input records serialize optional `previous_policy`; replay must compare policy. The current island has an explicit `RootMeta.policy: ?SessionPolicy = null` at `zig/src/planet_economy/state.zig:120`. Importing a default creator policy instead would produce different first-tick previous-policy bytes from importing that null state. “All other local economy semantics” does not unambiguously define a new coordinator descriptor's genesis representation.
- **Suggestion**: Define genesis descriptor policy explicitly, preferably null until the first published tick, make its descriptor type optional accordingly, and require every local and global first-tick previous_policy to match that value. Alternatively accept an explicit configured initial policy and serialize it in genesis. Add independent genesis/first-empty-tick/policy-transition fixtures, including failed and canceled first ticks preserving the genesis policy.
- **Status**: open

### Issue 4: Entity exhaustion conflicts with inherited command rejection semantics

- **Severity**: minor
- **Priority**: Minor clarification
- **Section**: Routing, roles and local economics; Work, failures and performance proof
- **Description**: The specification inherits “all other local economy semantics” but line 406 says “A target aborts on ... tick/ID exhaustion.” The approved island specification states checked exhaustion rejects creation without consuming an ID, and current `zig/src/planet_economy/command.zig:862`, `:907`, `:926` emits a normal `IdExhausted` gameplay rejection for settlement/building/shipment creation. This leaves ambiguous whether the integrated adapter must intentionally change entity exhaustion into a whole-galaxy abort or retain command rejection while treating coordinator counters differently. Natural entity exhaustion is unreachable within the proposed million-tick horizon and bounded batches, so this is not a practical scale blocker, but failure-injection tests need one rule.
- **Suggestion**: Enumerate exhaustion behavior for entity IDs, job IDs, sequence watermark, tick/revision, descriptor generation and lease generation. Explicitly state any integrated semantic override, and forbid wrapping/reuse of generations that could resurrect stale tokens. Ensure the expected receipt/abort result for injected entity exhaustion follows the chosen contract.
- **Status**: open

### Strengths

- Correctly identifies existing integration barriers: fixed identities in `ids.zig`, local target derivation in `job.Authority.beginOwned`, automatic island publication, invisible history-tail finalization and obsolete-root selection in `Candidate.advanceDiscard`. The proposal does not mistake sequential island polls for a globally atomic operation.
- Specifies all-world phase barriers and deterministic local command subsequences, including command-free worlds, global sequence consumption for published gameplay rejections and a recorded trusted policy on empty ticks. Existing `play_role.zig` confirms the reused role/provenance tags and creator/ruler admission model.
- Retained immutable descriptors and lease backpressure reconcile hidden per-planet marker changes with old/new complete-galaxy reads using two roots. Explicit ownership of partial COW operations addresses the current candidate cleanup hazard.
- Requested-memory arithmetic is internally consistent: four 8,642,560-byte planet reservations plus 12,582,912 coordinator bytes total 47,153,152. The existing planet `Reservation` asserts the advertised layout; the new coordinator remains Proposed and must supply its own alignment/control assertions. Source-plus-target replay memory is separately disclosed.
- Canonical receipt domains distinguish integrated and island identities. Header ordering matches current `history.Encoder.next`; routed inputs preserve rejected commands, and prepared/commit records bind every local head and count. Maximum global staging demand is correctly bounded at 262 records.
- CLI tick1–4 arithmetic is consistent with the approved local economy: farm output six, food consumption four/two, house cost eight, construction completing the following tick and no growth before eight fed ticks. Seed2/derived seed low bits make cell71 fertile on both worlds; approved topology witnesses establish 71/east→4 and hall70 adjacency to 69/71/74. Partial-install abort, retry, old-view retention and postcommit cancel are concrete acceptance witnesses.
- Performance values are labeled targets. Independent wire fixtures, work/allocation instrumentation, cancellation cuts, maximum capacities, replay adversarial cases and the Rust-blocked Zig gate are future proof obligations rather than invented results. Deferred strategic mechanics, persistence and hosts remain mandatory subsequent work.

### Evidence and verification limits

Current evidence consists of full specification reading, approved island contract, integration/acceptance audits and focused reads of `zig/src/planet_economy/{ids,state,job,command,history,replay}.zig`, `zig/src/play_role.zig` and `zig/src/surface.zig`. The frozen SHA-256 command exited 0 and matched the assigned input. No Rust execution, Zig compilation, gate, benchmark, allocator fault test or CLI execution was performed for this document-only review. New coordinator layout, runtime cancellation and replay remain unimplemented/unverified Proposed behavior; historical qualification logs were not rerun. No external system claim was needed for these findings.
