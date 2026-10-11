## Design Document Review: NYON Zig coordinated galaxy — fix 1 rereview

### Summary

**Approve the revised written design for human consideration.** All four prior findings are addressed, and no new blocking issue was found in the changed contracts. This is independent technical review, not Donald's written-spec approval, implementation authorization or runtime qualification; the specification remains Proposed.

Reviewed revision SHA-256 `7dce7fad01c04da0b1b7e3de005883169238d5711a254efc2985fda176f7a6a2` of `docs/superpowers/specs/2026-10-11-nyon-zig-coordinated-galaxy-design.md`. Scope: the four findings in the original review and breakage introduced by their corrections. The original review remains unchanged; its SHA-256 was recorded as `350eb8073039386bcc41aad41f86920efe362938390e898055bd920e64e58b8a` before writing this rereview.

### Issue 1: Fault shutdown cannot satisfy ordinary destruction preconditions

- **Severity**: major
- **Priority**: Important
- **Section**: Abort, cancellation and reclamation, lines 349–374
- **Resolution**: Addressed. Permanent faulted poll/cancel results are separated from once-only ordinary terminal notification. `beginFaultShutdown` explicitly closes acquisition and mutation while existing leased value queries and release remain legal. `destroyFaulted` requires shutdown and zero leases, frees the independently owned reservations through a creation-time allocation ledger, and bypasses undrainable page/refcount cleanup. Ordinary deinit explicitly returns Busy for faulted galaxies. Deallocation is an explicit operation outside poll budgets; ownership-ledger corruption is outside the claimed recoverable fault model.
- **Qualification obligation**: The added undrainable-refcount witness must prove repeated fault results, published read availability, acquisition closure, Busy while leased and exactly-once reservation frees after release. These tests are requirements, not executed evidence.
- **Status**: addressed

### Issue 2: Global history capacity absent from creation configuration

- **Severity**: major
- **Priority**: Important
- **Section**: Limits, identities and memory; Canonical history and replay
- **Resolution**: Addressed. Creation config now includes `global_history_capacity:u16`, default 4,096 and range 1..4,096. Genesis consumes one slot. Exact tick demand `2 + planet_count + command_count` agrees with tick_input, all routed inputs, all planet_prepared records and tick_commit. Checked demand is tested against committed occupancy and the configured limit before preparation. Replay uses the exact serialized capacity. Fixed allocation bytes remain distinct from logical admission limits.
- **Qualification obligation**: Added capacities 1/3/4 fixtures correctly distinguish valid genesis-only creation, one-short failure and exact-fit one-planet empty tick, followed by exhaustion. Global/local capacity failures must preserve the committed policy and sequence.
- **Status**: addressed

### Issue 3: Genesis policy undefined

- **Severity**: major
- **Priority**: Important
- **Section**: Architecture, ownership and interfaces
- **Resolution**: Addressed. Descriptor trusted policy is explicitly optional; all local and global genesis policies are null. Tick/revision/watermark begin at zero. First local/global tick_input records encode absent previous_policy as one zero byte, subsequent previous_policy matches committed policy, and failed/canceled first ticks preserve null. This matches the inspected island `state.RootMeta.policy` null convention without inventing a creator default.
- **Qualification obligation**: Added independent genesis/first-empty-tick/policy-transition checks must establish exact previous/current bytes and failure/cancellation preservation.
- **Status**: addressed

### Issue 4: Entity exhaustion versus inherited rejection semantics

- **Severity**: minor
- **Priority**: Minor
- **Section**: Work, failures and performance proof
- **Resolution**: Addressed. The counter table now retains entity `IdExhausted` as a gameplay rejection, matching current island command behavior. Job/revision/descriptor exhaustion fails before lane admission; tick horizon is explicit. Maximum command sequence is accepted once without increment, while later empty ticks remain legal. Descriptor and lease generations never wrap/reuse; exhausted released lease slots are skipped, and generation reservations are explicitly scheduling metadata excluded from canonical replay.
- **Qualification obligation**: Separate injected counter witnesses are now required, including entity rejection versus coordinator admission failure, maximum sequence followed by an empty tick and stale-token protection across exhausted slots.
- **Status**: addressed

### Strengths

- Corrections preserve separate planet ownership, all-world phase barriers, hidden installation, one-descriptor publication and two-root lease backpressure.
- Fault destruction frees allocation ownership directly rather than trusting the structure that already reported a cleanup invariant defect.
- Canonical capacity and genesis-policy choices now have one reproducible interpretation for construction, first-tick records and replay.
- Added proof obligations directly exercise each correction; neither the revised specification nor this review promotes them to implemented evidence.

### Evidence and verification limits

Read the revised specification and focused changed lifecycle/counter sections; the source checks from the original review remain the basis for comparisons to island behavior. Hash verification matched the assigned revision. No new broad codebase audit, Rust/Zig execution, heavy gate, benchmark or runtime test was performed. No product source, specification, original review, instruction file or user data was changed; only this rereview was written. No commit or external publication occurred.
