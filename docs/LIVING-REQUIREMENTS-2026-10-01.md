# Living Galaxy requirement reconciliation — 2026-10-01

Status: **Partial**. This ledger covers every LG-01–27 requirement individually.
Source/test locations describe present coverage, not completion. The 1,353-test
dispatch checkpoint is historical once forecast input source changes. New gate
results must retain their own checkpoint identity. Runtime evidence is limited
to the browser journey explicitly recorded in the dispatch evidence document.

| ID | Requirement | Source | Automated coverage | Remaining acceptance |
| --- | --- | --- | --- | --- |
| LG-01 | **Partial** — Visible diorama, correct compositing | src/presentation/living.rs; src/ui/platform_sdf.rs | living_presentation, living_contrast | Live populated route/world screenshots, compositor and contrast matrix |
| LG-02 | **Partial** — Readable responsive interface | src/sandbox_session.rs; src/ui/platform_sdf.rs | workshop_ui_layout, living_scene_labels | All declared viewport/scale combinations, zero overlap, resize during edits |
| LG-03 | **Partial** — Complete visible inspector | src/sandbox_session.rs; src/living_navigation.rs | living_navigation, living_presentation | Live inspection of every entity kind at capacity |
| LG-04 | **Partial** — Searchable reachable hierarchy/history | src/living_navigation.rs; src/sandbox_session.rs | living_navigation, living_history_ui | First/last/deepest filtered reveal and all retained branches live |
| LG-05 | **Partial** — Conventional form editing | src/living_creator.rs; src/sandbox_session.rs | living_creator, living_creator_qualification | Live keyboard caret/selection/paste/cancel/focus restoration |
| LG-06 | **Partial** — Galaxy/system/world navigation | src/living_camera.rs; src/living_camera_input.rs | living_camera, living_initial_fit | Pointer/keyboard navigation and selected-entity removal journey |
| LG-07 | **Partial** — V2 deterministic authority | crates/nyon-workshop-core/src/living/simulation.rs | living_play, living_wire, living_queue | Complete bounded resident execution and live canonical platform parity |
| LG-08 | **Partial** — Real resource economy | crates/nyon-workshop-core/src/living/rules.rs | living_play, living_supply_completion, living_trade_facts | Capacity scarcity and live supply/recovery journeys |
| LG-09 | **Partial** — Autonomous construction/expansion | crates/nyon-workshop-core/src/living/rules.rs | living_play, living_decision, living_content | Bounded forecasts/intents and all ordinary starter witnesses |
| LG-10 | **Partial** — Diplomacy | crates/nyon-workshop-core/src/living/rules.rs | living_play, living_decision | Live bilateral reasons, simultaneous negotiation and expiry journeys |
| LG-11 | **Partial** — Fleet conflict/occupation | crates/nyon-workshop-core/src/living/rules.rs; crates/nyon-workshop-core/src/living/step_job.rs | living_play, living_event_facts | Live simultaneous conflict/capture/retreat and asset conservation qualification |
| LG-12 | **Partial** — Continuous creator sandbox | src/sandbox_session.rs; crates/nyon-workshop-core/src/living/simulation.rs | living_play, living_runtime | Render cadence/speed parity through complete bounded resident path |
| LG-13 | **Partial** — Honest events and strategy explanations | src/living_chronicle.rs; src/sandbox_session.rs | living_chronicle, living_notebook_evidence | All recorded explanation surfaces reachable and authenticated live |
| LG-14 | **Partial** — History and what-if comparison | crates/nyon-workshop-core/src/living/history.rs; crates/nyon-workshop-core/src/living/archive.rs | living_history, living_history_ui | Bounded replay/comparison/retirement and exact matching-tick visible differences |
| LG-15 | **Partial** — Durable save/Continue/recovery | src/living_store.rs; src/living_store/native.rs; src/living_store/web.rs | living_store, living_runtime | Native quit/reopen and actual quota/corruption/interruption/CAS recovery; browser tick-1 save/reopen observed |
| LG-16 | **Partial** — Portable import/export | src/living_transfer.rs; src/living_transfer/web.rs | living_client, living_notebook_attachment | UI export/import into separate slot and next-1,000-tick reproduction |
| LG-17 | **Partial** — Legacy preservation | src/game; src/workshop; src/scenario/store.rs | rules_v1_facade, campaign, scenario, workshop_web | Live legacy open and primary-slot precedence; frozen automated oracle passed at prior checkpoint |
| LG-18 | **Partial** — Accessible operation | src/ui/accessibility.rs; src/ui/platform_native.rs; src/ui/platform_web.rs | workshop_accessibility, living_creator_qualification | Complete pointer/keyboard/native/browser screen-reader journeys |
| LG-19 | **Partial** — Graphics/backend resilience | src/platform/native.rs; src/platform/web.rs; web/loader.js | workshop_web, living_runtime | Native acquisition/recovery matrix; WebGPU/WebGL2 Low ordinary launch and reopen observed |
| LG-20 | **Partial** — Performance/offline operation | src/sandbox_session.rs; crates/nyon-workshop-core/src/living/step_job.rs | living_qualification | Named-device p95 timing, capacity memory and full network-disabled journey |
| LG-21 | **Partial** — Starter/content/experiments | src/living_content.rs; src/sandbox_session.rs | living_content, living_corridor_journey, living_notebook_journey | Controlled Corridor, war alternative and verified portable-reproduction completion |
| LG-22 | **Partial** — Audio and presentation isolation | src/living_audio.rs; src/presentation/living.rs | living_presentation, living_contrast | Actual audio mute/rate/no-device cleanup and presentation digest isolation |
| LG-23 | **Partial** — Documentation and delivery | docs/PLAYER-MANUAL.md; docs/LIVING-DISPATCH-EVIDENCE-2026-10-01.md | Documentation review against executed controls | Update full manual after complete behavior; commit/push/publication outside current authorization |
| LG-24 | **Partial** — Unrestricted creator capability | src/living_creator.rs; crates/nyon-workshop-core/src/living/creator_additions.rs | living_creator_qualification, living_creator_additions | 46-tool paused/running/capacity/dependency/keyboard/live matrix |
| LG-25 | **Partial** — Cross-authority Continue and lifecycle | src/app/client_runtime/living.rs; src/living_store/manifest.rs | living_runtime, living_store, living_client | Cross-authority Continue failures, generation races and real adapter reclamation |
| LG-26 | **Partial** — Canonical V2 wire identity | crates/nyon-workshop-core/src/living/wire.rs; tools/living-v2-vectors.py | living_wire, living_additions_vectors | Live native/wasm full-byte equivalence; original and 17 additive vector modes passed at prior checkpoint |
| LG-27 | **Partial** — Running creator queue | crates/nyon-workshop-core/src/living/simulation.rs | living_queue, living_command, staged_step_ unit tests | Complete bounded complex creator operations and resident tail/fault publication |

No row is marked complete solely because compilation or a test suite passed.
No host is marked Blocked without an attempted prerequisite failure. The
native bare-executable computer-use binding failed; native runtime qualification
still needs an app-bound journey. The scripted cross-build check does not extract
canonical bytes from two live browser engines.
