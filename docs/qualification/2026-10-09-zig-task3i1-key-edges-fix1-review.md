# Task3I1 key edges FIX1 scoped independent review

2026-10-09. Verdict: M1 ADDRESSED. New Critical 0, Important 0, Minor 0. This review covers only the pending-clear fixture correction and breakage attributable to it. The original full review remains immutable. Parent aggregate qualification remains Partial; no router, editor, host or whole-game acceptance follows.

## Frozen evidence

- FIX1 report SHA256: 98717977fc8da6c7a81f616042064745ddc1383d64ee16b263a69a255a5c1b03.
- FIX1 test SHA256: 3f980eb08b94c3a11c0bf0a23ca1a5efcc64ddc6db0e39c89f7bc3b0306c98e5, 36320 bytes.
- Production remains 9e02aaff2b09abbcacf5f0fa1912c4e25ca3803e7d9043ae0eb83fe8c6c91a45, 17943 bytes.
- Original full review remains 8b3f6c07c593efb3bbe970235ea242adbc1cad34b4c448286ab972c2918a43f7.
- Original artifact manifest remains a26ea3715dae49b130056854745a986e5d5e6c2656074d6efa34f9e4683b454a.
- FIX1 artifact manifest: 46fc7a35a960f09323bcecf88ad1a33d663764afcc0a17026099fa370c2a866f.

Read the full correction report, entire additive test/diff, archival runner and actual focused receipts and outputs. Independently verified all 391 FIX1 artifacts and all 684 original artifacts against hashes and byte lengths, with no mismatches. All 73 protected live identities are unchanged. FIX1 baseline copies match its baseline receipt and original final identities. Final copies match current source and final-source receipt. Production/root/README are exactly unchanged; paired tests preserve the complete original 34796-byte helper prefix and append 1524 bytes. The accepted 12833/25516 foundation prefixes remain exact. Independently regenerated full-context paired-test patch equals the archived patch; other three patches are empty because those files did not change.

## M1 correction and new-breakage assessment

Location: zig/src/classic_input_tests.zig:644–671. Each inline method iteration creates a fresh InputState, presses all 15 actions, and directly asserts every held bit and every pending bit true before reset. Direct pending reads do not consume an edge. Exactly one selected method is then called. Every action must have no pending edge afterward. clearTransient and finishFrame must preserve held, whereas clearAll must clear held. Since each method receives a separate fresh setup, one successful method cannot mask a later ineffective method.

An independently ineffective pending clear therefore fails immediately on the first postreset take; a transient/frame implementation that incorrectly clears held fails the held assertion; an all-clear implementation that only clears held fails the pending assertion. This reasoning establishes that the fixture detects the original no-op corner without deliberately modifying production or fabricating a new executable RED.

Following reset, duplicate down for transient/frame must not produce an edge until release/repress; all-clear must allow a new down edge. Every action then checks held remains true and the edge consumes only once. The fixture uses existing methods and a nonmutating inspection of public pending state, with no fake repair or new production API. Local state dies at test end; no allocator, token, queue, router or external lifetime is involved. No correction-induced breakage was found.

The report explicitly acknowledges the original fixture defect and limits its new claim to additive regression qualification, preserving original evidence. M1 is addressed in both executable coverage and provenance.

## Focused archival receipts

All receipts use canonical NYON/zig cwd, resolved pinned executable /Users/donaldfilimon/.zvm/master/zig, SHA256 7a6a4ee02ae6277258a699503d2396fbfbc86704b6216cd4e52c7cc502334bfb, version 0.18.0-dev.120+9fe22a29b with version exit 0, isolated caches and UTC start/finish. Runner copies actual sources before commands. Debug, ReleaseSafe, ReleaseFast and fmtcheck production/test/root prerun snapshots match final identities. Format's prerun test differs only in @intFromEnum versus formatter-normalized @backingInt; subsequent final tests and fmtcheck use the normalized final source. No false claim that the preformat copy equals final is made.

Format 23:43:09.156355–23:43:09.214824 UTC exits 0. Debug 23:43:09.359588–23:43:12.306208, ReleaseSafe 23:43:12.483100–23:43:19.893706 and ReleaseFast 23:43:20.061797–23:43:28.815711 each exit 0 with all 31 tests passed and the named FIX1 fixture explicitly OK. These are nine helper plus 22 inherited/imported tests per mode, not 93 distinct tests. fmtcheck 23:43:28.991790–23:43:29.042153 exits 0 and checks both helper files.

No compiler, formatter, test, aggregate gate, product edit, subagent or Git mutation was performed by this reviewer. Only this new ignored review was written. Parent remains responsible for fresh aggregate qualification; all earlier caller-lifetime, I3, editor, host and manual proof gaps remain unchanged.
