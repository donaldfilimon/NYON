# Living Resident Runtime Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development, one fresh implementer per task and independent task review before the next source release. No commits or worktrees: Donald explicitly selected the canonical dirty checkout.

**Goal:** Keep resident execution, reclamation and private presentation within one update budget.

**Architecture:** ClientRuntime owns durable retirement outside SandboxSession. Session preparation and cache owners publish together only after the capture fence.

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

### Task 1: Durable runtime retirement

**Files/responsibility:** Modify ClientRuntime, SandboxSession disposal/admission and opaque runtime retirement handoffs/tests.

**Interfaces:** One prospective boundary and one retirement task; retirement continues hidden/no resident; incoming rejected owners retained.

- [ ] Capture ownership and first-touch identity scope; add focused named regression assertions for the stated behaviors.
- [ ] Implement the smallest complete child-and-parent slice: Adopt jobs, tokens, stale rejections, retired publications and rejected incoming sessions without nested synchronous destruction. Drain before next admission; cap requests20 and do not queue full cancelled candidates. Test pause/leave/replacement/history/refusal/hidden cleanup and exact retained ownership.
- [ ] Run the focused tests, preserve actual failures, then freeze and run covering documented gates.
- [ ] Self-review full first-touch diff and exhaustive owner/count map; write report/logs/identities and stop writes for fresh independent review.

### Task 2: Private presentation and shared scheduler

**Files/responsibility:** Modify resident scheduler, Chronicle/decision/guidance/audio preparation and tests.

**Interfaces:** One1024-visit remaining budget includes retirement/history/boundary/cache; max20 publications.

- [ ] Capture ownership and first-touch identity scope; add focused named regression assertions for the stated behaviors.
- [ ] Implement the smallest complete child-and-parent slice: Prepare deep rows/labels/evictions and cues privately, move them only after successful commit; stale/cancel zero rows/cues. Preserve integer1/4/20, dropped excess debt, exactly-one Step, controls clear prospective timing/request debt but preserve accepted commands. Rolling actual-publication1s pace omits zero-duration denominator entries. Test deterministic time groups, multiple publications, every presentation cut, staleness and lifecycle journeys; assert no resident compatibility-helper call.
- [ ] Run the focused tests, preserve actual failures, then freeze and run covering documented gates.
- [ ] Self-review full first-touch diff and exhaustive owner/count map; write report/logs/identities and stop writes for fresh independent review.

