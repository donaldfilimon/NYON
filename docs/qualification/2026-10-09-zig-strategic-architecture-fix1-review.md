## Design Document Review: Strategic cubic planets — architecture fix1

### Summary

**APPROVED for architecture-section presentation.** Both original nonblocking Minor notes are resolved; the independently reconstructed diff contains only their three expected text regions, with no new regression established. Human architecture approval remains pending; this verdict grants no formal specification/plan approval, implementation authority, or GPU/runtime acceptance.

Reviewed 2026-10-09 in canonical `/Users/donaldfilimon/dev/active/NYON`, shared dirty `main`. Scope is the two corrections only, not a reopened full architecture review. The original raw review remains unchanged with its original open statuses; dispositions below apply to the revised proposal.

### Issue 1: Carry forward the entire LG acceptance ledger explicitly

- **Severity**: minor — original nonblocking clarification
- **Section**: revised proposal §7, line 125
- **Description**: Original review Issue 1 identified that the compact Strategic acceptance table could displace independently accountable LG rows. The revision now states **“Retain all LG-01–LG-27”**, says the table “supplements that ledger,” and requires the formal spec/plan to preserve untouched requirements and independent completion evidence. Direct inspection of `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-qualification.md:18–47` confirms all 27 rows, including LG-22 audio isolation, LG-23 documentation/delivery, LG-25 cross-authority lifecycle, LG-26 wire identity and LG-27 running creator queue. Journeys, experiments, all 46 tools and the full host matrix remain separately retained.
- **Suggestion**: Carry this explicit mapping obligation into the later formal spec and plan; no further architecture correction is required.
- **Status**: resolved

### Issue 2: Reserve the list and recovery path promised on arena exhaustion

- **Severity**: minor — original nonblocking specification follow-through
- **Section**: revised proposal §6, lines 115 and 119
- **Description**: The revised geometry row and essential-capacity paragraph require reservation before optional work across geometry/upload, fact/layout/hit/semantic and GPU capacity. Reserved records cover a bounded list page, Inspector, page/search/navigation, error/recovery, and draw/hit/focus/semantics; **“Optional work cannot borrow these partitions.”** Essential admission precedes activation; failure retains an already-admitted host recovery surface. A known preallocated status/error presentation remains usable without replacement-scene allocation. Optional exhaustion must publish a usable essential page with explicit detail-unavailable status and complete entity reachability through bounded paging/search. Forced exhaustion witnesses cover each optional arena, first activation and rebuilds, and pointer/keyboard/semantic actions. This establishes admission and failure behavior rather than optimistic fallback allocation or larger arenas.
- **Suggestion**: The formal spec must bind exact reserve quantities, record/text/page/detail bounds and matching upload/driver admission, then implementation must provide the required forced-exhaustion witnesses. Those are explicitly future obligations, not missing architecture approval evidence.
- **Status**: resolved

### Strengths

- All numeric ceilings retain Proposed status. The changed row preserves 65,536 triangles, 8 MiB geometry/upload, 8 MiB fact/layout/hit/semantic, three packets, 48 MiB total and the 128-row maximum; the 192 MiB GPU ceiling is unchanged.
- Exact comparison confirms all remaining proposal text is unchanged: separate variable-capacity depth renderer, six faces, CPU picking/depth agreement, complete product scope, persistence/CAS, Strategic shader extension, authority isolation and human architecture/spec/plan gates are preserved.
- Existing source contracts remain honestly distinguished from the proposed runtime. `src/living_navigation.rs:62–79` uses full entity IDs and parent/world links; `src/engine/render_frame.rs:9–25` has scene/UI/underlay/overlay seams; `src/engine/render.rs:185–213` composes those layers; `src/engine/scene_renderer.rs:374–378` still rejects non-seven-world Classic scenes. These source reads do not establish that the proposed Zig essential-capacity mechanism exists.

### Verification and frozen evidence

Read the revised proposal and original raw review in full, inspected the fix report, and independently reconstructed the before/after diff. `diff -u` exited **1**, its normal differences-found result. A separate read-only Python identity/diff check exited **0**: all five frozen artifact hashes below match, exactly three changed line regions exist (replacement at revised line 115 and insertions at lines 118–119 and 124–125), and all remaining proposal lines are identical. The qualification ledger hash also matches, and its row IDs are exactly LG-01 through LG-27. No Critical/Major/Minor/Nit regression is established in the changed text.

| Artifact | SHA256 |
| --- | --- |
| Original raw review | `0aaf0d939f173054b341e46a346485473b0184027b212372dbc691640a3c2d8e` |
| Pre-fix proposal | `74556ab55576845592ea8c88ce1c14332f57693ee4cf2589b08e27cda975d002` |
| Revised proposal | `83a9ca2d51104d4807ea6863856be72112b2501bf8dcc072d4ed134313203590` |
| Fix1 report | `9fe671607d37af9560241e7501c133830c4dff89691c133941af51edc4ab6c52` |
| Original writer report | `31720e70d6e2d1eaf10a0caf91a66f54c72adc03815346c5efda508942cf4fa7` |
| Living qualification ledger | `1c36f4c3d588fc15a655fbd1bacb2a633b9c21f93bc45233ff03d612ef1e7177` |

**Evidence limits:** no builds, tests, repository gate, GPU/shader/runtime, capacity measurement, host recovery execution or accessibility execution were run under this document-only dispatch. External platform behavior was not independently verified. Exact numeric reserve feasibility and successful exhaustion behavior remain untested Proposed obligations. Historical source-inventory drift noted in the original review is neither refreshed nor relabeled by this scoped review. Only this assigned report was written; source, specs, plans, dependencies, assets and other agents' work were left untouched. No commit, push, deployment or publication occurred.

Memory was used only for routing prior Strategic-first context (`MEMORY.md:58–65`); fresh artifact comparison and source reads determine this verdict.
