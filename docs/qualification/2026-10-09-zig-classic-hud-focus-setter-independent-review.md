### Spec Compliance

- ✅ **Spec compliant.** Current frozen setter prerequisite satisfies the execution brief and accepted FIX1 seam. Critical 0, Important 0, Minor 0.
- `zig/src/classic_ui.zig:101`: nonempty → playing mode → unpublished → full optional UTF-8 → exact complete current focus-order membership checks precede the single canonical assignment at line 116. Null is accepted, caller bytes never escape, and no allocation, retain, request, storage, node/text/geometry/order or attachment flag mutation is introduced.
- `zig/src/classic_ui.zig:393`: unchanged publication commits the published flag only after successful H1 replacement (lines 398–400). The setter preserves failed-publication retry and permanently rejects mutation after successful publication, including after frame release. Disabled focus remains valid; activation still rejects at line 408.
- `zig/src/classic_ui_tests.zig:1144`: independent world3 marker, canonical pointer identity and caller poisoning witness; line 1167 verifies actual one Header request of 33,312 bytes, zero calls/frees/ref/ledger/pending/reservation/charge delta, unchanged backing/count/geometry, disabled activation, and exact marker payload.
- `zig/src/classic_ui_tests.zig:1227`: actual first H1 allocation failure preserves old focus backing/content, revision, refs and budget; setter correction and successful retry are exercised, followed by permanent rejection after release. Line 1277 verifies absent marker/hidden source rejection, independent H1 raw-history rejection, null publication without first-control fallback and repeated-attachment rejection.
- `zig/src/classic_ui.zig:916`: private rejection helper compares complete initialized Header bytes, old FrameSlot view and copied focus bytes, ledger, pending bytes and reservations. Line 938 exercises all five error kinds, mode/publication precedence, malformed full UTF-8, prefix/suffix/NUL/missing identities, marker mismatch, repeated attachment and frame-clear rejection. Existing recursive field/padding initialization at line 745 makes whole-byte comparisons defined.
- `zig/README.md:721`: added contract documents the narrow producer prerequisite and leaves router/host integration pending. Root is unchanged; its existing test imports remain in the frozen package.
- ⚠️ **Out of scope / unverified:** source-history capture, Session restoration, Settings/editor/guide domain conversion and host router behavior are wider I3 obligations. This setter changes no Session history. Native/browser/accessibility/DPR/player acceptance, aggregate repository qualification and transitive stack bounds do not follow from this task. Parent aggregate gate remains required; no suite or gate was rerun by this reviewer.

### Strengths

- `zig/src/classic_ui.zig:105`: a local optional canonical ID gives a small auditable transaction; all failure returns precede mutation. No new production unwrap, clone, lock, allocator or dependency is introduced. New private-test optional unwraps at lines 917 and 932 are fixture-invariant assertions, not unchecked production input.
- `zig/src/classic_ui_tests.zig:1250`: the retry fixture induces a real allocator failure and verifies it occurred rather than manufacturing a publication error. Tests use actual preparation, attachment, H1 publication and action validation.
- `task-3i3-hud-focus-setter-evidence/red-behavior/source/zig/src/classic_ui_tests.zig:1157`: the archived absent-method scaffold performs no product mutation; the actual appended marker remains unfocused and the nonnull behavior assertion fails. RED exit 1 is genuine missing behavior. The final patch removes that scaffold.

### Issues

#### Critical (Must Fix)

None. Status: closed for this scoped review.

#### Important (Should Fix)

None. Status: closed for this scoped review.

#### Minor (Nice to Have)

None. Status: closed for this scoped review.

### Evidence Checks

- Current: patch SHA256 `3fd811913f0d4efae8df6f66e47468c919e9a893ae2ef4b2d306d567fd87538c`; implementer report `b1cd97961889917f193f5cf8b809f50b353775d9cb8e1082c97e5001791095a4`; artifact manifest `4b6b9111cc257d242b8d4a123981a487c0416a44ecf2ba6cd18a315dca7e90ef`; final-source metadata `32c0f46027748c4ee875a96388577da5204121262070a5ee517a758ed251ce98` all freshly match supplied identities.
- Current: all **1,179** manifest entries match actual bytes/SHA256; all **75** protected identities match current files. Current and final-copy hashes match UI `c45dc5c5bfd1275170e5af9ca00064fdeac2beb74e53575b557ef168ea5bf32c`, tests `d194588c0fbcc493c5e6050802ed5f6109617cdf4c303d1c5f38724387fe0fb2`, unchanged root `7e0fb30484cd3f910a9d24bd88195bc11b16b05381e63dc73998fc981c6245d6`, README `45085e39d0f7ca88701c93e0426a7652c0969683a73add54f25e41b981b0670c`.
- Current: accepted durable FIX1 and scoped review hashes match `4f18a59d9c8a447508114e0841a6938efc611b67d396e24ccc0caeb26fd7fd4d` and `7fe8c6178c010ce9282f1b3bfe6480ee06757b114bf46287c5692df839c74380`.
- Verified archived runs: reviewed-Debug, reviewed-ReleaseSafe and reviewed-ReleaseFast full outputs each terminate with **All 57 tests passed**, actual compiler terminal exit **0**; five newly added setter tests appear in each. reviewed-fmt has empty output and terminal exit **0**. Full output contains requested measurement diagnostics, no warning or unexpected diagnostic. These are existing focused receipts, not independently rerun checks.
- Current evidence binding: all 75 archived precommand source hashes in each of those four final receipts match their copies; all four owned final-source hashes match final metadata. Precommand and receipt shared fields agree, with exact argv/cwd/times/cache environment. Current pinned compiler hash matches `7a6a4ee02ae6277258a699503d2396fbfbc86704b6216cd4e52c7cc502334bfb`; archived version is `0.18.0-dev.120+9fe22a29b`.
- Verified RED receipts: API-absence RED exit **1** and marker-focus behavior RED exit **1**, with actual TestUnexpectedResult output. Earlier final-Debug fixture failure remains archived; it is not substituted for final reviewed evidence.
- Review method: used the supplied baseline-to-final full-context package, not Git HEAD diff. Initial tool rendering truncated the large patch; recovered its additions and required surrounding publication/initialization context from that same package. No changed source file was separately reread. Inspected only archived RED source to verify the named risk of manufactured test failure. No new compiler/test/fmt/gate/index/HEAD/agent/commit mutation occurred. Read-only identity verification exited **0**; one initial verification script exited **1** due to assuming object-valued receipt hashes, then the corrected string-valued check exited **0**.

### Assessment

**Task quality: Approved.**

**Reasoning:** The seam implements the requested atomic canonical focus conversion without widening producer ownership or resource policy. Independent fixtures and source-bound final receipts substantiate the setter prerequisite; router/host and aggregate acceptance remain pending.
