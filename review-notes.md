# Code Review Notes — NYON Session Changes

## Summary
Reviewed 4 changes against repo conventions, frozen design contracts, and executable sources. **One correctness issue found** in a doc comment; otherwise clean.

---

## Findings

### 1. `src/game/model.rs:4-7` — DEFAULT_SEED doc comment
**Severity:** Medium (correctness)
**Issue:** The comment states "Governed by section 10 of the living galaxy rules" but the seed `0x4947_5731_2026_0902` is the **RulesV1/WorkshopV1** frozen seed, not a Living V2 seed. Section 10 of `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` describes V2 archive format and hashing formulas using a `genesis_seed_32` field — a different concept. The RulesV1 seed is governed by the frozen WorkshopV1 design (see `tests/rules_v1_facade.rs:11`, AGENTS.md:121, and design docs).
**Suggestion:** Change to "Governed by the frozen RulesV1 design; never changes without a versioned design decision." (removes the incorrect section reference)

### 2. `src/game/model.rs:147-149` — RulesV1 doc comment
**Severity:** Low (style/precision)
**Issue:** "seven worlds" in the doc comment for `RulesV1` is contextually accurate (the rule set operates on 7-world campaigns) but the struct itself does not encode the world count — `WORLD_COUNT` does. Not incorrect, but slightly imprecise.
**Suggestion:** Consider "The immutable RulesV1 rule set for the seven-world campaign: fixed tick rate, and the canonical command taxonomy." or leave as-is (acceptable).

### 3. `AGENTS.md` — Living V2 section additions
**Severity:** None
**Status:** All claims verified against executable sources:
- Living V2 path: `crates/nyon-workshop-core/src/living/` ✓
- Contract: `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` ✓
- V1/V2 isolation: `tests/living_wire.rs` `no_crate_source_file_names_both_a_workshop_v1_and_a_living_v2_identity` ✓
- `CRATE_SOURCES` and declaration count: `tests/living_wire.rs:693-726, 837-876` ✓
- `living-v2-vectors.py verify`: `tools/living-v2-vectors.py` exists ✓
- UI atlas workspace: `tools/ui-atlas/` ✓
- Asset files and `ui_assets` test: `assets/ui/`, `tests/ui_assets.rs` ✓

### 4. `AGENTS.md` — Store/Launch/Slow-test additions
**Severity:** None
**Status:** All claims verified:
- `src/app/client_runtime/library.rs` owns exact-catalog loading, returns typed `LibraryEvent` outcomes; `ClientRuntime` owns screen policy/recovery/session ✓
- `WorkshopStore::abandon` forgets outcomes, not work; re-list after abandoned mutations; generation CAS preserved: `src/workshop/store.rs:470-483` ✓
- Native launch: `cargo run --release`; Browser: `python3 -m http.server 8000` → `/web/`; `?backend=webgl2` fallback ✓
- `workshop_store_web` tests host transaction model, not real IndexedDB; wasm adapter needs wasm gate + live browser: `tests/workshop_store_web.rs`, `src/workshop/store/web/wasm.rs` ✓
- Slow test warning for `workshop_ui_layout`: capacity/paging test at line 424, 1337+ ✓

### 5. `tasks/goals.md` — Ledger format
**Severity:** Low (style)
**Issue:** Outcome bullet "All gates passed: native + wasm fmt/clippy/test/build-web/check-workshop" reads as accomplished fact but `status: in_progress`. This is a target, not a result.
**Suggestion:** Rephrase to "Target: all gates pass (native + wasm fmt/clippy/test/build-web/check-workshop)" or "Gate validation: native + wasm fmt/clippy/test/build-web/check-workshop"

### 6. `tasks/todo.md` — Checklist format
**Severity:** None
**Status:** Clean. 8 specific, actionable items with checkboxes. All grounded in repo context (CRATE_SOURCES, living-v2-vectors.py, WGSL⇆Vertex, rules_v1_facade, WorkshopStore::abandon, cross-build fingerprint, cargo doc).

---

## Verdict
**One correctness fix needed** (DEFAULT_SEED doc comment). All other changes are accurate, well-grounded, and follow repo conventions. The AGENTS.md additions are particularly thorough — every claim traces to source code or tests.

### Required Fix
```diff
- /// Frozen deterministic seed for the RulesV1 generator (0x4947_5731_2026_0902).
- /// Governed by section 10 of the living galaxy rules; never changes without
+ /// Frozen deterministic seed for the RulesV1 generator (0x4947_5731_2026_0902).
+ /// Governed by the frozen RulesV1 design; never changes without
```

### Optional Polish
- `goals.md`: rephrase the "All gates passed" bullet to clarify it's a target
- `RulesV1` doc comment: minor precision improvement if desired