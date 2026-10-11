# Task 3D FIX1 independent scoped review

**Spec compliance: APPROVED. Task quality: APPROVED. M1 resolved.** No Critical, Important or Minor findings remain in this supplementary scope. This verdict covers the native facade reopen proof and scoped lifecycle documentation clarification, not the parent full gate or full game acceptance.

## Identity and boundaries

Reviewed 2026-10-09 against the saved `task-3d-fix1-before/` baseline, original review's M1 and the frozen FIX1 report/source manifest. Implementation report SHA-256: `007059c9bbb5a874c19fdde7f3d95d4b41d9ab4305aa4649815b782e8d3f4162`. Exact patch SHA-256: `a488eade48432a68a9bfe285ce40f69ec940ce35a50171d0084ade8941760728`. Independent reconstruction of every patch hunk from the baseline and current files succeeded. Only `zig/src/classic_editor_transactions_tests.zig` and `zig/README.md` differ within the four owned paths; transaction implementation and root remain byte-identical. [task-3d-review.md, § Issues; task-3d-fix1-before-source.json; task-3d-fix1-after-source.json; task-3d-fix1-evidence/review-package.patch]

| Reviewed path | Final SHA-256 |
| --- | --- |
| `zig/src/classic_editor_transactions.zig` | `e4776d63c6f81c8d33371ecf5ee7db5ee59b85095226869f304ef2c0a20d3913` |
| `zig/src/classic_editor_transactions_tests.zig` | `22ebae197ccbfe7c2b1263e9d3951c99f74287b3a01cf5743c525236e9e76bbc` |
| `zig/src/root.zig` | `1145fda6c12639c294551355181cace34e59f834a26e4c4538138c50949fcc46` |
| `zig/README.md` | `363d748afba781c9fdfb9e94e248a6c72d34824cb17703942b4be6ab55ef997a` |

All four current hashes match the frozen manifest and final receipt source copies. All 30 protected hashes match the original dispatch manifest. Independently rehashed all 179 original artifact entries and all 61 FIX1 artifact entries: zero mismatches. Existing original report/review evidence is preserved. These are source/artifact identity checks, not rerun tests. [task-3d-fix1-after-source.json; task-3d-preserve-identities.json; task-3d-fix1-evidence/freeze/final-protected-after.json; task-3d-evidence/freeze/artifact-identities.json; task-3d-fix1-evidence/freeze/artifact-identities.json]

No production/test/documentation edits were made by this reviewer except this new raw report. No builds, gates, dependency operations, profiles, branch/index/commit mutations or subagents were run. Unchanged sources were read only to resolve the concrete ownership and effect-contract risks below.

## Resolved M1 and quality checks

**Current: fresh facade reopen proof closes M1.** After the real captured save confirms successfully, the test retains its original direct-file exact-byte comparison and detailed `NATIVE SAVED` decode. It then constructs a separate `NativeStore.atPaths(a, io, .scenario, primary, legacy)`, calls that new store's `raw().load(a)`, first asserts `.payload`, compares the returned bytes exactly with the independent pre-confirm capture, and separately detailed-decodes the literal world name `NATIVE SAVED`. This is an actual new native facade readback, not a renamed direct-file read or reuse of the first facade. [zig/src/classic_editor_transactions_tests.zig:495-517]

**Current: ownership and cleanup are correct.** The new store duplicates both path slices; its failure cleanup and deinit own those duplicated bytes. Raw load opens/closes the file inside the call and returns separately allocated payload bytes under the caller allocator. The test registers payload cleanup after store cleanup, so reverse defer order releases the load outcome before the reopened store. The inline decoded scenario owns no extra heap result; the codec releases its temporary draft internally. Outer original store/path/fixture owners remain live until their existing reverse cleanup. No escaped raw facade or double-owned payload was introduced. [zig/src/classic_store.zig:431-445,470-474,492-518; zig/src/classic_store.zig:72-83,97-110; zig/src/scenario.zig:208-213,497-508; zig/src/classic_editor_transactions_tests.zig:443-458,508-521]

**Current: preservation witnesses remain independent and scoped.** A literal preference sentinel is installed before the scenario transactions. Fresh readback asserts exact legacy and preference sentinel bytes and no `.nyon-classic-` temporary files. Existing injected create/write/sync/persist failures retain the scenario and legacy byte checks, now also asserting the preference sentinel. Fixture cleanup closes the disposable directory before deleting its temporary path. No real profile/environment resolution occurs in the added calls. [zig/src/classic_editor_transactions_tests.zig:411-438,459,518-533]

**Current: lifecycle documentation matches the source seam.** README binds consumption to this call's `opened`, `closed` or `applied` outcome, avoiding replay of stored effects after unrelated outcomes. Open/close only replace the stored effect value with `clear_focus`; successful apply replaces it with all three effects. The stated HUD distinction agrees with Rust: editor open/close preserve the stored HUD action identity, while restart resets it. This assigns a future consumer policy without altering transaction production or claiming graphical integration. [zig/README.md:135-147; zig/src/classic_editor_transactions.zig:40-61,164-174,190-239; src/app/core.rs:821-831,959-982]

## Independently inspected execution evidence

Read every final receipt and its complete log, rehashed each log, and matched all 34 source identities and four saved source copies to the frozen/current identities. Every final receipt records the pinned compiler `0.18.0-dev.120+9fe22a29b`, actual child exit 0 and distinct receipt-local `ZIG_GLOBAL_CACHE_DIR`/`ZIG_LOCAL_CACHE_DIR`. The evidence runner obtains the compiler version, snapshots source before invocation, records the actual subprocess return code, writes the complete log and exits with that same child result. No reviewer test rerun is claimed. [task-3d-fix1-evidence/run.py; zig/.zigversion; task-3d-fix1-evidence/05-final-debug/receipt.json; task-3d-fix1-evidence/06-final-release-safe/receipt.json; task-3d-fix1-evidence/07-final-release-fast/receipt.json; task-3d-fix1-evidence/08-final-fmt/receipt.json]

| Final receipt | Recorded command/result | Verified complete log SHA-256 |
| --- | --- | --- |
| `05-final-debug` | Pinned Zig `test zig/src/classic_editor_transactions_tests.zig -O Debug --test-filter 3D`; child 0; 23/23 | `2d3ccf34e2eb97d76828345da2c600c09c20ee75f7d9c72ec5854abd63c337ec` |
| `06-final-release-safe` | Same focused target with `-O ReleaseSafe`; child 0; 23/23 | `2d3ccf34e2eb97d76828345da2c600c09c20ee75f7d9c72ec5854abd63c337ec` |
| `07-final-release-fast` | Same focused target with `-O ReleaseFast`; child 0; 23/23 | `2d3ccf34e2eb97d76828345da2c600c09c20ee75f7d9c72ec5854abd63c337ec` |
| `08-final-fmt` | Pinned Zig `fmt --check` on transaction source/tests/root; child 0; empty log | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

The three 4,109-byte test logs each show the native composition case passing, all 23 tests passing and the unchanged 64 Memory plus 36 Native operation OOM ordinal observations. The new reopen assertions extend the existing test; they do not introduce a new exhaustion family. FIX1 is explicitly supplementary GREEN evidence with no claimed behavioral RED. Original behavioral RED history remains in its unchanged original artifacts. [task-3d-fix1-evidence/05-final-debug/log.txt; task-3d-fix1-evidence/06-final-release-safe/log.txt; task-3d-fix1-evidence/07-final-release-fast/log.txt; task-3d-fix1-report.md, § Honest test provenance and final qualification; task-3d-fix1-evidence/freeze/qualification.json]

## Issues and proof limits

**Critical:** None. **Important:** None. **Minor:** None. **Original M1:** Resolved at `zig/src/classic_editor_transactions_tests.zig:508-521`.

**Outside this review / Partial:** The parent's new full gate, actual Task3E effect consumption, live native/browser/GUI delivery and full Task3/game acceptance are separate work. This review neither runs nor adopts the parent gate result. New reopen assertions use normal allocations; inherited exhaustive allocation tests remain separately scoped. [task-3d-fix1-report.md, § Final source identities; zig/README.md:140-147]
