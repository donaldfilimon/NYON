# Task3I1 normalized key/command-edge preparation review

2026-10-09. **Specification compliance: PASS for preparation. Quality: PASS for preparation. Critical 0, Important 0, Minor 0.** This is the proposed second chunk after independently accepted foundation implementation. It is not product dispatch, implementation, runtime or host acceptance.

## Reviewed evidence

Read the entire binding and report, accepted routing intake, corrected brief and FIX1 review. Independently read full Rust `src/engine/input.rs`, Classic keyboard/router precedence, Settings navigation, command/navigation maps, guide-open input reset, suspend/focus-loss resets, key/IME delivery branches, Rust editor text/IME/focus behavior, and stable Zig command/Session/editor methods needed for comparison. No transient guide source, root or README was read. No compiler, test, gate, product/Git/dependency/asset change or subagent occurred. Only this new review was written.

Verified supplied SHA256 identities:

- Binding: `817e44c2c8dfad328ed0a03c2a4a6dbc48fe3ff07568e933d4833cf0878d9f35`.
- Actual report filename `task-3i1-key-edges-dispatch-binding-report.md`: `fa86b302118df5bd6f22b10a1d1a9f032ed8b3f563792b65721ddf9d1fb86f4c`.
- All ten frozen source/document table entries match current bytes. Verification script exited **0**. `zig/src/classic_input.zig` and `classic_input_tests.zig` remain absent; absence was checked without reading transient implementation.

An initial report lookup used the shortened `task-3i1-key-edges-dispatch-report.md`, which does not exist; focused retrieval located the actual frozen report above. This lookup is not presented as successful evidence.

## Findings

No concrete defect or unresolved policy conflict found in this preparation. No source correction requested.

## Specification and quality assessment

| Status | File:line | Assessment / suggestion |
| --- | --- | --- |
| Verified source agreement | `task-3i1-key-edges-dispatch-binding.md:64`; `src/app.rs:2520` | Whole logical-character ASCII lowercasing matches exact 1/2/3/q/e/p/brackets/c/s/r/h rows. Named Space/Escape/F4 alone supply launch/cancel/scenario; Enter/F1 and multi-scalar or Unicode lookalikes remain unmapped. Exact comparison can implement this without allocating or truncating the string. Preserve these literal fixtures during implementation. |
| Verified source agreement | `task-3i1-key-edges-dispatch-binding.md:66`; `src/app.rs:2636` | Navigation is named-only; Shift alone changes Tab direction. Arrows, pages, Activate, Escape and edit-navigation rows are correct. Modifiers do not veto Classic command mappings; mode acceptance remains the later router's responsibility. |
| Verified source agreement | `task-3i1-key-edges-dispatch-binding.md:68`; `src/engine/input.rs:55` | Down creates a pending edge only on false-to-true held transition. Release clears held only, pending edges coalesce, take consumes once, transient/frame clear preserves held, all-clear resets both. Action-level alias behavior deliberately matches the source sets rather than introducing physical-key counts. Two fixed 15-bit boolean sets with exhaustive enum indexing are sufficient. |
| Verified source agreement | `task-3i1-key-edges-dispatch-binding.md:70`; `src/app/input_router.rs:677` | Ordinary mapped gameplay events have no blanket nonrepeat gate. A first repeat after all-clear may create an edge; held repeats cannot. F1 and special Playing H instead use explicit nonrepeat guards. Navigation repeats are not accidentally globally suppressed. |
| Verified source agreement | `task-3i1-key-edges-dispatch-binding.md:76`; `src/app/input_router.rs:524` | F1 toggles first, guide consumes remaining events, then Classic editor and Settings precede Playing handling. Playing Tab/focused Enter/Space precede command edges. Repeated H falls through to ordinary Help action; absent focus makes Enter no-op and named Space launch. Earlier consumed branches intentionally do not mutate action-edge state. |
| Verified source agreement | `task-3i1-key-edges-dispatch-binding.md:77`; `src/app/input_router.rs:625` | Present key text without any Unicode Cc scalar and accepted text focus inserts before navigation, including empty text and without a modifier/repeat typing veto. Cc ranges U+0000–001F/U+007F–009F reproduce Rust char::is_control; combining and format characters are not control scalars. Full UTF-8 validation must precede scalar iteration. |
| Verified source agreement | `task-3i1-key-edges-dispatch-binding.md:82`; `zig/src/classic_commands.zig:147`; `zig/src/classic_session.zig:366` | Keyboard launch uses hovered_world; contextual launch is a distinct command path. CommandAction only has launch/increase/decrease. The proposed Action enum does not invent a Session all-action signature or conflate UI activation with keyboard launch. |
| Verified source agreement | `task-3i1-key-edges-dispatch-binding.md:84`; `src/app.rs:2243`; `src/editor.rs:245`; `zig/src/classic_editor.zig:167` | IME preedit and commit are separate carriers and later direct editor calls. Commit does not inherit the key-text control test or invented duplicate suppression. Modal refusal/editor allocation failure belong the existing editor. Normalized input validation is explicit even where an editor's inert modal/unfocused path would return before inspecting a payload. |
| Verified bounded ownership proposal | `task-3i1-key-edges-dispatch-binding.md:56`; `task-3i1-key-edges-dispatch-binding.md:58` | Carriers borrow complete strings synchronously, retain nothing and impose no semantic-queue-derived text/IME cap. InputState stores only bits. No allocator/token/queue/platform/owner coupling appears in helper contracts; later storing consumers must separately copy/admit bytes. Temporary poisoning fixtures appropriately prove no retained slice in the edge state. |

The independent jury covers every action/navigation row, full UTF-8 validation, case/Unicode/multiscalar distinctions, release-before-take/coalescing/alias edges, distinct-action independence, clear semantics and text/control corners. Synthetic router precedence and actual editor/host witnesses are explicitly separated from this helper chunk's acceptance. This prevents a mapping-only test from becoming a claim of live key or IME delivery.

## Later ruling and verifiable gaps

The frozen binding's line **95** and report describe queue8×4096 as Proposed. Parent subsequently supplied its accepted numeric ruling: eight slots up to4096,32768 slot storage plus measured Header, explicit EventLimit/QueueFull, and no raw IME cap; host delivery remains pending. Treat the frozen wording as historical, not an unresolved conflict in the key helpers or an instruction to reopen the numeric ruling. Key/IME borrowed slices remain independent of semantic queue admission under both versions. No first-chunk implementation exists yet, so no actual API/source compatibility was assumed.

**Proposed / unverified:** Concrete Zig helper implementation, enum/layout assertions, malformed UTF-8 behavior, actual source-buffer preservation, executable behavioral RED/GREEN, focused three-mode/fmt results and parent aggregate gate. No unrun check passed.

**Dispatch prerequisite:** Freeze the accepted foundation source/API and ensure sequential ownership before implementing the second chunk. No new owner/queue API should be inferred from these carriers.

**Later I3 / host obligations:** Real guide/editor/Settings/Playing routing and caller reset boundaries; typed Session effects and hovered-versus-contextual launch behavior; current retained focus/action capability admission; editor IME commit/confirmation/OOM witnesses; host synthetic filtering, normalization/modifier snapshots, physical/logical conversion, IME ranges/protocol and keyboard/IME duplicate delivery behavior. Native/browser/accessibility/manual acceptance and queue host delivery/drop/backpressure disposition remain unqualified.

## Verdict

Prepared key normalization and action-edge contracts are source-consistent and ready to be used after the prerequisite foundation acceptance. No blocking or nonblocking finding was identified. No product implementation or host delivery is accepted by this review.
