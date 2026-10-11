# Task 3H2a independent frozen implementation review

2026-10-09. **Spec compliance: Approved for H2a with the explicit parent ScratchExhausted coverage ruling. Task quality: Approved.** No actionable Critical, Important or Minor defect found. Acceptance still requires the parent aggregate gate for these exact identities. This review does not accept H2b, a retained text consumer, host/rendering behavior or game completion.

## Scope and authority

Read home/repository AGENTS, SDD task-reviewer guidance, the complete accepted brief including FIX1, dispatch and foundation bindings, worker report, actual loader/tests and root/README changes. The dispatch's sole write allowance is this new review file; no source, test, asset, index, branch or other agent work was changed. No compiler, build, test or gate was executed by this reviewer. Parent runs aggregate independently. The new files were reviewed as full source, rather than only patch hunks; the full-context package and baseline copies bind the same scope.

Verified brief SHA ab2f0b4c98010388d61516f0d1048346c46db14b9b1562da9e11bb230aeeb46f; dispatch bc4c33edda33224c2d0fbe9acdbfcf19a3829164cbd6a2d7e1b347e89a41ef25; foundation 915e97167068cdc2c0badcba661941b4028e70e7f5f493bdaecef0225d0e87c2. Parent explicitly permits the narrow README appendix and accepts the unexercised public OOM branch subject to review/gate; neither permission is inferred from older Proposed documents.

## Frozen identities and preservation

Current independent SHA/length reads confirm all four live sources and all four final copies equal final-source.json: retained_atlas d67cb4dcfe71c089042f42c8f2eda882fd85c342530a99abaf40b4b5c9885627 (15600 bytes); retained_text_tests faa032256fa6dd431549ff00ecd03fe1f7928c91b90c1f6c46da57272a6564ea (20533); root ef33f8f45dadd7cc7649430e52f668b1eab4112b7fbf58a17b776c131f6668bc (3013); README 8b9a25e833f9af1d2d1aae542da7358f45bc0d761860be66defa99efce261b0f (32295).

Verified all 258 manifest artifact identities without drift, and all 58 live protected hashes against preserve-identities.json without drift. The three asset bytes independently equal git HEAD (each git show exit0), agreeing with protected-final.json. No new asset or generator provenance is implied.

Frozen package hashes: report 0373436bce808edaeb7d338409349a16b56b2e7b4fd8328d2c8be625c0108272; final-source 288fc8f969d3317e52ad3e63ecd213d39584771d2664ac5b92c0e5f25fdb2223; full-context patch 36c3e11abc7fd3cf9df65a4873054b625058d2e4031db7b793827298b095f80e; manifest affa0814ab688669d74f7967531e2f9c810f2304e003c2aa8c6fa1b21ed4ed5d; artifact index b8af0cf4966a067f09328bee24131b2b0c52197dca91f7d6851fcfdf340c5616.

## Contract and correctness checks

- Current: fixed numeric205-entry AtlasMetrics and closed owned diagnostics contain no parser/input/scratch references (zig/src/retained_atlas.zig:8,14,45,47). AtlasView is a resident borrow; shared H2b records are declarations only (48-53).
- Current: production precedence is length, digest, scratch/decode (70-75); fixture bypasses only digest (55-67). Parser uses caller prefix1048576 with strict unknown/duplicate handling, and Parsed is deinitialized before return. No heap retry or cached FBA context escapes.
- Current: checked complete address ranges reject overflow/overlap before scratch writes (58-59,92-98). Replacement separately protects resident against readable JSON and whole supplied scratch, stages candidate and assigns once (79-87). Failure preserves old resident/view, including complete byte witnesses (tests:43-87,150-168). A checksum failure can precede JSON/scratch alias diagnosis as explicitly required production admission precedence; it makes no scratch writes.
- Current: strict number/string/enum wrappers reject coercion, impose decoded token256 on borrowed and allocated values, preserve source integer widths and owned parse causes (119-175). Missing nullable optionals default null while required fields remain required (177-195). Explicit null and absence share optional semantics; canonical glyph/icon checks enforce the correct presence/absence (218-230).
- Current: descriptor constants, payload length, actual count and global key duplication are validated before fixed-table installation (198-217). Bounds addition is checked, cells are positive64/aligned, advances finite/nonnegative and overlap comparisons use previously validated bounds (232-243). The guarded optional dereference and bounded bufPrint catch unreachable at224 are proven by preceding null checks and the maximum fixed ASCII key length; no actionable unwrap, clone or lock issue.
- Current: root adds only atlas export and test import (zig/src/root.zig:38,41). README adds only its H2a appendix (zig/README.md:477); it retains honest exclusions and OOM gap. No retained_text implementation or consumer wiring was introduced.

## Independent fixture and receipt assessment

Mapping and bounds checks cover every205 entries using ASCII32+i%95, two95-glyph weight blocks, literal15-icon order and64-cell16-column formula (tests:58-64). Focused unchanged-source checks of Rust schema (src/ui.rs:130-186), icon order (354-374) and mapping (746-778) establish source authority. Those expectations are not harvested from candidate execution. Literal space/Regular A/SemiBold A advances (tests:65-67) are independent existing-metadata constants. The generic JSON reader's all205 advance transfer check (248-267) is hash-bound numeric preservation, not independent font generation, and the report correctly distinguishes them.

Tests independently exercise schema/type/descriptor/count/mapping/bounds/overlap/finite admission, trailing input, copied/borrowed token lengths, exact scratch and input boundaries, poison/reuse ownership, replacement rollback and full-range aliases (tests:18-32,43-96,97-218,248-287). The compact and escaped adversaries (219-245) genuinely decode full65536-byte overcount inputs; expected counts follow constructed input rather than decoder output.

Read existing qualified-Debug/Safe/Fast receipts and logs: each terminal exit0, seven tests, zero skips. qualified-fmt exit0; qualified-root exit0, eight tests including root importer. All five receipts bind the same four final files, their copied identities and log hashes verify, and pre-run records equal receipt pre-run fields. These are recorded executions, not reviewer reruns or aggregate evidence. Deliberate storage/adversarial measurement output is evidence, with no compiler warnings observed in qualified logs.

Read the three contemporary RED logs and copied source receipts: red-admission exit1/one behavioral failure; red-owned exit1/two behavioral failures; red-schema exit1/one behavioral failure, all compiled runnable tests against the earlier always-ScratchExhausted stub. They predate implementation and carry different contemporary source identities. They are not compile-error REDs and are not relabeled final qualification. Later historical fixture-construction failures are correctly excluded from RED claims.

## Resource evidence and limitations

Current recorded native sizes: AtlasEntry32, AtlasMetrics6600, Outcome6608, Generator28, Boundary16, GlyphPlacement48; RawMetrics80, RawEntry96, Parsed88, FBA24. Caller53288-byte JSON and caller1MiB scratch are separately physical; production contains no1MiB local stack array. The allocation-probe2 copied implementation uses the actual wrappers and reports frozen129600, compact740-record460416, escaped489-record493456 FBA bytes. These fit observations are not exhaustive sufficiency proofs.

Checked assembly identity and named function prologues against stack-accounting.json: Debug native AArch64 local frames decode7616, load6848, replace13808, validator87808. These are local emitted frames, not measured worst transitive parser/caller stack. Production source identity is unchanged across the earlier assembly receipt and final qualification; later tests do not establish a new stack maximum.

## Findings and disposition

| Severity | File:line | Description | Suggestion | Status |
|---|---|---|---|---|
| Informational coverage limitation | zig/src/retained_atlas.zig:102; zig/src/retained_text_tests.zig:219 | Actual public ScratchExhausted is unexercised. Correct parser OutOfMemory mapping exists; measured adversaries reach EntryCount, not OOM. | Preserve explicit Partial label. Do not claim unreachable OOM, universal sufficiency or manufactured exhaustion coverage. | Accepted explicit parent ruling; not an actionable defect. |
| Informational evidence boundary | .superpowers/sdd/2026-10-09-nyon-zig-migration/task-3h2a-stack-accounting.json:47 | Local emitted frames do not establish transitive runtime stack usage. | Retain scope statement; qualify future consumer/platform stacks when those become authorized. | Correctly disclosed; not blocking H2a. |

Critical: none. Important: none. Minor: none. Task quality Approved; H2a is spec compliant under the accepted coverage ruling. Parent aggregate acceptance and any H2b dispatch remain separate decisions.
