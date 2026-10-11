# Task 3D independent review

**Spec compliance: APPROVED. Task quality: APPROVED.** No Critical or Important findings. One nonblocking Minor test-evidence improvement is recorded below. These verdicts cover the frozen headless Task 3D production behavior, not full Task 3, the game, hosts or graphical acceptance.

## Review identity and scope

Reviewed 2026-10-09 against the binding `task-3d-brief.md`, exact Rust trace `task-3d-editor-transactions.md`, current API intake and store-status clarification. Read the entire 90,243-byte package once, in contiguous bounded chunks. Package SHA-256: `05f0b2c6588368f160a6e9a5c2dfb472bce8d43de6e6aca2fc8242fc630ce469`. Implementation report SHA-256: `2753837585f699f6ad1107786e16a52aea07fe29fa89ec2b76f05f9c3e61185e`.

The package reconstructs exactly the saved `task-3d-before/` contents and the four current owned files. The two new modules were absent at dispatch; a HEAD-only diff would omit them. No production files, tests, dependencies, index, branches or commits were mutated by this reviewer. No tests or gates were run, and no subagents were dispatched. The parent full gate remains separate.

| Reviewed path | Frozen SHA-256 |
| --- | --- |
| `zig/src/classic_editor_transactions.zig` | `e4776d63c6f81c8d33371ecf5ee7db5ee59b85095226869f304ef2c0a20d3913` |
| `zig/src/classic_editor_transactions_tests.zig` | `52d0cfb42b8fd0e1af98b33ef1486bce5ce6ffa8dd5c756a2f3e8ff3e399eca7` |
| `zig/src/root.zig` | `1145fda6c12639c294551355181cace34e59f834a26e4c4538138c50949fcc46` |
| `zig/README.md` | `e50ea39b5130a6cb56fa974638be9a2ea0cc3943c3872b547a75b54de9bd85c0` |

Independently hashed all 179 entries in `task-3d-evidence/freeze/artifact-identities.json`: zero mismatches. All 30 protected source identities still match dispatch. Checked all 29 receipts, logs, available owned-source snapshots and protected-source manifests; final formatting has no separate source-copy directory but records the identical four final hashes. No immutability/source identity discrepancy was found.

## Strengths and contract checks

- **Current — ownership and integration:** One optional detached editor, one embedded capsule, typed store status plus action-only diagnostic, and no retained core/store/self pointers or duplicate mode flag (`classic_editor_transactions.zig:12–62,190–240`). Mode/session disagreement fails explicitly. Open stages active INITIAL content before entering Editing; close releases the editor and composes Playing/recover, preserving core campaign/queue/preferences (`:40–62`). Movement tests invalidate abandoned transaction/store variables and recreate raw facades after concrete-store moves (`classic_editor_transactions_tests.zig:860–918,1010–1035`).
- **Current — load/save publication:** Load uses raw presence, detailed draft decode, semantic validation and complete current-epoch preparation before publication (`classic_editor_transactions.zig:83–110`). Save orders parse rejection, detailed codec validation, canonical encoding and occupancy; returned arbitrary payload is occupied, while native raw errors block writing (`:127–153`). ORIGINAL and active authority are preserved. Wrong-record checks happen before callbacks (`:78–82`; tests `:205–230`).
- **Current — one-shot ownership:** Confirm moves the capsule, immediately restores an empty resident capsule with the original allocator and defers one local release (`classic_editor_transactions.zig:155–189`). Captured overwrite bytes are saved without reread/reencode/CAS; deferred release survives success, typed failure and OOM. Cancel explicitly displaces captured save/apply state (`:201–208`; tests `:368–391`). Dismiss has one kind-independent path to the frozen capsule cleanup (`:210–213`; frozen `classic_editor.zig:54–68`). No double ownership or post-install editor access was found.
- **Current — diagnostics:** Save semantic errors use `codec.validation`, Apply uses `validation`, and both preserve prior store status. Store outcomes transfer dynamic failure ownership once and clear old action errors; absence/save/load success use static tags. OOM propagates before publication and does not overwrite either resident diagnostic channel (`classic_editor_transactions.zig:63–77,83–153,190–194`; tests `:1037–1110`). No allocation follows successful durable save.
- **Current — apply comparison/install:** Fresh candidate extraction precedes complete meaningful comparison. The comparator checks both name/report/text bounds before slicing, covers every non-name world field and all live warning data, and ignores unused backing bytes/padding (`classic_editor_transactions.zig:164–175,245–264`). Tests exercise equal-fingerprint unequal values, missing capture, fresh invalidation, malformed live warning lengths, runtime fields and ignored unused capacity (`classic_editor_transactions_tests.zig:232–315,920–956`). AppCore installation happens before editor destruction. Effects remain explicit values for future camera/focus/advisory owners.
- **Current — reset recovery:** Revert clones ORIGINAL; defaults regenerate source defaults; regeneration uses typed seed/rules without prevalidation, stages all epoch state and preserves presentation/preedit/ORIGINAL/store status (`classic_editor_transactions.zig:178–188`; tests `:317–366,981–1007`). Period-zero/cap-invalid drafts are deliberately still editable and rejected later by Save/Apply.
- **Current — meaningful adapter and exhaustion checks:** Tests exercise actual MemoryStore byte/callback behavior and disposable NativeStore primary/legacy precedence, cap/UTF8/I/O distinction, exact captured durable bytes, decoded named values, create/write/sync/persist fault preservation and temporary cleanup (`classic_editor_transactions_tests.zig:51–230,442–518`). Literal malformed inputs, sentinels and named candidate expectations are independent of transaction return values. OOM snapshots include full current/ORIGINAL/worlds/rules, 90 buffers, parse flags, preedit, presentation, both diagnostic channels, core state and queue; slots/temp paths are checked separately (`:554–611,635–858`).

## Named unchanged-source risk checks

Inspection outside the package was limited to concrete contract risks:

1. **Diagnostic source parity and confirmation consumption:** Compared Rust `src/editor.rs:473–684` with the new dispatcher and the clarification. Store-message updates, semantic Save/Apply distinctions and one-shot take match the contract.
2. **Epoch/capsule ownership and absence of a duplicate owner:** Checked frozen `classic_editor.zig:34–115,206–222` and the existing all-modal cleanup test `classic_editor_tests.zig:318–356`. Replacement stages before resident release and retains the single capsule; cleanup is allocation-free and kind-independent.
3. **Admission/install reachability and preservation:** Checked `scenario.zig:236–285`, `classic_app.zig:278–309` and `simulation.zig:78–85`. Fresh validated extraction assigns positional IDs, zero initial runtime remainders, bounded names, checked rules/world values, derived warnings and a freshly computed fingerprint. Full equality establishes that the captured value has those same meaningful values. Both admissions therefore succeed under frozen invariants; simulation/clock construction is allocation-free. The catch at `classic_editor_transactions.zig:168` preserves the editor until installation succeeds, but no executed post-equality 3D admission failure is claimed or required via artificial injection. This is a source-reviewed defensive path.
4. **Open/close preservation:** Checked Rust `src/app/core.rs:821–831` plus frozen `classic_app.zig:130–155,210–226`. Active INITIAL projection and mode/clock/recover composition do not restart the campaign or mutate its queue/preferences.
5. **Raw store and native atomicity composition:** Checked frozen `classic_store.zig:180–243,409–452,499–582`. Memory returns arbitrary scenario bytes; native enforces cap/UTF8 before payload and falls back only on absence. Save prepares temporary-path/message allocations before replacement and cleans failed temporaries. The transaction does not reimplement persistence.
6. **Invalid-rule regeneration:** Checked `scenario.zig:73–96` and `generator.zig:45–77`. Generation discards rules and uses bounded fixed arithmetic; typed invalid rules are retained until later admission.

## Verified existing execution evidence

All receipts use the exact compiler `0.18.0-dev.120+9fe22a29b` and two distinct receipt-local cache variables. Reported exits below are Zig child exits, not the evidence wrapper exit. Source/log/receipt hashes agree with the frozen manifests.

| Behavioral TDD pair | RED child / actual failure counts | GREEN child / actual count |
| --- | --- | --- |
| `01-lifecycle-red` → `01-lifecycle-green` | 1 / 0 passed, 1 failed | 0 / 1 passed |
| `02-load-fixture-red` → `02-load-green` | 1 / 1 passed, 2 failed | 0 / 3 passed |
| `03-save-clean-red` → `03-save-green` | 1 / 3 passed, 3 failed | 0 / 6 passed |
| `04-apply-behavior-red` → `04-apply-green` | 1 / 6 passed, 2 failed | 0 / 8 passed |
| `05-reset-cancel-red` → `05-reset-cancel-green` | 1 / 8 passed, 3 failed | 0 / 11 passed |
| `14-status-parity-red` → `15-status-parity-green` | 1 / 22 passed, 1 failed | 0 / 23 passed |

Observed failures are runtime assertions against the old source snapshots, including the status-parity assertion `expected .no_saved_scenario, found .failure`. Compiler whitespace failure `04-apply-red` and helper-type failure `06-native-supplementary` are preserved and are **not** behavioral RED. Original fixture/early-cleanup artifacts remain preserved rather than substituted. The original pre-correction `11/12/13` runs remain 22/22 on their old hashes and do not qualify the corrected source.

Final frozen `17-corrected-freeze-debug`, `18-corrected-freeze-release-safe`, `19-corrected-freeze-release-fast`: each child exit 0, **23/23**, identical four reviewed source hashes. Each log records 64 Memory + 36 Native = **100 operation OOM ordinals**, followed by successful operation attempts, plus independently exhaustive open and native path initialization tests. The intentional `std.debug.print` family counts are successful test observations, not failure diagnostics. `20-final-fmt`: child exit 0 with empty log and identical final source hashes.

## Issues

**Critical:** None.

**Important:** None.

**M1 · Minor — fresh native facade readback evidence:** `classic_editor_transactions_tests.zig:499–504` reads persisted bytes through `Fixture.read`, checks exact capture equality and detailed-decodes `NATIVE SAVED`; it does not construct a fresh NativeStore and read that saved candidate through its RawStore facade, as the source-trace test checklist proposes. Existing native load exercises the actual adapter, and the readback independently proves durable bytes/codec validity, so this does not undermine production behavior or block this task. Add a newly initialized disposable NativeStore after successful confirmation, call its raw load, and assert exact bytes plus the named decoded candidate to close that narrow reopen-evidence gap. Do not describe this current test as an executed fresh-adapter reopen.

## Assessment and proof limits

**Task quality: APPROVED.** The implementation stays within the four-path ownership boundary and composes existing atomic seams without duplicated state. Error ownership, request OOM preservation, confirmation OOM consumption, meaningful value comparison and installation ordering are inspectable and supported by real adapter/exhaustion evidence.

**Partial/unverified outside this review:** Parent full gate, live native/browser/GUI behavior, actual lifecycle-effect consumers, crash durability beyond the inherited store contract, cross-process CAS and compiler-enforced linear ownership. The defensive post-equality admission catch is source-reviewed rather than independently executed. Full Task 3 and game acceptance remain Partial.
