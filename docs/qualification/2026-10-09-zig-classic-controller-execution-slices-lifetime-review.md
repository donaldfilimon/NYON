# Task3I execution slices lifetime correction independent review

2026-10-09. Status: Proposed. Verdict: ACCEPTABLE as a scoped specification-quality closure of Important I1. No new Critical, Important or Minor findings identified. This is preparation review, not implementation, runtime, target-layout, host-policy or product acceptance.

## Frozen evidence

Read the original execution slices, original report, original independent review, additive lifetime correction and correction report. Read stable H1 ownership/budget/identity context in `zig/src/retained_semantics.zig`. SHA256 verification command exited 0; assigned correction/report identities match exactly. No compiler, test, gate, Git, dependency, asset or product mutation was performed. No transient marker integration UI/tests/adapter/root/README was read. Only this new review document was written.

|Evidence|SHA256|
|---|---|
|task-3i-execution-slices-lifetime-fix.md|f33e008d82c5bf0c8eab4b857203385b3253370a4af4b19bc13b5fcd29b0e42f|
|task-3i-execution-slices-lifetime-fix-report.md|ee7e64a265a7ea1603233f82e3dd3f8e0f3918f508f73ccf8a087ee145982972|
|task-3i-execution-slices-review.md|3490d3f24efe5f8969b515534d2617af0ae122ebefa3b3e718a38fda95203f80|
|task-3i-execution-slices.md|db78dd944b41ff15777ff67411199d076c2b93ca2659d5836e776e59c1297b60|
|task-3i-execution-slices-report.md|99a1c08cbf0939d938ccd6c19f59c672fb864588505400b8bd7a13e6f20fc97a|
|zig/src/retained_semantics.zig|7e68e9b83d5b5da7e145a02bb8d2c04968558e22c307535b637cade597a735cb|

## Findings disposition

### Important I1 — registry lifetime and matching revision domain

- Severity: Important, inherited from original review.
- File:line: `task-3i-execution-slices-lifetime-fix.md:19`, `:21`, `:23`, `:25`, `:29`, `:31`.
- Description: Address-bound tickets now reject cross-instance use between distinct simultaneously live pinned registries before indexing or mutation. Same-instance generation checks reject canceled/released tickets after slot reuse. Overflow leaves entry and occupancy unchanged; the explicit conservative selected-exhausted-slot failure does not reset generations. Terminal deinit asserts quiescence, sets closed and preserves generations; closed reserve/mutations have deterministic rejection.
- Reinitialization assessment: The package selects the original review's permitted stable noncopyable/nonresettable lifetime alternative. Address reuse is not claimed to be a nonreused discriminator. Direct replacement, restoration, movement and same-address reinit of the admitted instance are explicitly forbidden caller operations, and no ticket copy may escape terminal shutdown. A wholly fresh coordinator requires destruction of every prior ticket/view/domain value. Thus the original retained-ticket/reinit counterexample is outside the newly stated supported contract, rather than falsely claimed to be detected by address equality.
- Revision assessment: Clock is scoped to the same coordinator, closed terminally only after queue/publication/current domains are destroyed, never reset for normal transitions/recovery, and checked for conflicts/overflow without mutation on failure. Exhaustion requires non-dispatching preservation. External source mutation and hidden value copies remain explicitly caller responsibilities, consistent with H1's UI-thread and ownership-by-convention contracts (`zig/src/retained_semantics.zig:17`, `:66`, `:107`, `:774`). Actual token retention still independently prevents allocation reuse while owning events survive (`zig/src/retained_semantics.zig:91`); it is not presented as registry teardown protection.
- Suggestion: Implement and qualify the exact runtime checks and caller shutdown protocol with the independent same-live-instance, cross-live-instance, exhausted/closed and revision-domain fixtures specified at correction lines 41–45. Require actual I3 coordinator wiring before integration acceptance. Do not introduce a same-address arbitrary stale-ticket rejection claim without a separately reviewed nonreused discriminator.
- Status: Addressed at specification level; implementation and coordinator qualification untested.

## New breakage assessment

No new finding. Added pointer/closed-state storage and RegistryClosed/ClockClosed errors are acknowledged, with queue unwind propagation and mandatory recalculation of ticket/slot/header/clock sizes (`task-3i-execution-slices-lifetime-fix.md:37`). Existing metadata<=256 and slot<=4096 ceilings remain unchanged; feasibility is explicitly deferred to authorized target qualification, not inferred from this read-only review. Queue constructor registry-address lifetime requirements remain in the original slices at line 98.

## Unaddressed boundaries

No unaddressed portion of I1 remains within the selected caller-constrained guarantee. Runtime enforcement of noncopyability, arbitrary address-reuse rejection, external-mutation detection and hidden-copy detection are not promised. Caller protocol must remain visible in future APIs/documentation and verified at I3.

Queue8×4096 remains Proposed: <=32768 slot storage plus actual header overhead, still requiring parent numeric disposition. Host delivery/drop/backpressure/retry policy remains unresolved. Final consumer/guide rebinding, stale-source/publication recovery, coordinator teardown and host acceptance remain later prerequisites. This verdict authorizes none of those decisions and establishes no executed check beyond document/source identity verification.
