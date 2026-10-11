# Task3I1 key-edge frozen independent implementation review

2026-10-09. Current: full review of the frozen second helper chunk. Production specification compliance: PASS by source inspection. Qualification: Partial, one Minor fixture correction required. Critical 0; Important 0; Minor 1. Parent aggregate gate is pending. No router, host or whole-game acceptance follows.

## Evidence and scope

Read home and repository guidance, complete frozen key binding and preparation review, accepted foundation identity and FIX1 review/gate receipt, entire appended helper implementation and all eight appended tests, exact README delta, actual Rust engine input and Classic action/navigation mappings and keyboard routing precedence. Read actual archival runner and focused receipts/outputs; independently hash-verified all 684 manifest artifacts and 73 protected live identities. Only this ignored review is written. No compiler, test, formatter, aggregate gate, product edit, Git mutation or subagent was run by this reviewer.

Verified frozen SHA256 values:

- Binding: 817e44c2c8dfad328ed0a03c2a4a6dbc48fe3ff07568e933d4833cf0878d9f35.
- Preparation review: 1dd81d949131ebd419395fdacd082c460c4fd3c1d95ca45cc5e3db6d22e2e47e.
- Baseline-source receipt: b0c84b73774ffcef77022f25ff865af6a29c5ce26be9e8a8ec10380be76d351b.
- Report: 7efd4d3b24e47101e300e699772cf21d07f35ff36ff8b5cf7f9d5e4e6df9e198.
- Artifact manifest: a26ea3715dae49b130056854745a986e5d5e6c2656074d6efa34f9e4683b454a.
- classic_input.zig: 9e02aaff2b09abbcacf5f0fa1912c4e25ca3803e7d9043ae0eb83fe8c6c91a45, 17943 bytes.
- classic_input_tests.zig: e5fa10ce0e5a4d30ebc56c70443b4b353e4612dd23514f31dfd9a25715cc3750, 34796 bytes.
- root.zig: 2404ec9281fd811875c8e879902a48d918463463d7c1d640d72ed007b23279c5, 3863 bytes unchanged.
- README: 08cf371bca2764fa991414575bf7960ac0385ffe1957b8f9c9b9b8e5563a0566, 46152 bytes.

The exact 12833-byte production foundation and 25516-byte FIX1 test foundation are retained as byte prefixes. Root is exactly unchanged; README's 45242-byte baseline prefix is unchanged. All four final copies match current bytes and final-source receipt. Independently regenerated full-context source/test patches equal archived patches byte for byte. Final Debug/ReleaseSafe/ReleaseFast/fmt-check pre-command production, paired-test and root copies match final source. The runner copies the Zig tree before invoking version and command, records UTC times, cwd, argv, isolated caches, full outputs and terminal statuses. Manifest hashes validate archived source copies and command/version outputs. The executable's current hash matches every recorded executable identity: 7a6a4ee02ae6277258a699503d2396fbfbc86704b6216cd4e52c7cc502334bfb. Recorded version is 0.18.0-dev.120+9fe22a29b with version exit 0. This is archival evidence inspection, not a new compiler invocation.

## Finding

**Minor M1 — pending-clear fixtures never begin with a pending edge.** Location: zig/src/classic_input_tests.zig:549–590, especially 568–584. Each action's pending bit is consumed at 568 and checked false at 569 before clearTransient at 571. All bits remain false before finishFrame at 576 and clearAll at 579. Consequently a clearTransient that did nothing, finishFrame that did nothing, or clearAll that cleared only held would still pass these pending-clear assertions. Production classic_input.zig:324–332 correctly implements all three by inspection, so this is a regression-coverage defect, not a demonstrated runtime defect. The binding explicitly requires clearing every pending edge while preserving held for transient/frame clear and clearing both sets for all-clear; the report's claimed reset jury is broader than these executable witnesses.

Correction: create unconsumed edges immediately before each of clearTransient, finishFrame and clearAll; assert pending false afterward, held preserved for the first two and false for clearAll. Use independent setup for each method so an earlier clear cannot mask a later no-op. Preserve release/repress semantics and frozen prior evidence. Update correction provenance instead of rewriting this original review. This finding does not request router work.

## Specification and quality assessment

classic_input.zig:220–244 preserves complete borrowed UTF-8 character/optional text independently, and separate complete preedit/commit payloads. Validation scans whole strings; it has no queue-derived size limit or key-text control filter on IME. No allocation, retained token, escaping borrow, platform dependency or state/effect mutation is introduced.

classic_input.zig:246–275 requires a one-byte complete string before ASCII lowercasing, which is equivalent to Rust whole-string to_ascii_lowercase followed by the literal one-byte mapping table. It does not select the first byte/codepoint of a longer input. All 15 action rows match src/app.rs:2520. Named Space/Escape/F4 supply launch/cancel/scenario; named Enter/F1 and character Space remain unmapped. Unicode folding/lookalikes, combining sequences and multi-scalar strings are excluded. Tests cover literal rows, uppercase equivalents, unsupported and every unmatched named value. Source buffers are not modified.

classic_input.zig:276–296 matches src/app.rs:2636: navigation is named-only and only Shift affects Tab. Tests exercise every named row and all 16 modifier snapshots. Command mapping and InputState contain no modifier veto or blanket repeat filter. Carrier metadata stays distinct from held state. A first repeated down after clearAll can produce an edge; repeated down while held cannot. No special F1/H routing guard is incorrectly added to these helpers.

classic_input.zig:297–304 validates before scalar iteration and rejects exactly Unicode Cc ranges U+0000–001F and U+007F–009F anywhere. Empty, space, combining, emoji and format U+200D are allowed. The fixtures sample both Cc ranges, interior controls, malformed continuation/overlong/truncated/surrogate/out-of-range sequences; they are not an exhaustive executable test of every Cc scalar, although the complete interval implementation is explicit.

classic_input.zig:307–340 uses two fixed 15-action boolean arrays and exhaustive enum indices. No slice/event is stored. Methods reproduce src/engine/input.rs:55: down only creates an edge on false-to-true, release clears held without destroying pending, consumption preserves held, cycles coalesce, aliases share the action bit. Every action and cross-action independence is exercised. Temporary key-buffer poisoning cannot affect state. Compile-time assertions pin count, exact array types and two-field structure; focused output measures native InputState=30 bytes, bool=1, actions=15. This does not establish stack/process footprint or a cross-target layout promise.

Large IME fixture retains both carrier identities to one 8195-byte buffer, including newline and final multibyte scalar, with no cap/truncation/control veto. This proves borrowing/validation, not an actual editor insertion or host forwarding effect. README accurately calls the surface helpers only and separates later routing/IME/host work. No foundation API or semantic queue ownership change is present.

## Executable evidence disposition

RED at 2026-10-09T23:38:04.254742Z–23:38:07.274746Z is a valid behavioral test failure: the neutral introduced actionForKey returned null, literal Q=>decrease failed TestExpectedEqual, 22 passed/1 failed, child exit 1. The exact witness remains unchanged in final tests and passes in each final run. No correct source mapping was deliberately regressed.

Archived format/debug parser failures and debug2 reflection API failure are honestly reported implementation failures, not behavioral RED. format2 exits 0. Final debug3 (23:39:28.712727–23:39:31.489569 UTC), safe (23:39:31.656644–23:39:38.540136), fast (23:39:38.709436–23:39:47.065462) each have exit 0 and all 30 tests passed, including the Q witness and native size output. These are the same 30 tests per mode: eight new plus 22 inherited/imported, not 90 distinct tests. fmtcheck (23:39:47.260873–23:39:47.308011 UTC) exits 0 and covers both helper files.

Prior accepted foundation gate receipt records 448 Debug tests and terminal exit 0 with no identity drift; it qualifies the earlier foundation, not this new delta. This review does not rerun or promote it into the pending aggregate gate. Prior 4227/398 artifact sets were not rehashed here; their accepted prior reviews remain evidence for those frozen sets, while this review independently verifies the current second-chunk 684 artifacts and retained prefixes.

## Deferred jury and remaining gaps

The binding calls these pure mapping/edge fixtures qualification of the second chunk only, and explicitly leaves router/editor/guide/host witnesses unexecuted later obligations. Its synthetic I3 precedence bullets define a later literal jury, not a required fake router implementation in this helper-only chunk. Deferring them is consistent with that scope. No implementation can establish ordered F1/guide/editor/Settings/Playing precedence merely from the helper outputs; actual I3 must test those branches, disabled/focused control policies, caller clear boundaries and typed Session effects against current source.

Current: helper semantics by source inspection and focused archival runs. Partial: pending-clear regression coverage and fresh parent aggregate qualification. Proposed/unverified: actual Session routing, keyboard hovered-world launch, editor confirmation/OOM/IME effects, complete focus preservation and host integration. Out of scope: raw host conversion, synthetic filtering, modifier snapshot correctness, IME ranges/protocol, keyboard/IME duplicate delivery, native/browser/manual/accessibility and whole-game acceptance. Unknown: actual stack/process footprint. No heap or retention is introduced by helpers; ordinary borrowed lifetime validity remains caller-owned.

Verdict: no Critical or Important defect found. Resolve M1 with an independently re-reviewed additive fixture correction before aggregate acceptance. Do not claim pending-clear executable qualification from this frozen jury alone.
