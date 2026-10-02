# Living creator qualification matrix

Status: **Partial**. Reconciled from current command/rules/additive helper, client forms, and tests on 2026-10-01. The current user implementation plan accepts the creator contract despite the historical addendum Draft header. Later approved additive operations override only their explicitly reviewed surfaces; remaining conflicts are named below.

Every operation has a typed form and authority implementation. No operation is presently classified UnsupportedOperation. Current means a narrowly exercised case, not exhaustive capability acceptance. No row establishes live native/browser, keyboard/screen-reader, all running/paused boundaries or maximum-capacity performance.

Evidence identifiers:

- **A**: root `every_creator_form_valid_transaction_has_exact_undo_redo_and_canonical_reload` in `tests/living_creator_qualification.rs`; all 46 typed forms preview/apply through public session actions, then exact undo state, redo bookmark and canonical archive reload. Necessary preconditions are explicit: queued jobs for cancellation, split fleet for merge, agreement/war for removal/peace, shipment for disposition, Commerce at tick 100 for route edits and Interrupted Corridor for hazard edits. Earlier execution exit 0, 4/4 root qualification tests in `/tmp/nyon-creator-matrix-tests5.log`; the later boundary follow-up below also exercises all 46 variants. Earlier failed sweep remains defect-discovery evidence, not current failure.
- **B**: `every_creator_form_missing_reference_or_invalid_local_rejects_without_history_or_identity_changes`; replaces one actual existing target with a nonexistent identity, or breaks a no-reference creation local ordinal. Exact state and canonical archive/high-water/history preservation is checked. First sweep rejects 44/46; RemoveAgreement and ForcePeace were accepted incorrectly. Authority repaired both missing-participant cases. The earlier root sweep passes for all 46 with exact archive/state preservation (`/tmp/nyon-creator-matrix-tests5.log`, 4/4 overall); the later boundary follow-up remains green.
- **F**: `every_creator_tool_produces_a_typed_operation_form_and_search_is_draft_only` (`tests/living_creator.rs`) checks typed form construction for all 46, not successful application. A supplies actual public form preview/apply evidence.
- **Q**: shared structural tests `a_batch_over_the_declared_bound_is_refused`, `every_reference_bearing_field_is_validated`, `a_cascade_must_name_its_dependents_exactly_once` (`core/tests/living_command.rs`). They establish batch/schema constraints, not each authority allocation cap.
- **K**: new `system_and_per_fleet_hull_capacity_rejection_preserves_canonical_history` fills system and per-fleet hull caps, rejects the next operation and steps existing simulation. Passes in `/tmp/nyon-creator-matrix-tests5.log`; this covers system 64 and per-fleet hull 16, not global hull 2048.

- **G**: `star_civilization_deposit_and_hazard_caps_reject_whole_batches_then_reclaim_and_replay` exercises stars 128, civilizations 16, deposits 1024 and hazards 128 through validated public genesis fixtures and public creator transactions. Each oversized allocation follows an earlier valid inventory edit in the same batch: refusal preserves exact state and canonical archive, including history/high-water marks. Explicit removal reclaims capacity, creation succeeds, undo/redo and canonical reload preserve exact boundaries, and simulation can still step.
- **S**: `six_industrial_slots_include_reserved_jobs_and_same_batch_cancel_releases_capacity` exercises six world industrial slots including a reserved construction job. A seventh facility or second construction job rejects an entire batch, including its earlier inventory edit. Explicit cancellation followed by facility creation in the same batch releases capacity; exact undo/redo and reload retain the reservation or final facility as appropriate.
- **H**: `global_hull_capacity_counts_paid_reservations_and_releases_only_after_cancel` exercises the global 2048-hull limit with 2047 actual hulls plus one paid hull-job reservation. Creating a hull in a separate empty fleet rejects despite available local capacity; splitting the reservation target rejects without mutation. Explicit cancellation followed by creation succeeds at 2048 actual hulls. Exact undo/redo/archive reload preserve hull IDs, credits, HP, the reservation and inventory as part of complete state equality.
- **D**: `reviewed_system_cascade_rejects_omitted_and_unrelated_keys_then_restores_exact_history` independently requires the target System's worlds, stars, connected lanes, deposits and facilities in the reviewed closure. Reject-if-referenced, omitted/unrelated stable identities and omitted/unrelated derived keys all refuse a batch after an earlier valid inventory edit without changing canonical state/history. The complete reviewed removal deletes the system and its world/lane references, preserves surviving world records, validates, reloads, and has exact undo/redo.

G/S/H/D and A/B/K are exercised together by the latest eight-test root suite, exit 0, `/tmp/nyon-experience-creator-matrix-final-tests2.log`. These are automated host authority cases, not live platform UI or maximum-population performance qualification.

| Operation | Valid | Invalid / atomic refusal | Capacity / reservations | Cascade / dependency behavior | Undo / redo | Reload | Overall |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `CreateSystem` | Current: A | Current: B missing reference/local | Current: K system 64 | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `CreateStar` | Current: A | Current: B missing reference/local | Current: G stars 128; other boundaries Partial | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `CreateWorld` | Current: A | Current: B missing reference/local | Current: T worlds 512; other boundaries Partial | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `CreateLane` | Current: A | Current: B missing reference/local | Current: T lanes 256; other boundaries Partial | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `CreateCivilization` | Current: A | Current: B missing reference/local | Current: G civilizations 16; other boundaries Partial | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `CreateDeposit` | Current: A | Current: B missing reference/local | Current: G deposits 1024; other boundaries Partial | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `CreateFacility` | Current: A | Current: B missing reference/local | Current: S six slots including reservation; global 2048 Partial | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `CreateConstructionJob` | Current: A | Current: B missing reference/local | Current: S six-slot/one-building-queue reservation; other boundaries Partial | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `CreateHullJob` | Current: A | Current: B missing reference/local | Current: H global reservation accounting; job admission boundaries Partial | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `CreateFleet` | Current: A | Current: B missing reference/local | Current: T fleets 256; other boundaries Partial | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `CreateHull` | Current: A | Current: B missing reference/local | Current: K per-fleet 16 and H global 2048 including reservations | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `CreateRoute` | Current: A | Current: B missing reference/local | Partial: operation-specific boundary matrix | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `CreateShipment` | Current: A | Current: B missing reference/local | Partial: operation-specific boundary matrix | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `CreateHazard` | Current: A | Current: B missing reference/local | Current: G hazards 128; other boundaries Partial | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `SetWorldInventory` | Current: A | Current: B missing reference/local | Partial: operation-specific boundary matrix | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `SetWorldOwner` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Partial: focused source tests below; full dependency matrix absent | Current: A | Current: A | Partial |
| `CreateColony` | Current: A | Current: B missing reference/local | Partial: operation-specific boundary matrix | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `RemoveColony` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Partial: focused source tests below; full dependency matrix absent | Current: A | Current: A | Partial |
| `SetDepositRemaining` | Current: A | Current: B missing reference/local | Partial: operation-specific boundary matrix | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `SetCivilizationPolicy` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `SetRelationBase` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `SetAgreement` | Current: A | Current: B missing reference/local | Partial: operation-specific boundary matrix | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `RemoveAgreement` | Current: A | Current: B missing reference | Current: no intrinsic allocation; Q batch bound | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `ForceWar` | Current: A | Current: B missing reference/local | Partial: operation-specific boundary matrix | Partial: focused source tests below; full dependency matrix absent | Current: A | Current: A | Partial |
| `ForcePeace` | Current: A | Current: B missing reference | Current: no intrinsic allocation; Q batch bound | Partial: focused source tests below; full dependency matrix absent | Current: A | Current: A | Partial |
| `SetFleetOrder` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Partial: focused source tests below; full dependency matrix absent | Current: A | Current: A | Partial |
| `SetShipmentDisposition` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Partial: focused source tests below; full dependency matrix absent | Current: A | Current: A | Partial |
| `RemoveEntity` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Current: D reviewed System stable/derived closure; other closure matrix Partial | Current: A | Current: A | Partial |
| `EditSystem` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `EditStar` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `EditWorld` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Partial: focused source tests below; full dependency matrix absent | Current: A | Current: A | Partial |
| `EditLane` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Partial: focused source tests below; full dependency matrix absent | Current: A | Current: A | Partial |
| `EditDeposit` | Current: A | Current: B missing reference/local | Partial: operation-specific boundary matrix | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `EditCivilization` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `SetFacilityEnabled` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `ScrapFacility` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Partial: focused source tests below; full dependency matrix absent | Current: A | Current: A | Partial |
| `CancelConstructionJob` | Current: A | Current: B missing reference/local | Current: S same-batch capacity release; Q batch bound | Partial: focused source tests below; full dependency matrix absent | Current: A | Current: A | Partial |
| `CancelHullJob` | Current: A | Current: B missing reference/local | Current: H global capacity release; Q batch bound | Partial: focused source tests below; full dependency matrix absent | Current: A | Current: A | Partial |
| `EditFleet` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Partial: focused source tests below; full dependency matrix absent | Current: A | Current: A | Partial |
| `EditHull` | Current: A | Current: B missing reference/local | Partial: operation-specific boundary matrix | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `SplitFleet` | Current: A | Current: B missing reference/local | Current: H paid target reservation refusal; fleet 256 Partial | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `MergeFleets` | Current: A | Current: B missing reference/local | Partial: operation-specific boundary matrix | Partial: focused source tests below; full dependency matrix absent | Current: A | Current: A | Partial |
| `EditRoute` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `SetRouteSuspended` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `EditHazard` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Current: no removal cascade in schema; B validates reference | Current: A | Current: A | Partial |
| `CancelHazard` | Current: A | Current: B missing reference/local | Current: no intrinsic allocation; Q batch bound | Partial: focused source tests below; full dependency matrix absent | Current: A | Current: A | Partial |

Specific existing test witnesses (source mapping; this section does not silently promote every referenced test to an independently executed latest gate):

- `CreateSystem`: `route_suspend_adjustment_and_hazard_edits_replay_in_running_queue`, `system`.
- `CreateStar`: `a_second_command_sees_the_first_commands_entities_in_its_projection`, `structural_and_projected_failures_are_rejected_at_submission`.
- `CreateWorld`: `edits_apply_to_existing_records`.
- `CreateLane`: `route_suspend_adjustment_and_hazard_edits_replay_in_running_queue`, `created_entities_take_the_identity_their_revision_derives`.
- `CreateCivilization`: `derived_only_cascade_with_no_stable_dependents_is_representable`, `a_rejected_paused_edit_changes_nothing`, `edits_apply_to_existing_records`.
- `CreateDeposit`: `edits_apply_to_existing_records`.
- `CreateFacility`: `shipyard_and_battery_uniqueness_count_existing_and_reserved_assets`.
- `CreateConstructionJob`: `outstanding_world_queue_reservations_reject_new_work_atomically`, `shipyard_and_battery_uniqueness_count_existing_and_reserved_assets`, `cancellation_refunds_overflow_and_hull_reservations_are_atomic_and_replayable`.
- `CreateHullJob`: `cancellation_refunds_overflow_and_hull_reservations_are_atomic_and_replayable`.
- `CreateFleet`: `fleet_split_merge_preserves_hull_ids_hp_credits_and_rejects_bad_membership`.
- `CreateHull`: `fleet_split_merge_preserves_hull_ids_hp_credits_and_rejects_bad_membership`, `cancellation_refunds_overflow_and_hull_reservations_are_atomic_and_replayable`.
- `CreateHazard`: `route_suspend_adjustment_and_hazard_edits_replay_in_running_queue`.
- `SetWorldInventory`: `cancellation_refunds_overflow_and_hull_reservations_are_atomic_and_replayable`, `structural_and_projected_failures_are_rejected_at_submission`, `edits_apply_to_existing_records`.
- `SetWorldOwner`: `cancelling_a_job_refunds_half_and_creator_owner_changes_require_explicit_cancellation`.
- `RemoveColony`: `independent_entity_removal_and_colony_removal_have_deterministic_history`, `removal_of_a_missing_colony_is_refused_without_spending_sequence`.
- `SetDepositRemaining`: `edits_apply_to_existing_records`.
- `SetCivilizationPolicy`: `edits_apply_to_existing_records`.
- `SetRelationBase`: `derived_only_cascade_with_no_stable_dependents_is_representable`.
- `RemoveEntity`: `remove`, `a_world_cascade_rejects_ambiguity_and_omissions_before_spending_then_replays_exactly`, `independent_entity_removal_and_colony_removal_have_deterministic_history`, `generic_creator_job_removal_is_an_override_without_economic_refund`, `exact_reviewed_cascade_is_explicit_and_rejects_incomplete_keys`, `derived_only_cascade_with_no_stable_dependents_is_representable`.
- `EditSystem`: `edits_preserve_ids_and_replay_after_nonzero_boundary`.
- `EditStar`: `edits_preserve_ids_and_replay_after_nonzero_boundary`.
- `EditWorld`: `edits_preserve_ids_and_replay_after_nonzero_boundary`.
- `EditLane`: `route_suspend_adjustment_and_hazard_edits_replay_in_running_queue`.
- `EditDeposit`: `edits_preserve_ids_and_replay_after_nonzero_boundary`.
- `EditCivilization`: `edits_preserve_ids_and_replay_after_nonzero_boundary`.
- `SetFacilityEnabled`: `disable_due_clock_no_catchup_and_scrap_preserve_atomic_history`.
- `ScrapFacility`: `disable_due_clock_no_catchup_and_scrap_preserve_atomic_history`.
- `CancelConstructionJob`: `cancelling_a_job_refunds_half_and_creator_owner_changes_require_explicit_cancellation`, `cancellation_refunds_overflow_and_hull_reservations_are_atomic_and_replayable`.
- `CancelHullJob`: `cancelling_a_job_refunds_half_and_creator_owner_changes_require_explicit_cancellation`, `cancellation_refunds_overflow_and_hull_reservations_are_atomic_and_replayable`.
- `EditFleet`: `fleet_and_hull_edits_validate_health_and_preserve_transit_boundaries`.
- `EditHull`: `fleet_and_hull_edits_validate_health_and_preserve_transit_boundaries`.
- `SplitFleet`: `fleet_split_merge_preserves_hull_ids_hp_credits_and_rejects_bad_membership`.
- `MergeFleets`: `fleet_split_merge_preserves_hull_ids_hp_credits_and_rejects_bad_membership`.
- `EditRoute`: `route_suspend_adjustment_and_hazard_edits_replay_in_running_queue`.
- `SetRouteSuspended`: `route_suspend_adjustment_and_hazard_edits_replay_in_running_queue`.
- `EditHazard`: `route_suspend_adjustment_and_hazard_edits_replay_in_running_queue`.
- `CancelHazard`: `route_suspend_adjustment_and_hazard_edits_replay_in_running_queue`.

Contract discrepancies and remaining acceptance work:

- `SetWorldOwner`: addendum WO.3 rejects foreign paid jobs until explicit earlier cancellation/removal; prior source auto-cancelled/refunded as capture. Root ruled creator ownership must follow WO.3, while conquest capture retains automatic cancellation/refund. Authority repaired the ownership guard; new root `generic_removal_is_free_explicit_cancellation_refunds_and_owner_change_requires_paid_job_disposition` passes refusal and explicit earlier cancellation/removal batch branches with reload.
- X7 forbids creator refunds globally, while later approved explicit cancel/scrap operations specify half refunds and overflow receipts. Explicit additive cancel/scrap semantics supersede X7 for those tags. Root ruled generic `RemoveEntity` remains a free creator override with no refund; authority repaired it and the strong root differentiation regression passes. Explicit economic cancellation followed by owner change refunds; generic removal followed by owner change does not.
- Every operation still needs its applicable minimum/maximum scalar/catalog/target/transit boundary cases, staged running-queue replay, stale acceptance, and all relevant capacity/reservation and cascade closure cases. Missing cells are acceptance gaps, not newly invented gameplay.
- Capacity reclamation and a full reviewed System stable/derived closure are exercised by G/S/H/D. Civilization/world cascades and derived-only closure variants have selected existing focused tests, but the exhaustive all-key/per-operation cross-product remains Partial.
- Actual live native/WebGPU/WebGL2 creator keyboard/pointer/semantic previews, apply, undo, reload and rejection journeys remain unverified. All Current cells above are automated host cases only.

Independent current focused gates:

- `cargo test --test living_creator_qualification`: exit 0, 4/4, including all46 successful/undo/redo/reload forms, all46 invalid reference/local transactions, two actual capacity boundaries and creator-free/economic-refund distinction; `/tmp/nyon-creator-matrix-tests5.log`.
- `cargo test -p nyon-workshop-core --test living_creator --test living_creator_additions --test living_queue --test living_additions_vectors`: exit 0, 7/7 creator, 9/9 additive creator, 18/18 queue, 6/6 independent additions vectors; `/tmp/nyon-creator-core-review-tests2.log`. The earlier run hit a temporary owned core test `.receipt` compile error; the repaired run above is the current evidence.
- Original Living vector corpus unchanged: Python verification exit 0 before/after new shipment fields (`/tmp/nyon-shipment-kind-before.log`, `/tmp/nyon-shipment-kind-after2.log`). Independent additions fixture covers omitted legacy kind, Aid, Trade, and Trade waiting at source with sparse return flag. Codec/formula tests do not by themselves prove relationship counter gameplay.

No UnsupportedOperation row remains for the46 approved operation schemas. Every row remains overall Partial until applicable scalar/relationship/transit/capacity/reservation/closure matrices and live platform journeys are complete.

Strict workspace Clippy at this moving-tree snapshot exited 101 first on
presentation crest parity (owner repaired), then on new unused phase3 helper
`wire.rs::record_at_mut` (`/tmp/nyon-creator-matrix-clippy2.log`). No final strict
gate pass is inferred from focused test passes. Shipment vector follow-up passes
6/6 (`/tmp/nyon-shipment-kind-vectors2.log`) with isolated noncanonical explicit
null/false rejection in otherwise correctly ordered shipment bytes.

## 2026-10-01 creator boundary follow-up

Current: the latest public authority matrix suite passes 8/8, exit 0 (`/tmp/nyon-experience-creator-matrix-final-tests2.log`). All 46 existing form preview/apply/undo/redo/reload cases and all 46 invalid reference/local cases remain green alongside four newly added boundary tests. The fixtures use public genesis validation and creator APIs; they do not alter a live authority's state or reuse authority-generated expected digest goldens. This slice adds no production code or wire changes.

Partial: global world 512, lane 256, fleet 256, industry 2048, route 2048 and shipment 4096 creator allocation/reclamation boundaries are not established by this root matrix. In particular, the independent 512-world decision audit is not creator-allocation evidence. Numeric/catalog/transit boundaries for every operation and staged running-queue capacity/cascade transactions still require their applicable tests. The selected System closure is not proof of every Civilization/World/derived-reference closure combination.

Partial: these host tests do not establish live native, WebGPU or WebGL2 form geometry, typing, pointer/keyboard/screen-reader operation, graphics recovery, or maximum-capacity timing. The historical failed broad lint snapshots above remain historical evidence; the separately reported root full gates own current workspace qualification.

The focused strict lint invocation `cargo clippy --test living_creator_qualification -- -D warnings` passed before the final unrelated-derived-key assertion (`/tmp/nyon-experience-creator-matrix-focused-clippy.log`, exit 0). Its final-source rerun exited 101 on two new authority-owned `production_job.rs` constant `ok_or_else` closures (`/tmp/nyon-experience-creator-matrix-focused-clippy2.log`). This is a shared production lint failure, not a passing final lint gate or a creator-test failure; its owner was notified.

The final test source also pins the accepted numerical table independently of the implementation constants: systems 64, stars 128, civilizations 16, deposits 1024, hazards 128, hulls 2048 and hulls per fleet 16. The rerun adding those pins stopped before test execution on the in-progress client Notebook-evidence seam (`LivingNotebookEvidenceV2` missing import in `src/app/client_runtime/living.rs`, `/tmp/nyon-experience-creator-matrix-final-tests3.log`, exit 101). The eight-case passing result above predates those added assertions; a coherent shared-source rerun is pending.


## 2026-10-01 resumed creator and durable Notebook journey

Current: `cargo test --test living_creator_qualification --test living_notebook_journey` exited 0 with **9 creator cases and 2 Notebook journey cases**, no failed or ignored tests (`/tmp/nyon-journey-final-tests.log`). This supersedes the earlier interrupted missing-import rerun for the numerical assertions. All 46 valid and invalid form sweeps remain included.

- **T**: `world_lane_and_fleet_caps_reject_atomically_and_reclaim_through_exact_history` independently pins worlds 512, lanes 256 and fleets 256. Each public validated-genesis fixture is exactly at its accepted maximum. A creator allocation after an earlier valid inventory edit refuses the whole transaction without changing state, archive, history or high-water identities. Removing an unreferenced entity permits replacement; undo, redo and canonical reload preserve the exact boundaries, and the maximum state can step. This covers allocation/reclamation, not capacity rendering or timing.
- The durable Notebook journey records a genuine First Expansion settlement at tick 361, validates the portable evidence attachment, saves through the native store/coordinator, destroys the writer, resolves exact Continue in a new client, installs the replay-validated Notebook, and exposes the complete receipt/event identities through public UI controls with an empty receipt cache. A separate regression proves a legacy empty attachment retains the evidence ordinal high-water mark and the next allocation cannot recycle an identity; the new attachment passes independent replay validation. A creator edit at the same tick rejects the previous settlement witness without changing authority, Notebook metadata or generation.

Partial: the concrete remaining creator axes are global **industry 2048, route 2048 and shipment 4096** allocation/reclamation; applicable inventory/deposit/HP/time/distance integer minimum/maximum and overflow cases; unknown catalog references for archetypes, facilities, hulls, policies and hazard definitions; docked versus departing/transit/arrived fleet and shipment edits; queued running commands with stale cursor or exhausted capacity; world/civilization removal closures covering colonies, occupations, relations, agreements, paid jobs, fleets, routes, in-flight manifests and observations; duplicate/missing/unrelated derived closure keys for every applicable target; and split/merge/transfer reservations at each local/global limit. Selected source tests cover pieces of these axes, but this root matrix does not establish the complete cross-product. These are qualification gaps for approved behavior, not proposed gameplay.

Partial: focused strict lint exited 101 (`/tmp/nyon-journey-final-clippy.log`) before checking the owned test targets, on current authority observation-work changes: unused observation imports in `living/rules.rs` and a collapsible conditional in `living/step_job.rs`. The authority owner was notified; a repaired rerun is required. Host tests, live native/WebGPU/WebGL2 journeys, actual IndexedDB, assistive-technology speech and maximum-capacity timing remain separate evidence boundaries.

Current: after the authority owner repaired those two lints, `cargo clippy --test living_creator_qualification --test living_notebook_journey -- -D warnings` exited 0 on the unchanged owned test sources (`/tmp/nyon-journey-final-clippy2.log`). The earlier exit 101 remains defect-discovery history; it does not describe the repaired focused snapshot. This focused lint pass does not replace the full workspace or wasm gates.


## Qualification limits retained during the player-manual refresh

The verified durable Notebook attachment records exact ordinary-receipt/event references, validates them through replay, and restores them through native save/Continue with an empty receipt cache. It does not complete five remaining suggested experiment evidence workflows: supply chain, trade delivery, corridor impact, war alternative, and portable reproduction. Their visible briefings and fail-closed recorder refusals are current behavior; full outcome witnesses and artifact-pair reproduction evidence remain Partial.

Whole-step budgeting remains Partial. Individual arrivals, production, repair and observation cursors do not establish a bounded full tick: the resident Session still drains complete synchronous authority steps, while forecasting, full validation, canonical receipt sealing and publication also require their accepted whole-path treatment. The twenty-tick catch-up cap bounds transport count, not visited records, wall time, or maximum-capacity frame performance. These limitations remain explicit in PLAYER-MANUAL.md rather than being promoted from successful compilation or old live bundles.


## Exact creator recovery observation follow-up

Current: the public recovery slice passes 8/8 host tests, exit 0 (`/tmp/nyon-recovery-journey-tests3.log`): six authority/portable-Notebook cases and two Session/native-store journeys. The actual pre-consumer replay test failed with Evidence for genuine creator recovery while five exclusion cases passed; the repaired consumer validates an exact CreatorIntervention reference using authority-produced, receipt-bound whole-revision transition proof. An existing actor must begin Dormant with zero colonies and end Active with an owned colony that remains in the matching bookmark. Newly created actors, initially Active actors, unusual Dormant actors already owning a colony, transient create/remove batches, unrelated name/inventory/ownership-only edits, another revision's proof and final colony loss do not qualify. Creator recovery emits no autonomous CivilizationReactivated event and introduces no wire shape or gameplay-rule change.

The Session journey preview/applies the genuine intervention, records its observation, saves through native Living storage/coordinator, reopens exact Continue in a new client, and displays complete replay-verified receipt/event references without the recent receipt cache. An unrelated creator inventory edit refuses recording without authority or Notebook mutation. Five other suggested evidence workflows and whole-path budgeting remain Partial as stated above. Actual platform journeys remain separate.
