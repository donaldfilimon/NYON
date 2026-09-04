# Workshop V1 Baseline Review — 2026-09-04

## Verdict

**REQUEST CHANGES — do not commit or merge the current Workshop V1 baseline as a completed implementation.** No P0 issue was found, but the current tree has four P1 product/recovery blockers and five P2 correctness or maintainability findings. The source gates are green; that does not compensate for missing or obscured release-acceptance behavior.

## Scope and evidence boundary

- Reviewed the complete current tracked and untracked Workshop implementation in `/Users/donaldfilimon/Public/NYON` against `AGENTS.md`, `.superpowers/sdd/nyon-v2/progress.md`, `docs/superpowers/specs/2026-09-02-nyon-v2-design.md`, and the active `docs/superpowers/plans/2026-09-02-nyon-v2.md` program and client-plan requirements. Newly appearing 2026-09-04 Living Galaxy planning documents were treated as concurrent future-program work, not Workshop V1 implementation authority.
- Included the untracked pure core, Workshop client/store/UI/presentation files, tests, assets, browser loader, and build/qualification scripts in the review. No product file was edited, staged, or committed.
- Current-tree source evidence is green: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-targets` (340 tests), and `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings` all exited 0.
- No native or browser build, live GPU interaction, keyboard/pointer acceptance journey, visual comparison, named-browser matrix, cross-platform run, or performance qualification was performed for this review. Per `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:17-27` and `:490-497`, the green checks establish source/test evidence only.

## Findings

### 1. [P1] The platform frame nearly completely covers the Workshop scene, and fully covers it in high-contrast mode

- **Severity:** P1
- **File:line:** `src/app.rs:762-823`, `src/app.rs:849-870`, `src/app.rs:1253-1269`, `src/ui/platform.rs:105-126`, `src/engine/render.rs:185-211`
- **Description:** `build_workshop_frame` draws the derived Workshop scene into `underlay_batch`, then `install_platform_frame` draws the platform UI into `overlay_batch`. The renderer encodes the primitive underlay before the overlay. `PlatformUiFrame::draw` begins the overlay with a viewport-sized quad at alpha `0.96`, or alpha `1.0` in high-contrast mode. Consequently the galaxy canvas is reduced to roughly four percent of its intended color contribution in normal mode and is completely hidden in high-contrast mode. Workshop also never supplies the renderer's `scene` channel: `render_frame` populates that field only for `ClassicSector`. This defeats the central Shape/Observe surface even though scene extraction tests can remain green.
- **Suggestion:** Make the Workshop shell background panel-region-aware so the center canvas remains uncovered, or give `WorkshopSceneFrame` an explicit renderer layer whose ordering is intentional. Add a renderer/layout regression that asserts a center-canvas world/star pixel is not covered by an opaque shell quad in both normal and high-contrast modes, then perform a live visual recheck.
- **Status:** Open — merge/commit blocker.

### 2. [P1] Required archive, content-pack, and save-slot workflows have no product UI route

- **Severity:** P1
- **File:line:** `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:33-38`, `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:42-61`, `src/workshop/session.rs:25-38`, `src/workshop/session.rs:287-289`, `src/workshop/store.rs:306-337`, `src/app/client_runtime.rs:418-459`, `src/ui/workshop.rs:974-1109`
- **Description:** The accepted Share pillar and Two-System Forge steps 16-18 require slot save/quit/continue plus export and re-import of both a content pack and a complete Workshop archive. The session and store expose typed load/export/import and list/rename/archive operations, and `ClientRuntime` exposes catalog import/export, but the Workshop UI constructs only submit, playback, undo/redo, branch selection, and `RequestSave` actions. There is no caller that connects the archive bytes to a native file sink/source or browser download/upload, no catalog import/export control, and no in-product slot list/load/rename/archive surface. These paths are therefore headless APIs, not a playable acceptance journey.
- **Suggestion:** Obtain the documented UX/dependency decision for native file selection and browser upload/download, then add a slot manager and visible archive/catalog import/export controls through the existing typed action boundary. Cover native and browser byte sources/sinks, invalid-import preservation, slot lifecycle, focus restoration, and the complete save/quit/continue/export/re-import acceptance sequence.
- **Status:** Open — merge/commit blocker; archive file I/O requires an explicit design decision rather than an invented dependency.

### 3. [P1] Recovery discards the generation actually loaded and can treat a corrupt-head fallback as durably ready

- **Severity:** P1
- **File:line:** `src/workshop/store.rs:253-265`, `src/workshop/store.rs:1101-1123`, `src/workshop/session.rs:189-204`, `src/workshop/session.rs:237-248`, `src/workshop/session.rs:605-614`, `src/workshop/session.rs:645-669`, `src/app/client_runtime.rs:616-638`, `src/app.rs:1205-1237`
- **Description:** `LoadedSlot` deliberately distinguishes the generation whose bytes were read from the current CAS head. Both load paths correctly report that distinction when they fall back to an older generation, but both session installation paths discard `loaded.generation` and retain only `loaded.head_generation`. The startup path then calls `WorkshopSession::from_loaded`, which marks that corrupt-head generation as the resident slot and selected Continue generation while leaving the recovered session clean. Because `continue_ready()` consequently returns true, durable exit can terminate without committing the known-good recovered bytes. The corrupt head remains authoritative in storage, so the next startup repeats recovery instead of repairing it. The in-session load path likewise records the head as though it were the generation materialized in memory, obscuring which bytes the user actually accepted.
- **Suggestion:** Track `loaded.generation` and `head_generation` separately through decode and session state. A user-accepted fallback should remain a recovery state that is not `continue_ready` until the recovered archive is durably promoted with CAS against the current head and explicitly selected. Add an end-to-end test: corrupt authoritative head, recover the previous generation, accept/save/exit, reopen, and verify the slot loads cleanly without another recovery while preserving the expected digest and branch graph.
- **Status:** Open — merge/commit blocker.

### 4. [P1] Save retention can discard the known-good recovered predecessor in favor of an authority-invalid head

- **Severity:** P1
- **File:line:** `src/workshop/store.rs:689-700`, `src/workshop/store.rs:972-989`, `src/workshop/store/web.rs:415-475`, `src/workshop/store/web.rs:791-808`, `src/workshop/store/web.rs:817-835`, `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:411-427`
- **Description:** Native and IndexedDB commits select the retained predecessor by finding the first generation that passes storage-level checks. At this layer, `validate_archive` proves only the byte cap plus that the bytes parse as a JSON object; it does not perform the authoritative archive replay/digest validation required by the design. If the current head has intact length/hash metadata but is authority-invalid, a session can recover from the older authoritative-valid generation and then commit new valid bytes while the store retains the bad head as its sole fallback, dropping the known-good recovered generation. A later corruption of the new head then leaves no authoritative-valid fallback despite the requirement to preserve the last two valid generations.
- **Suggestion:** Carry the authoritative-valid loaded generation into the recovery-promotion request and retain that exact descriptor as the predecessor. Do not let storage-shape validation substitute for session-level archive validation. Add equivalent memory/native/IndexedDB tests with an integrity-valid JSON object whose archive replay/digest is invalid, followed by recovery, commit, corruption of the new head, and successful fallback to the known-good recovered generation.
- **Status:** Open — merge/commit blocker.

### 5. [P2] Capacity truncation makes tail entities, branches, and redo choices unreachable by keyboard or pointer

- **Severity:** P2
- **File:line:** `src/ui/platform.rs:91-103`, `src/ui/platform.rs:391-438`, `src/app.rs:849-869`, `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:211-230`, `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:458-458`
- **Description:** The platform frame silently takes only 14 outliner entries and four branches, and stops timeline controls when the remaining width is insufficient. `focus_order()` is derived only from those instantiated platform controls and replaces the application's base focus order. At declared caps (64 systems, 512 worlds, 64 branches, and potentially multiple redo children), entities and branch/redo actions beyond these fixed slices have no pointer target and cannot receive keyboard focus. The richer semantic model does not cure this: action lookup is also limited to `frame.controls`. This violates the requirement that every Workshop action remain available without spatial interaction and that keyboard focus reach outliner, timeline, and branch controls.
- **Suggestion:** Add a single-source virtualized/scrollable, searchable, or explicitly paginated control model for outliner, branch chooser, and redo children. Focus/action resolution must address off-screen logical controls and scroll them into view. Add cap-level keyboard and pointer tests that select a tail entity, the 64th branch, and every redo child.
- **Status:** Open — required before release acceptance.

### 6. [P2] The derived and drawn Workshop scene omits stars as visual instances

- **Severity:** P2
- **File:line:** `src/presentation/workshop.rs:109-120`, `src/presentation/workshop.rs:127-171`, `src/ui/platform.rs:641-690`, `docs/superpowers/plans/2026-09-02-nyon-galaxy-workshop-client.md:352-371`
- **Description:** The client plan requires `WorkshopSceneFrame` to contain sorted systems, stars, worlds, lanes, and shipments. The implemented frame has no star instance collection. Star extraction creates only a semantic entry, and the scene drawing loop renders lanes, systems, worlds, and shipments. Therefore a created or selected star has no visual object or visual selection decoration even after the opaque-overlay defect is fixed.
- **Suggestion:** Add a stable sorted star instance representation, distinct star rendering/selection decoration, and GPU/primitive layout coverage as appropriate. Add a presentation test that creates and selects a star and asserts both visual instance output and semantic bounds without mutating authority.
- **Status:** Open — required before the scene-extraction task can be considered complete.

### 7. [P2] The sighted Workshop frame drops nearly all inspector and simulation-observation content

- **Severity:** P2
- **File:line:** `src/ui/platform.rs:360-598`, `src/ui/platform.rs:105-173`, `docs/superpowers/plans/2026-09-02-nyon-galaxy-workshop-client.md:316-329`
- **Description:** `WorkshopUiModel` builds inspector sections and detailed production/inventory/route/shipment/deposit/hazard information, but `build_workshop_platform_frame` does not turn those rows into visible panel content. It reduces the selected object to one status line and at most three inventory row summaries. `PlatformUiFrame::draw` then renders only the title, first four status lines, and controls; control descriptions and the remainder of the inspector are semantic-only. A sighted player cannot inspect why resources moved or production changed, satisfying neither the Observe pillar nor the client plan's inspector/status checklist.
- **Suggestion:** Render a dedicated scrollable inspector/status panel from the same model used for semantics, including fields, dependencies, production, inventory, routes, shipments, deposits, and hazards. Add snapshot/layout tests for representative entity kinds and a live Two-System Forge observation check that explains an alloy production change and an ion-storm logistics effect.
- **Status:** Open — required before release acceptance.

### 8. [P2] Asynchronous catalog import silently drops a failed requested Workshop transition

- **Severity:** P2
- **File:line:** `src/app/client_runtime.rs:430-435`, `src/app/client_runtime.rs:505-541`, `src/app/client_runtime.rs:550-568`
- **Description:** `begin_new_workshop_from_catalog` is asynchronous. After pack persistence completes, `poll_catalog_import` calls `install_new_workshop` with `let _ =`, discarding the result. The replaceability predicate is necessarily re-evaluated at completion; if the resident Workshop became dirty, queued work, or entered persistence work while the pack job was pending, installation can fail. The imported catalog is retained, but the requested new Workshop is silently not created and no diagnostic or caller-visible completion result explains the state. This is an error-handling gap across an async state transition.
- **Suggestion:** Represent catalog import completion and requested Workshop installation as explicit success/failure states, surface `install_new_workshop` rejection as a bounded diagnostic or recoverable result, and keep retry/cancel intent visible. Add a test that dirties the resident Workshop while `PutPack` is pending and asserts a deterministic user-visible outcome without losing either session or catalog.
- **Status:** Open.

### 9. [P2] The baseline concentrates unrelated responsibilities in several 1,300-2,100-line modules

- **Severity:** P2
- **File:line:** `src/ui/workshop.rs:1`, `src/app.rs:1`, `src/workshop/store.rs:1`, `src/workshop/store/web.rs:1`
- **Description:** The current baseline places 2,144 lines of model construction, creator semantics, outliner/inspector/timeline assembly, and action mapping in `src/ui/workshop.rs`; 1,758 lines of lifecycle, input, shell, Workshop frame assembly, semantics, GPU handling, and durable exit in `src/app.rs`; 1,452 lines of public persistence contracts plus memory/native implementations in `src/workshop/store.rs`; and 1,345 lines of IndexedDB schema, transactions, serialization, and adapter logic in `src/workshop/store/web.rs`. The size and mixed responsibilities make critical ordering and recovery invariants difficult to review and were directly relevant to Findings 1, 3, 4, 5, and 7.
- **Suggestion:** Before accepting the baseline, split along already-present ownership seams: Workshop UI model/outliner/inspector/timeline/semantics; app shell/frame/actions/accessibility/durable-exit; store contract/memory/native; and browser schema/transactions/serialization. Preserve public paths with narrow re-exports and move tests with their owning modules. Do not use extraction merely to hide line count; each resulting module should own one coherent invariant.
- **Status:** Open — maintainability blocker under the repository review standard unless explicitly justified and bounded.

## Commit recommendation

Do **not** stage or commit the current mixed Workshop V1 baseline as an implemented or completed product. Repair Findings 1-5 first, resolve the file-I/O UX decision in Finding 2, and rerun the workspace, pure-core wasm, root wasm, release, dual-artifact, and diff gates. Then independently exercise the complete Two-System Forge journey through both keyboard-only and pointer-only paths, including corrupt-head recovery and archive/catalog round trips. Findings 6-9 should be closed before claiming the scene, Observe experience, async catalog workflow, or maintainability review complete. Any later commit should include only the reviewed Workshop slice and must exclude unrelated concurrent Living Galaxy planning work unless that work receives its own authority and review.
