# Workshop inspector repair and Classic guide acceptance

Date: 2026-09-04

Independent reviewer: `map_sdf_ui`.

## Verdict

**Inspector specification and quality: APPROVED. Classic guide correction specification and quality: APPROVED. No open P0-P2 findings in these slices.** This supersedes the earlier inspector request-changes verdict for the repaired source, not for runtime qualification.

## Inspector closure

- Fixed title/body/footer: the title is independent of scrolling; the measured body ends above actual footer controls. The 1280x480 100%/130% regression covers maximum 512-byte detail, final record, title persistence, clipping containment, and control disjointness.
- Complete navigation: Next and Previous traverse wrapped lines symmetrically. Reveal resets offsets. Retained view state reconciles selection and responsive-width changes instead of applying stale deep offsets to new content. Drawer and docked layouts use the same wrapping calculation.
- Stable identity: full CatalogId keys identify inventory facts; full EntityId keys identify related objects and selected-entity namespaces. Cross-frame insertion tests preserve existing resource and related-object IDs.
- Focused ownership: private `src/ui/workshop_inspector.rs` owns catalog-backed derivation, identity, recipe/readiness and hazard-capacity calculations. `src/ui/platform_inspector.rs` owns sighted geometry/wrapping. Public model contracts remain in `ui::workshop`. The parent Workshop module decreased from 2794 to 1987 lines.

Evidence: `tests/workshop_ui.rs` tests `short_height_inspector_keeps_title_fixed_and_every_line_inside_body_clip`, `inspector_cursor_reconciles_selection_width_and_wrapped_line_identity`, and `inspector_fact_ids_survive_inventory_and_related_entity_insertions`.

## Guide closure

Guide and manual now teach half-current-energy launch strength and cost, unchanged source defense, defense protection and friendly reinforcement, and snapshotted launch Hydrosphere. Classic scenario saves remain clearly distinguished from resumable battles.

The first guide-test review found disconnected keyword checks that could pass incorrect teaching. The repaired test collects only Classic-topic pages with a finite guard, normalizes whitespace, and checks cohesive guide/manual clauses separately. Its public simulation fixture disables production, base regeneration, and Topology regeneration, proving 80000 energy becomes a 40000-strength fleet plus 40000 retained energy while 7000 defense remains unchanged and Hydrosphere 3 is snapshotted.

## Fresh controller gates

- `cargo fmt --all --check`: exit 0.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: exit 0.
- `cargo test --workspace --all-targets`: exit 0; **375 passed, 0 failed**.
- Pure-core and root wasm checks: exit 0.
- Root wasm warnings-denied Clippy: exit 0.
- Separate fuzz workspace format and locked Clippy: exit 0; no fuzz execution claimed.

The source tree remains a mixed, uncommitted Workshop baseline at documentation HEAD b79f5c9. Scoped before/after snapshots and worker reports are retained in the program ledger directory. These gates do not establish fresh native/browser rendering, live accessibility, IndexedDB durability, or non-skipped GPU parity. Live UI acceptance follows the integrated SDF slice using newly built artifacts. This source approval clears the SDF plan's inspector prerequisite, not whole-product completion.
