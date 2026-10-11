### Spec Compliance

- ✅ Spec compliant for Task3F3's authorized headless integration/projection scope. AppCore owns the controller and pending dispatch by value; initialization and installed replacement schedule synchronously without replacing controller history (`zig/src/classic_app.zig:102,116,324`). Each successful step schedules from its actual post-state and canonical fingerprint, after authority success and before the next step (`zig/src/classic_app.zig:161,311`). Failed steps return before advisory scheduling.
- ✅ Session consumes successful installation's stored restart effect once, preserving camera/focus handoffs (`zig/src/classic_session.zig:241`); only the two corresponding existing Session assertions change (`zig/src/classic_session_tests.zig:558,566`). Existing AppCore tests have no diff hunk, consistent with the authorized unchanged baseline.
- ✅ Typed projection copies all retained values, searches World.id, owns the WorldName and uses source backend/state labels (`zig/src/classic_advisory_projection.zig:4,15`). Numeric score retention correctly leaves percentage formatting to the later owner.
- ⚠️ Parent aggregate gate, independent protected-file ledger verification, actual GPU adapters, renderer/native/browser/readability/a11y and full-game acceptance are not established by this diff. They remain explicitly pending; this verdict does not close those owners.

### Strengths

- Per-step integration preserves authority ordering and allocation-failure recovery. The real second-step allocator-failure journey checks the prior successful publication, campaign digest, pending queue and sticky hold/recovery (`zig/src/classic_advisory_integration_tests.zig:117`). A separate witness checks a nonempty failing queue and failed installation (`zig/src/classic_advisory_integration_tests.zig:233`).
- The sixty-step journey independently derives material/cadence admission from one-step reports and literal cadence 30, checks exact IDs/provenance/queued metadata and compares complete final publication/queue/history with the batch (`zig/src/classic_advisory_integration_tests.zig:47`). Holds, rejected-only steps and pure projection queries consume no request (`zig/src/classic_advisory_integration_tests.zig:88`).
- Host dispatch is consumed by value; completion promotion and explicit device failure bind the controller without retained authority pointers (`zig/src/classic_app.zig:142,152,157`). Moved owner and dispatch witnesses exercise the real APIs (`zig/src/classic_advisory_integration_tests.zig:147`).
- The deferred F2 newest-fallback proof gap is covered: matching older MapFailure must republish the newer CPU score array and full metadata, with CPU FALLBACK and no remaining updating work (`zig/src/classic_advisory_integration_tests.zig:178-185`). This is an explicit publication assertion rather than only a disposition assertion.
- Real moved Session open/cancel/invalid apply/confirmation/success/restart operations verify installation once and preserved preferences/guidance (`zig/src/classic_advisory_integration_tests.zig:256`). The panel witness checks owned name retention, valid-priority unknown-name fallback and literal backend/state/source tick (`zig/src/classic_advisory_integration_tests.zig:199`).

### Issues

- Critical: none found.
- Important: none found.
- Minor: none required for this task-scoped gate.

### Assessment

**Task quality:** Approved.

**Reasoning:** The diff implements the bounded source contracts with a small allocation-free scheduling seam and meaningful real-operation tests. No blocking correctness, ownership or scope defect was found; broader host/graphics acceptance remains with its designated later owners.

### Checks and Review Identity

- Verified report SHA256 `9c023504c7ebf2603cf59f311f1af7e3b04d3d4639107ed54b7aa0c010d10f3e` and full-context diff SHA256 `a93d84e15c035cb0adc54ec2f0fa73b146164f7c161774d47689e8b1c1a1e13e`. Base/head `bac2b6e313f8034309bd718d3b8cb4e0448ac20f`; authoritative package compares frozen uncommitted/untracked baseline, not a commit range.
- Read the supplied diff in one review pass. Initial tool transport truncated its middle; retrieved the omitted sections from the same diff. No changed production source was separately reread.
- Named concrete interface checks outside the diff: controller ownership/correlation and completion outcome contract, checked `zig/src/classic_advisory_controller.zig:25-139`; applied lifecycle producer ordering and effect storage, checked `zig/src/classic_editor_transactions.zig:156-175`, with open/close effect assignments at lines 47 and 59. These confirm the consumer's changed handoff semantics; no broader crawl was performed.
- Inspected final focused Debug/ReleaseSafe/ReleaseFast logs: 11 tests each, including ten F3 tests and root harness; receipts record exit 0. Inspected cross-task log endpoints: Classic 70 and 3D 24 passed, receipts exit 0. Fmt log is empty and receipt exit 0. Existing Classic allocation-ordinal diagnostic is informational, not a warning/failure. Exploratory 3B/3C harness-only runs are excluded.
- No test/build suite rerun, product edit, dependency/profile action, branch/worktree/commit or subagent action occurred. Only this authorized review file was written.
