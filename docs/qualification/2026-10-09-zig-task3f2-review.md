### Spec Compliance

- ✅ Spec compliant for the pure Task3F2 controller: complete five-field correlation (`zig/src/classic_advisory_controller.zig:16`), bounded latest-only queue and immediate CPU publication (`:68-80`), matching-fault-before-latest-stale ordering (`:83-92`), sticky failure/new epoch behavior (`:38-48`, `:94-119`), and wrapping cadence admission (`:60-67`) match the supplied brief and inventory.
- ✅ Same-epoch availability=false retains ownership; completion accepts still-latest valid work without an availability gate and clears queued work instead of inventing fallback (`zig/src/classic_advisory_controller.zig:38-45`, `:83-92`, `:111-119`; literal two-case witness `zig/src/classic_advisory_controller_tests.zig:105-121`).
- ✅ Request/snapshot/outcome fixed arrays own their values; published borrow lifetime is explicit (`zig/src/classic_advisory_controller.zig:20-34`, `:56-58`). No retained observation/fleet/authority/host pointer appears in controller storage.
- ⚠️ Cannot verify from this task diff: later per-step AppCore/Session integration, host pending dispatch, real GPU/readback/native/browser/scene/full-game qualification and cross-target arithmetic. These are correctly deferred; the parent must run its aggregate gate before F3 dispatch.

### Strengths

- Literal tests independently perturb all five metadata fields and compare the complete controller before/after mismatches (`zig/src/classic_advisory_controller_tests.zig:47-60`); unrelated error completions cannot consume or disable owned flight.
- Six old matching failure variants cover typed error, NaN, Inf, lower/upper range and parity mismatch before obsolete rejection, then same-epoch toggles and historical-publication retention (`zig/src/classic_advisory_controller_tests.zig:76-104`).
- Copy/move witnesses replace the original observation/controller and accept the owned dispatch; separate real Simulation witnesses preserve a nonempty pending command and accept the requested source after three nonmaterial steps (`zig/src/classic_advisory_controller_tests.zig:134-155`, `:176-226`). CPU-score copies are disposition witnesses, not independent arithmetic oracles.
- The code is small, allocation-free and keeps completion validation/publication/queue promotion in one auditable sequence (`zig/src/classic_advisory_controller.zig:83-125`). Root and README changes stay within exports and ownership/qualification documentation.

### Issues

#### Critical (Must Fix)

- None found.

#### Important (Should Fix)

- None found.

#### Minor (Nice to Have)

- `zig/src/classic_advisory_controller_tests.zig:94-96`: old-fault fallback asserts newest metadata/backend but does not assert newest CPU score array or complete stored publication. Add an exact score/publication assertion when next extending these tests; the implementation currently selects latest CPU correctly at `zig/src/classic_advisory_controller.zig:105-109`, so this is coverage polish rather than a behavior defect.

### Assessment

**Task quality:** Approved.

**Reasoning:** The implementation preserves the specified lifecycle and source's unusual availability edge while maintaining value ownership and bounded storage. No blocking code or scope issue was found; actual host/runtime qualification remains outside this approval.

### Reviewed evidence and limits

- Review package report SHA256 verified: `654988234467f4039d3e66bff6b26d9c9e31d649b5c9f33ed4e3ec9780a1f92c`.
- Full saved-baseline diff SHA256 verified: `17867ba43edf66c4651f3350d74f7229a6accfd7ca989d30b00627551dc81599`; base/head supplied unchanged as `bac2b6e313f8034309bd718d3b8cb4e0448ac20f`. The untracked before/after patch, not an empty commit comparison, was reviewed.
- Full diff read once; tool output truncated the final test tail/root context, so that omitted diff section was recovered from the saved diff. No changed source file was separately read, no unchanged-code crawl or git command was run.
- Retained `debug-final2`, `safe-final2`, `fast-final` receipts each record exit 0, exact pin `0.18.0-dev.120+9fe22a29b`, both isolated caches and identical final source identities. Their complete logs contain 31 OK lines and the pass total with no additional noise: nine controller tests plus 22 imported tests, not 31 newly authored controller tests. Retained fmt receipt records exit 0 for the final identities.
- Retained compiled RED shows admission assertion failures; earlier compiler/setup failure and incidental null panic are separately classified in the implementation report and are not independent ownership evidence. Older 14-test receipts have superseded source hashes.
- Retained protected-verification receipt records 42 identities and zero mismatches; this reviewer did not independently recompute the parent's freeze. No tests/builds/dependencies/profiles/product files/commits were changed or rerun; only this review report was written.
