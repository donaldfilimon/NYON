# H3b retained editor independent implementation review

Status: PASS for the frozen pure retained editor slice. No blocking or nonblocking correctness findings identified. This is source and captured-evidence review, not aggregate gate or host acceptance.

## Frozen identity and independent audit

Reviewed the complete 43,872-byte module, 60,330-byte tests, root and README baseline differences, worker report, retained UI dispatch and ownership bindings, and accepted H1/layout/editor/transaction/session context. Independently recomputed all four live and captured final-source byte lengths and hashes, all 65 protected live identities, and all 198 artifact-manifest entries: zero mismatches. Manifest SHA256 is `e8d2d2b347fca2baed52101040820e517fa3ff36e193b8f7c861ffde9e7c3fac`.

Module SHA256: `829f33d96d78e30e27d34fcce64e59e08096ea2b9aa0dbdd1a4bb43980cf794b`.
Tests SHA256: `5b64f54700d63c6336347d3a2c5d4796230e1cf731655ec8507bbb689e2d3b12`.
Root SHA256: `6e718d935ded85c02222d03ac81a62f7e95026ae4ed9c8f979bccdb33cb8f61a`.
README SHA256: `e7f664d2f449e39aaf9291dab07d41f4f950ea558e1d789e2fcf897b0fbed744`.

Captured final Debug, ReleaseSafe, and ReleaseFast receipts each report terminal exit 0 and 38 passing tests; final fmt check reports exit 0. Their captured source identities match final-source.json. These are worker-run receipts independently inspected, not compiler or tests rerun by this reviewer. Earlier failed snapshots remain historical evidence and were not adopted as final sources.

## Spec compliance and correctness

- `classic_editor_ui.zig:248` validates owner/context, source and report borrowing, full report, selected world before narrowing, exact nonmodal source focus, closed modal focus and premodal history before layout writes. `:697` guards source object and backing ranges, including editor buffers, preedit, current by-value DraftView world-name slices, diagnostic objects and raw lanes. The initialized Workspace alias regression exercises real overlapping storage rather than undefined uninitialized bytes.
- `:176` and `:196` count full unique arena and H1 copied text separately, reject excess without truncation, and validate UTF-8. Joined diagnostic component slices refer into owned arena bytes; raw status, action, recovery, field and IME lanes remain complete. Ordered validation quartet construction retains actual fields, severity, code and message semantics. Static catalog and nonaction IDs are closed and bounded; no user text becomes an ID.
- `:248` performs scalar preflight, reserves and charges one fixed Header before the allocator request, cancels on allocation failure, replays under the explicit immutable source lease, and unwinds acquired token slots independently. Workspace LayoutView does not escape. Header accessors borrow Header arrays/context, geometry is copied scalar data, and successful H1 publication owns its own complete frame. Same-token and distinct-token reference overflow cases unwind prior retains.
- `:310` preserves historical restore independently of base membership; `:322` binds focused only when enabled/focusable membership exists and preserves null. `:765` focus ID accounting matches actual H1 copied focus handling. In particular, accepted layout emits both modal descriptors enabled/focusable even at tiny invisible bounds (`classic_editor_layout.zig:144`); both are appended and counted, so the modal restore deduplication assumption remains valid. Underlay capabilities remain unavailable during modal confirmation; source IME uses remembered editor focus rather than invented overlay focus.
- `:337` rejects stale complete owner/revision and budget mismatch before replacement, replaces the frame atomically through H1, then updates the published revision. `:647` resolves only catalog capabilities present and enabled in the trusted published frame, checks revision/owner and modal category, rejects read-only fields, and performs no mutation. The session wrapper remains a synchronous pure adapter.
- Root changes are exactly one public module export and one test import. README accurately separates pure preparation/publication from host routing, persistent modal history and runtime acceptance. No protected product edits or unrelated root behavior were introduced.

## Test quality and limits

Fixtures provide literal IDs, roles, DFS order, focus orders, source strings and diagnostic categories rather than checking only self-derived adjacency. They cover all 26 actual validation code families, all 90 typed fields, read-only identity, source world 7 rejection, full Unicode text, orphan/attached IME, empty and malformed inputs, move/deinit lifetimes, stale publication and targets, independent recovery lanes, complete source preservation, and charged allocation failures.

Exact U/T boundary fixtures use an observed baseline to size the final variable lane, then assert exact limits and the extra-byte rejection. This is suitable boundary evidence, supplemented by independent literal component/multiplicity assertions; it does not independently establish every possible corpus byte count. The 455-node fixture deliberately crafts an admitted 98-issue report; actual validator attainability remains unknown as the worker report states.

The allocation observer and transition sweeps inspect actual allocation requests, reservation/charge ordering, every observed OOM ordinal, old frame pointers/content/revision, retained capsule/preedit and retries. P and P-1 are measured after installing the old frame. Test-only token ABI access first checks actual allocation size/alignment and fields before changing the reference counter; it is not a production ABI dependency. Reported Header/metadata and static inventories are actual target observations; linker pooling, transitive stack and whole host resource totals remain unknown.

No compiler, tests, aggregate gate, rendered UI, host event loop, H2, persistent focus coordinator or whole-game runtime was executed or approved by this review. Parent aggregate qualification remains required. This report owns no product changes.
