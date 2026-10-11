### Spec Compliance

- ✅ Spec compliant for Task3F1: all six owned paths appear in the saved full-context diff; narrow copied-world/hazard and synchronous const-fleet observation, fixed constants/results and all requested APIs are present (`zig/src/classic_advisory_types.zig:3-17`, `zig/src/classic_advisory_features.zig:3-49`, `zig/src/classic_advisory_model.zig:14-64`, `zig/src/root.zig:20-25`, `zig/README.md:275-292`).
- ✅ Exact source arithmetic is preserved: array-index IDs, all-fleet saturating faction sums, i128 subtraction/squaring followed by f64 minimum/division and f32 conversion, affected-field-only adjustment before clamp (`zig/src/classic_advisory_features.zig:4-33`, `:45-49`).
- ✅ Weight offsets and sequential strict f32 evaluation match the contract; priority sanitizes scores and uses strict greater-than, and finite parity remains separate from score range eligibility (`zig/src/classic_advisory_model.zig:5-35`, `:42-64`).
- ✅ Independent factory bit literals, source feature/weight literals, seven packing rows, saturation/capacity, owner/hazard/distance and authority isolation witnesses are meaningful (`zig/src/classic_advisory_reference_tests.zig:13-107`, `:108-176`). No production-evaluator-generated expected bits were introduced.
- ⚠️ Cannot verify historical Rust bit-witness target/compiler provenance, cross-target float behavior, actual GPU parity or native/browser acceptance from this package. These are explicitly unqualified at `zig/README.md:286-292`; local scalar bits do not establish them. Parent aggregate gate remains pending and is required before F2 proceeds.

### Strengths

- Allocation-free scoring returns owned arrays/scalars and retains no campaign, simulation, queue, allocator or platform capability (`zig/src/classic_advisory_types.zig:10-17`, `zig/src/classic_advisory_features.zig:3-42`, `zig/src/classic_advisory_model.zig:14-40`).
- Tests deliberately exercise inconsistent World.id values, semantically unusual hazards and more than the scene fleet capacity, catching common port shortcuts (`zig/src/classic_advisory_reference_tests.zig:42-79`, `:97-107`).
- Authority tests use real pending envelopes and fleet allocation, compare fingerprint/queue/seed/counters and retain value results across move/deinit/replacement (`zig/src/classic_advisory_reference_tests.zig:148-176`).
- Near-tie acceptance explicitly permits different priorities, preserving the source's separate validity/parity policies (`zig/src/classic_advisory_reference_tests.zig:124-142`).

### Issues

#### Critical (Must Fix)

- None found.

#### Important (Should Fix)

- None found.

#### Minor (Nice to Have)

- `zig/src/classic_advisory_reference_tests.zig:84-96`: the distance fixture has one Union candidate. A future small literal fixture with two Union candidates, the nearer one encountered last, would directly protect minimum reduction against an accidental first-candidate implementation. Current production reduction is correct (`zig/src/classic_advisory_features.zig:13-20`); this is optional coverage, not a compliance blocker.

### Assessment

**Task quality:** Approved.

**Reasoning:** The pure implementation matches the immutable source contract and has useful independent reference and edge-case tests. Approval is limited to F1 and the supplied local evidence; parent aggregate validation and later controller/GPU qualification remain separate.

### Checks and Evidence Identity

- Read the full-context diff once. Tool presentation omitted a few unchanged root import tokens, with no truncated function requiring changed-source rereading. No git commands, builds, test execution, product edits or subagents were used.
- Named concrete risk: arithmetic/normalization order could differ despite matching high-level features. One focused unchanged-source check of `src/advisory/mod.rs:311-448` confirmed hazard multiplication-before-division, i128→f64 distance order, sequential f32 evaluation and separate parity/range semantics.
- Read staged final-debug/final-safe/final-fast logs: each contains nine passing tests, no skips/failures/warnings. Read saved feature/model RED and GREEN logs: failures are behavioral assertions against zero stub results; GREEN totals are one and two respectively. Saved receipt records exact pinned compiler `0.18.0-dev.120+9fe22a29b`, separate local/global caches, final exits 0 and fmt exit 0 (`.superpowers/sdd/2026-10-09-nyon-zig-migration/task-3f1-test-receipt.json`). This reviews existing receipts, not a new test run.
- Report SHA256 verified: `ceb84211c49292b14a1cfef33a01c970aed2a5f4ad196f3400efda463ea178c3`.
- Diff SHA256 verified: `525abaf0e85be1f83f54d9d137470236dd7f6c2e48d617584fab9d8b4e66b861`.
- Supplied unchanged base/head: `bac2b6e313f8034309bd718d3b8cb4e0448ac20f`. Review uses the saved baseline-inclusive package for untracked new code; no commit-range completion claim.
