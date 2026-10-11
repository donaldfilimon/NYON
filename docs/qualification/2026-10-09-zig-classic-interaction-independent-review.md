# Task3I2 independent implementation review

2026-10-09 package; reviewed after frozen delivery. Specification compliance: **PASS**. Code quality: **PASS**. No Critical, Important or Minor findings identified in the accepted pure interaction scope. Parent aggregate gate remains Pending.

## Scope and method

Read home/repository instructions and execution brief before the accepted dispatch binding, its full independent review, additive lifecycle correction and scoped closure. Read complete Rust interaction module including its tests, exact Rust lifecycle callers and clock suspension callback, complete Zig production module/tests/root, actual typed Session action and payload declarations, root/README baseline delta, full-context package and evidence. No compiler, tests, fmt, gate, product edits, subagents, commits or branches were used by this reviewer. Only this new ignored review file was written. Current source verification uses read-only hashes; qualification below is archived worker evidence independently inspected, not reviewer execution.

## Specification and implementation

`zig/src/classic_interaction.zig:71` preflights finite position, shared raw u64 duplicate, then available inline capacity before any mutation. Eight occupied entries and the ninth rejection are explicit; no eviction, coalescing, source tagging or device maximum is implied. Capacity failure retains the entire state, including active pinch. Zero capacity returns ContactLimit. Generic capacity has no default; the accepted actual-action specialization is eight in the tests.

Owner precedence at lines83–87 matches Rust: HUD overrides every source/button, mouse-primary world is TapOnly, empty primary or remaining middle/touch-primary world is OrbitOrTap, secondary is ImmediateLaunch. Update at line120 keeps strict staged f32 subtraction/sqrt and >=6 movement, sticky moved, and last-update delta. End at line141 removes first, preserves last finite position on nonfinite release, applies end-only threshold suppression without emitting a final movement delta, and returns null for unknown/repeated IDs. Typed HUD release uses full std.meta.eql action equality; source disabled/current-context effects remain outside this module. Touch-primary world tap does not implicitly launch.

Pinch scan at line92 sorts the two raw IDs after counting exactly two non-HUD touches. Every touch begin invokes detection, including HUD touches that can restart an existing pair. Mouse/HUD contacts are excluded; a third scene touch retains its independent owner without recomputing the existing pair. Update replaces baseline before invalid/epsilon checks and computes logarithm difference times600. Coincident/epsilon/nonfinite baseline transitions recover without an initial jump. Pinch end clears baseline and converts remaining pinch owners to moved TapOnly, without automatic re-pairing. Whole cancel resets all state. These details match source rather than a redesigned multitouch policy.

The recursive value-only guard at lines8–28 rejects pointer/slice/function/resource-shaped fields, recursively handles arrays/optionals/structs/tagged unions, and rejects untagged unions. The actual Session.UiAction is a self-contained typed value used only by tests; production imports std and classic_math only. No allocation, callback, Session/Core/camera effect or borrowed action lifetime enters the production path. The guard is source-reviewed; no independently executed negative compile fixture is claimed.

Finite admitted inputs may derive nonfinite orbit displacement, as in Rust. This source distinction is retained and tested; finite camera-consumer admission is still a required later router decision. No clamp, invented coordinate ceiling or finite-output claim is introduced.

## Fixture and receipt review

Fourteen interaction tests plus three imported model tests are the reported17, not17 independent new interaction tests. The18-row ownership expectations are literal values independent of the implementation branch expression. Fixtures cover all source/button/hit combinations, movement equality/stickiness/last and zero delta/diagonal proximity, release-only launch, end-only suppression, typed payload equality and inequality, all actual action variants, FieldKind and settings enum values, bool values and WorldId boundaries. They cover eight/ninth admission with existing pinch, cross-source duplicate IDs and ordered errors with whole-state equality, zero/empty/one/full/cancel/reuse, unknown/nonfinite events, logarithm signs with independent decimal tolerances, coincidence/epsilon/recovery, third-contact behavior, HUD restart, sorted-ID permutations, survivor suppression and derived overflow recovery.

Archived red2 contains the incomplete neutral API and first admission-count fixture before the implementation. It compiles and executes expected1/found0, terminal exit1, with three imported model tests passing. This is behavioral RED. Later added fixtures are honestly not claimed individually RED; aborted initial snapshot directories and later compile/fixture repairs are not mislabeled RED.

Independently inspected debug-literal, safe-literal and fast-literal full output and terminal receipts:17/17 pass in Debug, ReleaseSafe and ReleaseFast, exit0. fmt-literal checks the two new Zig files and root, exit0 with empty output. Every final precommand source identity matches the corresponding archived snapshot and current final four paths. Precommand metadata agrees with the receipt; all retain executable path/hash/version, argv, cwd, UTC times and private cache environment. The executable is /Users/donaldfilimon/.zvm/master/zig, version0.18.0-dev.120+9fe22a29b, SHA2567a6a4ee02ae6277258a699503d2396fbfbc86704b6216cd4e52c7cc502334bfb. This review did not invoke it.

All three final test outputs report actual target sizes action3, optional entry40, controller328 bytes against asserted ceilings64/128/1088. These are object sizes, not whole-process or stack measurement. The exercised100-cycle callable witness and inspected allocation-free production path support the limited allocation-free claim; no heap tracing, fake allocator or OOM result is claimed.

## Frozen identity verification

Read-only verification found zero mismatches across all2038 manifest entries, all75 protected current paths, final copies and current four-path source identities. Full-context package includes only the two new files, one root export/test import and narrow README addition. All final command snapshots match their precommand source maps. Frozen identities:

- Report: a013c530b8aae8c31c3ed5fb0d63e25897b0fcd29d0418873f132d5d844a786b
- Final source record: 9b23e154fabdda4dc955f4f156df18b685dbd6dc16a5005a4275250fca887012
- Full-context patch: d17197a89596e406b7ef6545b2f7969654e1a587578d1f4fb4c77653a2ec7e71
- Manifest: baaf0cbe9543019696bfdcc81f322eaa262afd86ea3a2bf017c1928aecbe481b
- Production: 1f47633662151e7bc4127682420854d19a9ebb21d2d7d86e0b3751d7dd2d4910
- Tests: 9cf55171984191d734d075816d5962886b58a9abb34fa8fdad25b4012cc102a7
- Root: 7e0fb30484cd3f910a9d24bd88195bc11b16b05381e63dc73998fc981c6245d6
- README: 6d620b5109404771cd66f859558e476aecf535b330af31e982c9b13df1fe6776

## Limits and pending work

Current: reviewed pure interaction module and archived focused qualification only. Pending: parent aggregate gate, actual Session/router wiring, HUD-first hit testing and disabled/current capability validation, owner/event freshness, world repick, pointer/DPR conversion, capacity rejection/backpressure, nonfinite derived-output consumer rejection and native/browser host delivery. No host-game acceptance follows from this review.

Lifecycle correction remains exact: absent-guide Open clears contacts; already-present Open/Show/Close/MainMenu arms do not independently cancel; Focused(false) and TouchPhase::Cancelled clear the whole Classic interaction controller; suspend clears input/clock while preserving contacts/baseline; occlusion does not cancel Classic contacts. Pure cancel fixtures do not qualify these later caller seams or actual event ordering.
