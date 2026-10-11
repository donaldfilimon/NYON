# PROPOSED, NOT DISPATCHED: Task 3I4 pure durable exit

2026-10-09. Concrete source binding for future implementation. This preparation writes only this new ignored document and its report. No compiler, product gate, product source, durable ledger, dependency, agent instruction or commit changes. The foundation mutator retains classic_input/tests/root/README ownership. This slice is independent of I1/I2/I3 consumers, with parent-controlled sequential dispatch. [docs/qualification/2026-10-09-zig-classic-controller-execution-slices.md:17]

## Source behavior and scope

Rust state is one default-false pending boolean; three action variants are ExitNow, PrepareWorkshop, Wait. request(false) returns immediately before pending examination or mutation. Thus an already pending state survives that action. request(true) sets pending on first request and returns PrepareWorkshop, then Wait while pending. observe_workshop returns Wait when absent regardless of ready; pending false-ready waits, pending true-ready clears and exits. cancel returns prior pending and clears it. [src/app/durable_exit.rs:4; src/app/durable_exit.rs:21; src/app/durable_exit.rs:32; src/app/durable_exit.rs:44]

The actual caller handles Living dirty/save with a separate living_exit_pending field before reaching this state. It derives requires_save from active Workshop and !workshop_durable_ready. PrepareWorkshop calls return_to_main_menu and logs rejected preparation without cancelling pure pending. Polling checks Living separately, then pending; it supplies workshop_durable_ready, sets quit_requested on ExitNow and retries return_to_main_menu on Wait. These calls and logging are caller behavior, not this module's effect. [src/app.rs:1938; src/app.rs:1962; src/app.rs:1985]

Shell Escape closes guide first, then consumes a successful durable-exit cancellation. Wrapper cancellation clears the separate Living flag first, otherwise calls pure cancel. Pure cancellation is therefore not proof a mutation/store job was cancelled. [src/app/input_router.rs:466; src/app.rs:1985]

Pure API has no error union: the Rust operations are total for boolean inputs and have no I/O failure output. PrepareWorkshop is a request for caller preparation, ExitNow a decision, Wait a decision. No host-close invocation, archive completion, Classic battle autosave, actual Workshop/Living storage adapter, timeout, retry limit, new policy, or new save-state taxonomy is introduced. [src/app/durable_exit.rs:1; src/app/durable_exit.rs:21; src/app.rs:1938]

## Exact proposed product API and implementation

Future worker owns ONLY NEW zig/src/durable_exit.zig and zig/src/durable_exit_tests.zig. Freeze their absence again immediately before admission. Public Zig export is a representation choice required for cross-file caller/test access, not Rust visibility expansion into host integration. Field remains implementation state; consumers call isPending/request/observeWorkshop/cancel. Naming follows existing Zig camelCase. No allocator, pointer escape, retained token, deinit or init allocation. A fresh .{} value resets state exactly as Rust Default. Copying this value copies independent state, not an owned resource. [src/app/durable_exit.rs:4; src/app/durable_exit.rs:16]

Proposed complete `zig/src/durable_exit.zig`:

```zig
pub const DurableExitAction = enum { exit_now, prepare_workshop, wait };

pub const DurableExitState = struct {
    pending: bool = false,

    pub fn isPending(self: DurableExitState) bool {
        return self.pending;
    }

    pub fn request(self: *DurableExitState, workshop_requires_save: bool) DurableExitAction {
        if (!workshop_requires_save) return .exit_now;
        if (self.pending) return .wait;
        self.pending = true;
        return .prepare_workshop;
    }

    pub fn observeWorkshop(self: *DurableExitState, continue_ready: bool) DurableExitAction {
        if (!self.pending) return .wait;
        if (continue_ready) {
            self.pending = false;
            return .exit_now;
        }
        return .wait;
    }

    pub fn cancel(self: *DurableExitState) bool {
        const previous = self.pending;
        self.pending = false;
        return previous;
    }
};
```

## Independent literal matrix and paired tests

Expected actions/states below are literal deductions directly from Rust branches, never computed through the Zig implementation or captured as goldens. Pending rows establish state with a first save-required request; this setup itself is asserted against literal prepare_workshop/true. Every row asserts output AND post-state. All eight request/observe combinations and both cancellation combinations are covered. [src/app/durable_exit.rs:21; src/app/durable_exit.rs:32; src/app/durable_exit.rs:44]

|Start pending|Call|Input|Return|End pending|
|---|---|---|---|---|
|false|request|false|exit_now|false|
|true|request|false|exit_now|true|
|false|request|true|prepare_workshop|true|
|true|request|true|wait|true|
|false|observeWorkshop|false|wait|false|
|false|observeWorkshop|true|wait|false|
|true|observeWorkshop|false|wait|true|
|true|observeWorkshop|true|exit_now|false|
|false|cancel|none|false|false|
|true|cancel|none|true|false|

Proposed complete `zig/src/durable_exit_tests.zig`:

```zig
const std = @import("std");
const exit = @import("durable_exit.zig");
const State = exit.DurableExitState;
const Action = exit.DurableExitAction;
const expect = std.testing.expect;
const eq = std.testing.expectEqual;

test "durable exit literal request and observation matrix" {
    const Method = enum { request, observe };
    const Case = struct {
        pending: bool,
        method: Method,
        input: bool,
        action: Action,
        after: bool,
    };
    const cases = [_]Case{
        .{ .pending = false, .method = .request, .input = false, .action = .exit_now, .after = false },
        .{ .pending = true, .method = .request, .input = false, .action = .exit_now, .after = true },
        .{ .pending = false, .method = .request, .input = true, .action = .prepare_workshop, .after = true },
        .{ .pending = true, .method = .request, .input = true, .action = .wait, .after = true },
        .{ .pending = false, .method = .observe, .input = false, .action = .wait, .after = false },
        .{ .pending = false, .method = .observe, .input = true, .action = .wait, .after = false },
        .{ .pending = true, .method = .observe, .input = false, .action = .wait, .after = true },
        .{ .pending = true, .method = .observe, .input = true, .action = .exit_now, .after = false },
    };
    for (cases) |case| {
        var state: State = .{};
        try eq(false, state.isPending());
        if (case.pending) {
            try eq(Action.prepare_workshop, state.request(true));
            try eq(true, state.isPending());
        }
        const action = switch (case.method) {
            .request => state.request(case.input),
            .observe => state.observeWorkshop(case.input),
        };
        try eq(case.action, action);
        try eq(case.after, state.isPending());
    }
}

test "durable exit cancellation returns prior pending and permits restart" {
    var state: State = .{};
    try eq(false, state.cancel());
    try eq(false, state.isPending());
    try eq(Action.prepare_workshop, state.request(true));
    try eq(true, state.cancel());
    try eq(false, state.isPending());
    try eq(false, state.cancel());
    try eq(Action.prepare_workshop, state.request(true));
    try eq(true, state.isPending());
    try eq(Action.exit_now, state.observeWorkshop(true));
    try eq(false, state.cancel());
    try eq(Action.prepare_workshop, state.request(true));
}

test "durable exit immediate decision preserves pending save until cancellation" {
    var state: State = .{};
    try eq(Action.prepare_workshop, state.request(true));
    try eq(Action.exit_now, state.request(false));
    try eq(true, state.isPending());
    try eq(Action.wait, state.request(true));
    try eq(Action.wait, state.observeWorkshop(false));
    try eq(true, state.isPending());
    try eq(true, state.cancel());
    try eq(false, state.cancel());
    try eq(false, state.isPending());
}

test "durable exit ready observation clears preserved pending save exactly once" {
    var state: State = .{};
    try eq(Action.prepare_workshop, state.request(true));
    try eq(Action.exit_now, state.request(false));
    try eq(Action.exit_now, state.observeWorkshop(true));
    try eq(false, state.isPending());
    try eq(Action.wait, state.observeWorkshop(true));
    try eq(Action.wait, state.observeWorkshop(false));
    try eq(Action.prepare_workshop, state.request(true));
}

test "durable exit state copies are independent and allocation free" {
    try eq(@sizeOf(bool), @sizeOf(State));
    try eq(@alignOf(bool), @alignOf(State));
    var original: State = .{};
    try eq(Action.prepare_workshop, original.request(true));
    var copy = original;
    try eq(true, copy.cancel());
    try expect(original.isPending());
    try expect(!copy.isPending());
    // A comptime execution witness forbids runtime allocation/I/O on these paths.
    const witness = comptime blk: {
        var state: State = .{};
        if (state.request(true) != .prepare_workshop) break :blk false;
        if (state.request(false) != .exit_now or !state.isPending()) break :blk false;
        if (state.observeWorkshop(false) != .wait) break :blk false;
        if (state.observeWorkshop(true) != .exit_now or state.isPending()) break :blk false;
        if (state.cancel()) break :blk false;
        break :blk true;
    };
    try expect(witness);
}
```

Memory assertions compare target size/alignment to target bool, not a guessed ABI byte ceiling. Report actual @sizeOf/@alignOf on the pinned target during implementation. Source inspection must show sole bool field and no allocator/import in product module; the proposed comptime witness plus real lifecycle tests establish allocation-free callable paths. Do not invent OOM fixtures for an allocator-free API or claim a dormant failing allocator proves no global allocations. This state has no borrowed data, dangling slices, shutdown frees, queue limits or ref reservations. Those remain I1/I3 concerns. [src/app/durable_exit.rs:5; docs/qualification/2026-10-09-zig-classic-controller-execution-slices.md:17]

## Fresh admission, RED/GREEN, coordinated integration

1. Parent must accept independent review of this binding and explicitly dispatch the two new files. Before dispatch establish no active owner of those paths and confirm absence with fresh identities. Re-hash Rust module/callers and cited preparation inputs; a changed behavior requires rebind/review. Preserve every other concurrent edit. Current root/README reads are observations only, not baselines frozen for later edits.
2. Implement test-compilable API scaffolding with an intentional incorrect behavioral branch, for example request(false) incorrectly clearing pending. Run the preserved-pending witness to a real assertion failure. Save exact source hashes, compiler identity, argv, target, exit code and bounded log. A missing import, compiler syntax failure or absent module is not behavioral RED. Then implement correct early return and run the same witness GREEN. Do not leave a deliberate mutant in product or replace independent expected literals.
3. Run standalone paired suite Debug, ReleaseSafe, ReleaseFast with pinned Zig and private caches, serially coordinated with compiler owner. Proposed focused commands from repository root: `zig test zig/src/durable_exit_tests.zig -O Debug`, then `-O ReleaseSafe`, then `-O ReleaseFast`; verify actual .zigversion and executable before running. Save outputs/exits and before/after source identities; focused tests do not imply parent integration. Format both new files with repository Zig formatter. No compiler invoked in preparation.
4. Independent reviewer reads complete new module/tests and focused evidence, checking especially the preserved pending early return, absent-ready Wait, prior-state cancel, rearming and no caller fabrication.
5. ONLY parent, after foundation mutator releases root/README, owns fresh baseline and two additive root.zig hunks: `pub const durable_exit = @import("durable_exit.zig");` and `_ = @import("durable_exit_tests.zig");` inside existing test block. README change is at most an existing implemented-surface inventory entry with pure-decision limitation. Do not rewrite status tables, gate descriptions, host/game completion claims or unrelated entries. Parent may omit redundant README wording. Re-read full fresh diff, preserve foundation imports, record root/README pre/post hashes and coordinate ownership before mutation. [zig/src/root.zig:1; zig/src/root.zig:57]
6. Parent runs fresh pinned `./zig/tools/check.sh` after coordinated imports and freezes exact integrated hashes, exit/log and test totals for reviewer. This gate checks implemented Zig surface and explicitly does not certify unported modules or host qualification. No source/compiler/gate changes for this prep; future native/browser/manual/store receipts remain separate. [zig/tools/check.sh:1; zig/tools/check.sh:19]

Gaps: Proposed implementation and tests have not compiled or executed. RED/GREEN, target size output, focused three-mode receipts, independent implementation review and coordinated parent gate are future requirements. No host/store completion evidence exists in this binding.

## Read-only source SHA-256 freeze

Hashes are observations from this preparation, not accepted implementation or frozen future root/README ownership. All paths below were actually read; source citations use repository-relative line locations.

|Path|SHA-256|
|---|---|
|`src/app/durable_exit.rs`|`aa3d6156bf7400bbdf11f3a8b661ba7d977f9fcfd4a3758ea36b68465397f3b8`|
|`src/app.rs`|`67b489723ec3886b38873f12ac6a21e763d0f6fc6d21c709b85fd27730eaaea2`|
|`src/app/input_router.rs`|`966a9618f3dc44dc25feae1e09f79ad5ee4f83b24189df2601b477e3a49cc68a`|
|`docs/qualification/2026-10-09-zig-classic-controller-execution-slices.md`|`db78dd944b41ff15777ff67411199d076c2b93ca2659d5836e776e59c1297b60`|
|`zig/tools/check.sh`|`8bb44a18f4e069509ff96024ab056d41bf65afc1a49cbc5ad2081f624fdbdd96`|
|`zig/src/root.zig`|`44d6a0f02814153e0b1be8650aae203f3b5acecb31314af0dad4cfba96771f8c`|
|`.superpowers/sdd/2026-10-09-nyon-zig-migration/task-3i-routing-intake.md`|`d67931577d8a9d0a3174748e8f844a4aec26649e79642e027d2de42a5319be5e`|
|`.superpowers/sdd/2026-10-09-nyon-zig-migration/task-3i-routing-brief.md`|`d2a5702b16fbbc3bf8145d94dbb011ab770ade972c9376483222f6ef99481e7e`|
|`.superpowers/sdd/2026-10-09-nyon-zig-migration/task-3i-routing-brief-fix1-review.md`|`2dab0d66b560a5adf8440c8fe2ca9522eb2916485fffcae11961ca16b6e13816`|
