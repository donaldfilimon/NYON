## Design Document Review: NYON owned Zig UI atlas design — fix 2

### Summary

**PASS for planning.** The candidate-only publication outcome finding is resolved, and no blocking direct regression was found. This verdict concerns the specification and implementation-planning handoff only; it is not generator, filesystem, runtime, visual or retirement qualification.

Reviewed specification: `docs/superpowers/specs/2026-10-11-nyon-zig-ui-atlas-design.md`, SHA-256 `b65ded7fbb2b7b920e6ec4a72b7c91009bb52d5c0188afcd1865832ee5b4f0db`. Scope was solely the major finding in `2026-10-11-zig-ui-atlas-design-fix1-review.md` and its direct regressions. Both earlier review files remain unchanged.

### Resolution: Candidate publication and selection promotion now have distinct success contracts

- **Original severity**: major
- **Section**: Transactional publication and check mode, lines 176–225
- **Assessment**: Lines 178–179 explicitly terminate candidate generation after its own file/directory/parent barriers, with no selection or release-receipt access. Lines 184–192 define fresh and idempotent `CandidateDurable` success separately from `CandidateNotPublished` and `CandidateDurabilityIndeterminate`. Candidate cancellation is honored before rename/re-sync and deferred through its barriers thereafter; failures preserve final/preexisting directories for recovery. Lines 194–203 retain the selection-specific outcomes and commit point. Thus an uncertain candidate directory never implies an uncertain selection replacement that was not attempted.
- **Exit/result consistency**: Line 207 permits candidate success as `CandidateDurable`/0, selection success as `CommittedDurable`/0, and read-only verification as `Verified`/0. Candidate durability uncertainty is exit 6; selection-commit uncertainty is exit 5. Promotion's artifact-stage failure before selection replacement remains `NotCommitted`/2. A content check does not claim durability.
- **Status**: resolved at design level

### Strengths

- The eleven focused fixture rows include fresh/existing candidate success, early refusal/cancellation, post-rename and existing-directory sync failure, deferred cancellation, read-only verification, and both pre-selection and post-selection promotion failure. The proposed filesystem recorder rejects any candidate-only selection/receipt access.
- Candidate generation can proceed where its own directory barriers are supported even if selection replacement is unavailable. Unsupported candidate durability still refuses publication success; Linux/macOS/Windows filesystem behavior remains explicitly unqualified until independently demonstrated.
- The preceding durability barriers, compiled-source/executable binding, fixed arithmetic and artifact-relative identity rules remain intact. The original two major and two minor findings retain their resolved design status recorded by fix 1. No new runtime capability or external dependency was introduced by this correction.

### Verification scope

A read-only specification check independently validated all **11** focused rows against operation-specific outcome/exit invariants; checked the retained numeric literals previously verified in fix 1; and verified the specification pin plus both previous review hashes. It completed with **exit 0**. This checks the written truth table, not an implementation or filesystem simulation. The unchanged arithmetic/source-binding clauses were reread for direct regressions; no broader review or new feature demand was added.

The reviewed hash was rechecked at completion. A separate new-file whitespace/hash check completed with exit 0. Only this new review file was written. No source, assets, other documents, ledger, AGENTS files, dependencies or commits were changed. No builds, game gates, generator execution, filesystem durability tests, cross-host runs or visual acceptance ran. Those remain required implementation and qualification evidence before release or retirement.
