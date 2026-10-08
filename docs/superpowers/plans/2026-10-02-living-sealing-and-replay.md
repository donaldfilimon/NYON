# Living Sealing and Replay Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development, one fresh implementer per task and independent task review before the next source release. No commits or worktrees: Donald explicitly selected the canonical dirty checkout.

**Goal:** Use one complete executor for sealed authority publications and every interactive replay path.

**Architecture:** Prepare and preallocate privately, then commit only a freshness check and owner movements. Archive children poll the same executor using remaining work; keep historical modes and identities.

**Tech Stack:** Rust edition 2024, nightly-2026-09-01; existing pure nyon-workshop-core and native/wasm resident runtime.

**Spec:** Donald's approved revised boundary requirements of 2026-10-02 21:28 EDT, recorded in `2026-10-02-complete-bounded-living-resident-boundary.md` under Revised approved contract.

## Global Constraints

Preserve every existing uncommitted change. No dependency, schema migration, version/identity-formula change, branch, worktree, commit, push or PR. One product/build writer at a time. Verify adopted source identities and exact first-touch snapshots before release. Poll budgets are 1..=1024 actual visits; invalid budgets use zero work. Variable encoding visits emit at most 256 bytes. Reclamation retains original nested owners. Resident updates share 1024 visits and publish at most twenty boundaries; pending requests are capped at twenty. Complete corrected version-2 rules in place; preserve old files and safely reject affected archives that no longer replay. Record parity exceptions only for approved rule corrections. Historical evidence does not qualify changed source. Automated, live and measured performance evidence remain separate.

## Review Focus

- Stale success/failure must not mutate authority or publish presentation.
- Failed or cancelled partial work must retain original nested allocations until counted retirement.
- Children must charge failed work once and use only the parent's actual remaining budget.
- Corrected rules must have independently derived public-path expectations, not shared-cursor parity alone.
- Historical replay rejection must leave files, active sessions and accepted authority commands intact.

## Execution evidence

The existing complete-boundary SDD ledger remains the recovery map. Keep reviewed baselines, raw gates and complete task diffs; do not repeat completed prerequisite work. Each task follows focused regression/implementation/frozen covering gates/self-review/fresh independent review. Preserve development failures without claiming behavioral RED from compilation errors. Final whole-candidate review and fresh platform gates are mandatory.

### Task 1: Incremental sealing and publication

**Files/responsibility:** Modify Step preparation, receipt/facts/decision encoding and journals, existing validator/hash composition and retirement inventories.

**Interfaces:** Add LivingStepProgressV2::Ready and take_preparation(&mut self)->Result<LivingStepPreparationV2,LivingStepJobErrorV2>. Add immutable state(), receipt(), decision_receipt(), event_facts_receipt() references on LivingPreparedBoundaryV2.

- [ ] Capture ownership and first-touch identity scope; add focused named regression assertions for the stated behaviors.
- [ ] Implement the smallest complete child-and-parent slice: Set tick then validate/hash; preserve sorted applied IDs versus accepted journal order, receipt/facts/decisions/recovery/corridor fault order, retained-prefix omissions and canonical fields/domains. Encode variable payload visits <=256 bytes; prepare all publication owners before Ready. Incomplete/cancelled extraction retains owners; successful extraction once; valid repeated terminal polls0; current opaque failure alone latches authority. Test full three-receipt/state/journal parity, every stage stale/cancel, arithmetic/capacity and chunk accounting.
- [ ] Run the focused tests, preserve actual failures, then freeze and run covering documented gates.
- [ ] Self-review full first-touch diff and exhaustive owner/count map; write report/logs/identities and stop writes for fresh independent review.

2026-10-03 oracle preservation clarification: the current old running boundary scheduler and commit logic are separately implemented, but several phase wrappers and staged receipt comparisons reuse adopted cursors or the exact sealing suffix. Before public step is rewired, retain that old orchestration as a private cfg(test) reference and compose preserved independent phase kernels where available. Keep the original corrected-rule controls and independent vector encoders; canonical machinery reuse does not establish an independent encoder oracle. A test that calls `reference.step()` after both authorities use the new executor cannot qualify old-vs-new parity. Exact provenance and remaining gaps are recorded in the complete-boundary ledger's read-only oracle report; this is a test design requirement, not fresh qualification.

### Task 2: Unify public authority step

**Files/responsibility:** Modify authority step and explicit synchronous compatibility/test oracle routes.

**Interfaces:** Public step drains the complete executor and same commit; transitional variants remain declared but are never emitted.

- [ ] Capture ownership and first-touch identity scope; add focused named regression assertions for the stated behaviors.
- [ ] Implement the smallest complete child-and-parent slice: Remove running production synchronous suffix. Keep old running algorithm cfg(test) oracle and noninteractive explicit adapters. Commit only fences/moves. Test full outputs, faults and accepted tails at fixed/mixed budgets with only named corrected-rule exceptions.
- [ ] Run the focused tests, preserve actual failures, then freeze and run covering documented gates.
- [ ] Self-review full first-touch diff and exhaustive owner/count map; write report/logs/identities and stop writes for fresh independent review.

### Task 3: Complete interactive replay/history

**Files/responsibility:** Modify archive replay/comparison/history and their exhaustive retirement inventory/tests.

**Interfaces:** Advance/gaps, Running/Paused, Fork, finalization and comparison use remaining work; preserve branch/mode/high-water and saved-present behavior.

- [ ] Capture ownership and first-touch identity scope; add focused named regression assertions for the stated behaviors.
- [ ] Implement the smallest complete child-and-parent slice: Remove whole-state copy/hash/encoding/error-clone/destruction from interactive forward paths. Cover every stage cancellation, repeated forks and authentic comparison-finalization failure. Affected version2 archives fail existing typed validation before installation without storage rewrite. Test both changed affected fixtures and unaffected byte-identical replay.
- [ ] Run the focused tests, preserve actual failures, then freeze and run covering documented gates.
- [ ] Self-review full first-touch diff and exhaustive owner/count map; write report/logs/identities and stop writes for fresh independent review.
