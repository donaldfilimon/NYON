## Design Document Review: Full-scale authority / durable-session Task 0 contract

### Summary

**Needs revision — Task 0 compatibility is NOT PASS.** The initial codec, transport identities and capacity arithmetic agree, but four Important (major) interface gaps still prevent independent authority and session writers from implementing one consistent publication/lifetime contract. This review grants no execution approval and changes no specification.

Scope: the joint Task 0 interface only; not a repeat review of the complete session feature, city mechanics, storage adapters or authority algorithms. Review date: 2026-10-11. Existing raw reviews remain unchanged.

### Exact revisions and applicability

All hashes below were observed at the beginning and rechecked immediately before and after writing; they were unchanged (final hash/readback validation exit 0). Line citations refer to these bytes. A subsequent change requires a new hash comparison and affected-finding review; earlier PASS records do not approve newer bytes.

| Document | SHA-256 |
| --- | --- |
| Authority design (`A`): `docs/superpowers/specs/2026-10-11-nyon-zig-fullscale-authority-design.md` | `a8bc0d41767722b3215e2e6c334d06d5cf5cacc3107f28f83b3b4219ba6ce2f5` |
| Initial codec (`C`): `docs/superpowers/specs/2026-10-11-nyon-zig-fullscale-authority-codec.md` | `c116655859eae999ee1529180521254a575570ba471099fc0ca2e00c8778b6b9` |
| Authority plan (`P`): `docs/superpowers/plans/2026-10-11-nyon-zig-fullscale-authority.md` | `0125c407f3500c221993df1345ab436cad729a7a726410463ba6e81c5a7aa5a6` |
| Proposed session design (`S`): `docs/superpowers/specs/2026-10-11-nyon-zig-expanded-durable-sessions-design.md` | `ce8f58f9a4427a741b1fa72586975cee534ce18977669370af35edc78970b4f5` |
| Seeded design referenced by C | `5577e8886f80a48df656fa1eb53ff3421a74c9deaecb5d2e7307001f43f4320d` |

`P:237-270` already makes joint acceptance a hard prerequisite. Its `d448fa...` session observation and `A:569-590`'s `0014af...` M1 observation are historical snapshots, not the current session contract. Updating those references alone cannot close the issues below. The seeded hash matches the codec dependency; this does not independently qualify the seeded implementation.

### Issue 1: Joint publication has no common executable ownership contract

- **Severity**: major (Important; blocks Task 0)
- **Section**: A “One global transaction,” lines 310-324; S “Stream reservations and publication,” lines 636-660; P Task 0 and shared interface contract.
- **Description**: A says “One work unit switches the committed descriptor index” and permits no candidate queries. S adds `SessionAppendReservation`, `PreparedTranscriptCursor` and `GraphPrepared`, then says one step “switches the authority descriptor and transfers the prepared graph descriptor into the resident session.” A and P have no corresponding shared declarations or integration owner. S's acknowledgment lists ticket, parent, length, logical digest and Prefix but does not fix a complete session/branch/graph generation binding, consumption state, or the function that owns both switches. “The authority checks that acknowledgment against its candidate” is insufficient for the session logical digest and branch parent: those are session identities, absent from the authority codec. Independent implementations could validate different halves, accept a prepared graph after an intervening fork/replacement, or perform a fallible graph operation after authority publication. Current code does not supply the missing bridge: `zig/src/coordinated_galaxy/view.zig:98-108` is explicitly only an infallible descriptor primitive, while `zig/src/expanded_session/types.zig:18-63` models catalog values rather than a joint resident transaction.
- **Suggestion**: **Authority owner** amends the authority's barrier/publication contract and pure value declarations. **Session owner** defines the resident binding and preparation result. **Controller/integration owner** owns one non-reentrant operation with access to both owners; it must validate and seal both sides before an allocation-free, hash-free, callback-free, non-yielding `void` publication primitive consumes the sealed pair. Include session owner/generation, resident graph generation, selected branch/cursor and parent digest; authority owner/generation, job and candidate descriptor generation; session reservation slot/generation; exact payload length and Prefix. Session validates its logical digest/parent and authority validates its own prepared bytes/Prefix/length. Explicitly invalidate readiness on relevant local mutation and specify duplicate/stale acknowledgment outcomes. All counter checks and fallible generation comparisons occur before either visible write. A catalog CAS conflict is a separate durable-save outcome: it leaves an already committed resident tick dirty and cannot roll back authority publication. Freeze this division in P's controller addendum and name the bridge writer; do not make the authority depend on the platform store.
- **Status**: open

### Issue 2: Stream cursor coverage and lease transfer remain underspecified

- **Severity**: major (Important; blocks Task 0)
- **Section**: A lines 318-324, 353-374, 462-485; S lines 615-650 and 662-676.
- **Description**: S's candidate cursor yields the new tick bytes, permits repeated passes and allows “re-reading the old resident partial tail as needed.” The physical first changed segment can also require up to 1 MiB of bytes *before* the new tick. The two documents do not freeze an operation for that committed byte range, the initial genesis stream, or the ownership transition from immutable candidate cursor to the unsaved descriptor's committed history-window lease. A exposes committed record/value queries and a provider lane, but its only candidate access rule is “No query reads a candidate or internal cursor.” An implementer cannot infer whether an unsaved descriptor pins only history slots or also the old gameplay descriptor (which blocks retirement and next begin), when a cursor becomes stale, or how an aborted prepared graph releases its insertion reserve before another append. These choices determine whether the promised two-descriptor queue and seven-block reservation actually work without a complete extra tick copy.
- **Suggestion**: **Authority owner** specifies a privileged bridge capability distinct from public views: candidate-byte cursor with exact range, frozen candidate identity, restart/offset semantics, bounded copied result, and invalidation on cancel/publication; a metered committed-history window reader supplies the old partial tail and genesis stream. Specify whether history-window leases pin gameplay descriptors and the resulting backpressure, rather than implying two buffered ticks are always available. **Session owner** supplies an explicit ownership table covering reservation → graph preparation → joint publication → durable-prefix transfer → retirement, plus abort at every stage. The table must name owners of index pages, manifest, hash/path scratch, candidate/history leases and physical batches; postpublication work can regenerate segment bytes from retained windows, not retain every changed leaf payload. Cancellation closes cursor admission first, then incrementally releases only candidate graph/index/lease ownership; a completed durable batch remains broker-owned until reconciliation. No delayed cursor or save capture may read reused cache slots. Include all new controls/cursors in Task 1's concrete layout witness and the existing store partition.
- **Status**: open

### Issue 3: Replay publication needs an explicit alternative to live append admission

- **Severity**: major (Important; blocks Task 0)
- **Section**: A lines 551-567; S lines 601-660 and 670-672; P lines 737-739 and Task 13.
- **Description**: Both specifications require replay output to be compared with authenticated source bytes and “not archived again.” The added joint bridge, however, reserves a new logical node, insertion pages, a manifest and an unsaved descriptor before authority begin, then installs that prepared graph at publication. No declaration says how a detached replay target advances through existing ticks without invoking that live append path. P also says the standalone fake host must not bypass required publication preparation. Applying the live path to replay can duplicate nodes/output and consume the resident graph lane; skipping it ad hoc leaves a second, unreviewed publication protocol. Genesis installation has the same need for an explicit distinction from an ordinary append.
- **Suggestion**: **Both owners** freeze publication purpose as an explicit contract, for example live append, replay verification and genesis installation. Live append consumes the reservation in Issues 1–2. Replay verifies against an already retained expected logical boundary/Prefix and source token; its sealed result advances only the detached target and verified byte range, creates no new session node or output archive, and cannot mutate the resident graph. Initial installation binds genesis and its initial stream without spending a tick-node reservation. A successful target take moves target ownership and its source pin together; a later fork obtains its own exact-prefix token before live append. State how the fake host implements each mode under the same checks. Add fixtures for replay at a full graph cap, cancellation after target publication, target ending inside a source segment, and stale final installation; none may create a third authority or clear resident dirty state.
- **Status**: open

### Issue 4: Pending pin requests are not assigned an exact shutdown/disposition grammar

- **Severity**: major (Important; blocks Task 0)
- **Section**: A “Bounded provider and retention protocol,” lines 506-549; S “Exact-prefix acknowledgment and provider ownership,” lines 544-611; P shared types and Task 8.
- **Description**: Prefix/Ticket/token fields agree and both documents correctly separate cancel from physical disposal. However, `takeShutdownDispositions(out[4])` describes ownership tokens for two descriptors, one replay source and one active request without a tagged representation for a descriptor whose `requestPin` is still pending. `completePin` can arrive after cancellation/shutdown, before there was any authority-owned RetentionToken to transfer. The broker-side obligation to reject stale completions is stated, but the shared declaration does not fix who owns and closes the pending pin request, how it occupies the four-item allowance, or how a newly durable stale pin is released exactly once. The same ambiguity applies to the currently active source request versus its retained pin reference and the bounded abandoned set. This is an interface gap, not evidence of an implemented leak.
- **Suggestion**: **Authority owner** freezes tagged disposition/request state values, including pending pin Ticket+Prefix, owned token, and pending source Ticket+pin reference, with exact ownership on `Busy` versus accepted transfer. **Session broker owner** freezes a bounded ledger that retains every started request through physical disposal, accepts the reserved four-item shutdown batch, and disposes a late successful pin even when the authority namespace is already closed. Explicitly account for the already-abandoned set and prohibit admission that exceeds the fixed slots. Specify provider ingress stages (header, 32 sibling hashes, body, verification/consumption), exact-length/end checks, duplicate/late response handling, and whether token.ticket identifies the pin-acquisition request while the separate source Ticket identifies the current read. Keep copied tickets and bounded buffers only; no authority pointer in completions. Required joint fixtures: shutdown while `requestPin` is pending, then late success/failure/duplicate; cancel while a source buffer is in flight; broker unavailable at transfer; and namespace destruction before completion. Check pin/reference counts and disposal separately from public cancellation outcomes.
- **Status**: open

### Required declaration and transition acceptance

This is a checklist for the owners' amendments, not replacement specification text or permission to implement inferred interfaces.

| Shared item | Required frozen fields / behavior |
| --- | --- |
| `Prefix`, `Ticket`, `RetentionToken` | Retain the existing exact 112-byte Prefix grammar and hash domain; Ticket's four checked u64 values; token's Ticket, Prefix identity and independent host pin ID. Distinguish authority request IDs from persistent store operation IDs. |
| `SessionAppendReservation` | Include the authority job binding, session/resident graph and branch binding, session slot/generation, expected parent and maximum length; explicitly owned, never a borrowed graph pointer. Maximum must cover the registered bound before begin. |
| `PreparedTranscriptCursor` / committed window | Bind authority/candidate or committed generation, byte range, pass/offset and lease ownership. At most 256 copied bytes per charged copy; no zero-budget advancement, exposed slice or hidden complete transcript. Define close/restart/stale behavior. |
| `GraphPrepared` | Echo the exact reservation and candidate binding, parent, actual length, computed session logical digest and authority Prefix; identify the prepared graph ownership being consumed. Only the correct owner validates each identity. |
| Sealed joint publication | A final preparation transition consumes/checks all readiness and counter constraints. Either disallow intervening relevant mutations or revoke the sealed result before publication. One charged, non-reentrant commit transfers both prepared owners without a yield or fallible work. |
| Replay/genesis mode | Explicit readiness proof for detached verified publication or initial installation; no duplicate tick-node admission or archive output. |
| Cancel / release / shutdown | Exactly-once state transitions for reservation, candidate cursor, graph pages, windows, pins and physical requests; pending requests remain owned after authority destruction until the broker reports actual disposal. |

Suggested transition witness: reserve → authority prepare → frozen byte cursors → session graph prepared → sealed pair → one publication → durable-prefix transfer → retirement. Interrupt before/after each transition and every metered copy/hash/release; vary budgets 0/1/7/256/1,024. Include parent/branch replacement, descriptor/reservation generation mismatch, duplicate readiness, save CAS conflict, delayed provider completion and exhausted counters. A failed prepublication check preserves both committed owners; after publication cancellation reports already-published and cleanup touches only obsolete/capture ownership.

### Verified compatibility and arithmetic

**Current static evidence:** both full designs and the initial codec were read, Task 0 and consuming plan interfaces inspected, and current coordinated view/state and expanded-session catalog types read. Focused source search found no `SessionAppendReservation`, `PreparedTranscriptCursor`, `GraphPrepared`, `requestPin` or `takeShutdownDispositions` in the current `zig/src/expanded_session/` and `zig/src/coordinated_galaxy/` trees (`rg` exit 1 means no matches, not a test failure). Existing coordinated history is `[4096][2048]u8` with u16 counts (`zig/src/coordinated_galaxy/state.zig:9-25`); its value reader is committed-only (`view.zig:183-198`). No existing bridge implementation establishes the missing semantics.

An independent Python standard-library arithmetic check reconstructed the portable fixed overhead from field widths, summed the physical/memory rows and asserted the following values; actual process exit **0**:

| Check | Result |
| --- | ---: |
| Initial conservative framed tick | 4,057,265 bytes |
| Portable fixed overhead, including two maximum genesis fields and 64 maximum branches | 2,110,350 bytes |
| `C(100000) = 2110350 + 100000*(33554432+76)` | 3,355,452,910,350 bytes |
| `C(131072)` | 4,398,058,582,926 bytes; below 5 TiB |
| Per-node framed physical envelope | 44,734,124 bytes |
| `P(100000) = 8388608 + 100000*44734124` | 4,473,420,788,608 bytes |
| `P(131072)` | 5,863,399,489,536 bytes; below 8 TiB |
| Assigned store/transfer rows | 109,819,824 bytes |
| Remaining space under 128 MiB | 24,397,904 bytes |
| Source + one target + full 128 MiB store allowance | 511,836,160 bytes (488.125 MiB) |

The 33-leaf bound is conservative for a 32 MiB append straddling an old partial tail, and 33×32 bounds changed transport paths. The one-object-per-batch 115-byte inventory overhead is conservative under the stated prohibition on empty/duplicate-only batches. Portable 5 TiB and namespace 32 TiB ceilings are representable with checked u64 arithmetic; they do not imply available host quota. Initial codec and 32 MiB extension limits remain distinct. The 131,072-node graph cap covers 100,000 ticks but not the authority's entire 1,000,000-tick horizon, which S explicitly disclaims.

**Proposed, not proven:** concrete bridge/control/graph layouts, allocation lifetimes and every foreign-runtime copy still need the separately admitted Task 1 witness and session allocator tests. The numerical sum cannot prove those layouts fit or prove absence of an extra complete tick copy. A safe decomposition is possible with candidate bytes in authority's 32 MiB block, committed bytes in its 64 MiB cache, authenticated input in its existing scratch, and session segment/batch/index work in S's listed arenas; Issues 1–4 must make that ownership concrete. Do not reinterpret the 24,397,904-byte margin as permission for unknown adapter copies.

**Unverified / out of scope:** no product, compiler-layout, native/browser, crash, provider-runtime, performance or 100,000-tick endurance test ran. Per assignment, no build/gate, dependency change, source edit, other-document edit, ledger update, commit or publication occurred. Native filesystem and IndexedDB behavior remain adapter qualification obligations; this review provides no remote or host durability evidence. The repository gate is deliberately unrun for this document-only review.

### Strengths

- Session's revised 32 MiB streamed tick contract fits the authority envelope while retaining the tighter initial codec bound and distinct G/E/transport/object identities.
- Exact-prefix authentication uses a locally trusted root, immutable partial tails and independent durable pin ownership; a segment write alone does not permit eviction.
- Descriptor-only unsaved queues and explicit arena accounting avoid prescribing another complete tick allocation.
- Storage arithmetic charges old partial tails, path nodes, index insertion pages and inventories; host quota and real endurance qualification are explicitly separate from representability.
- Both designs preserve old data/identities, reject silent history truncation, share the remaining work budget and require postpublication cancellation to remain truthful.

### Task 0 disposition

**Blocked for compatibility acceptance by Issues 1–4.** The controller should obtain the authority-owner and session-owner amendments, pin the resulting exact hashes in the plan addendum, and obtain fresh independent joint review. Only after those four interfaces agree can Task 0 receive PASS; the separate Task 1 layout gate and later runtime/host acceptance remain mandatory.
