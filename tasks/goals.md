## Ship verified repository guidance

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Canceled on 2026-10-19:** Donald canceled further Zig rewrite work. Unfinished Zig tasks and continuation instructions in this document are superseded; retain its historical evidence and existing source. See [the current implementation decision](../REWRITE-DECISION-2026-10-19.md).
<!-- END 2026-10-19 rewrite decision -->

status: in_progress
- AGENTS.md enhanced with Living V2 guard, storage, atlas, and slow-test warnings
- Doc comments added to 5 pub items in src/game/model.rs (WORLD_COUNT, DEFAULT_SEED, RulesV1, World, Fleet)
- All gates passed: native + wasm fmt/clippy/test/build-web/check-workshop
- Plan agent assessments delivered (research only; 5 improvements, dependency constraints, risk notes, 3-month slice)