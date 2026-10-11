## Design Document Review: Strategic cubic planets — architecture proposal

### Summary

**APPROVED for architecture-section presentation**, with two nonblocking Minor notes below. The separate variable-capacity depth renderer, selected-world first delivery, immutable authority boundary and retained full product scope form a coherent basis for writing the formal Strategic specification after human architecture approval; this review is neither that human approval nor specification, plan, implementation or runtime acceptance.

Reviewed 2026-10-09 in canonical `/Users/donaldfilimon/dev/active/NYON`, shared dirty `main`. The proposal, complete frozen source inventory and writer report were read. Production source contracts were inspected narrowly as listed below. No Critical or Important findings were established. Minor findings remain open and are preserved here as the original independent review.

### Issue 1: Carry forward the entire LG acceptance ledger explicitly

- **Severity**: minor — Minor, nonblocking architecture clarification
- **Section**: §7 Acceptance, risks and remaining gates; proposal lines 123–133, especially 130
- **Description**: The acceptance table explicitly retains “All 18 Living release and 18 Workshop Forge steps plus seven experiments,” but never explicitly retains **all LG-01–LG-27**. These are distinct obligations: `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-qualification.md:18–47` includes canonical wire identity, running creator queue, documentation/delivery, audio isolation and cross-authority lifecycle as individually accountable rows. The proposal retains many of their behaviors and references the full inventory, so this is not evidence of deliberate scope removal. Its compact list nevertheless risks becoming a smaller replacement checklist in the next artifact.
- **Suggestion**: State explicitly that all LG-01–LG-27 remain required alongside the manual journeys, 46 creator tools, experiments and host matrix. Have the formal spec/plan map Strategic witnesses to the affected rows while preserving the untouched rows and their independent evidence. No implementation-level test enumeration is required for architecture approval.
- **Status**: open

### Issue 2: Reserve the list and recovery path promised on arena exhaustion

- **Severity**: minor — Minor, nonblocking specification follow-through
- **Section**: §6 Mesh, shader and bounded execution contract; proposal lines 114–119
- **Description**: Line 115 gives the fact/layout/hit/semantic arena an 8 MiB bound, then promises “Exhaustion removes decoration first, then reports detail unavailable with complete list access.” The same paragraph puts virtualized rows and selected detail within the packet resource discussion. The architectural fallback is sound, but the current wording does not establish that list navigation, error text and recovery semantics have reserved capacity when preparation has exhausted that arena. This is an allocation-priority detail to bind in the formal spec, not evidence that the proposed numeric budget is infeasible.
- **Suggestion**: Make essential list/Inspector/recovery capacity reserved or independently admitted before optional scene/detail work; require exhaustion to keep a usable bounded page and navigation/error actions. Include forced arena-exhaustion witnesses in the formal spec. Keep the numeric limits Proposed until measured; do not raise the bound merely to hide a failure.
- **Status**: open

### Strengths

- The options are materially different, with a clear recommendation and credible cost explanation (§1, lines 10–18). A complete selected-world first slice is explicitly staged against Galaxy/System/search/presets/Continue/demos/settings obligations, rather than being represented as Task 7 completion (§§1, 7).
- Authority and presentation remain separate: domain/full entity keys, separate V2/V3 engines/catalogs/codecs/archives, no conversion/default filling, detached creator validation and explicit Apply (§2, lines 24–54). Surface placement is honestly labeled as derived, including gas-world orbital presentation; no face ownership, physical gravity, mining or editable terrain rules are invented.
- The six face frames are concrete and consistently right-handed: each listed right × up equals its outward normal. Quaternion orbit removes the current pole restriction; face snaps, visible buttons, keyboard/semantic routes, reduced motion, retained framing, deletion recovery and hidden-face reveal are covered (§3, lines 60–79).
- Picking uses the displayed immutable packet, CPU/backend depth agreement, foreground UI precedence, front-surface proxy visibility and deterministic face/key edge ties. Collision lists and full-key search preserve reachability without x-ray input. Stable anchors exclude collection order, owner, status, tick and branch; deletions and history do not shuffle survivors (§§3–4).
- Separate mesh/instance ABI and variable capacity preserve Classic's fixed-seven and 36-byte primitive contracts. The proposal explicitly requires a Strategic shader corpus/profile/layout review beyond the existing seven-family/twenty-render-entrypoint compiler scope, with all six target witnesses and mandatory native GL/WebGL2 Low (§6, lines 103–107).
- Packet, geometry, GPU, upload, work and retirement bounds are explicit Proposed values. Admission/backpressure, safe device epochs, hidden/no-session retirement, truthful quality fallback and failure recovery are architectural requirements rather than unmeasured performance claims (§6).
- Per-document presentation records, capped resident caches, CAS conflict/error handling, absent-only legacy fallback and recovery of unsupported data are separate from authority/portable archives and exact Continue (§5). Formal schema/encoding remains appropriately deferred to the written specification.
- Existing content identity and authentic receipt timelines are retained. New size profiles are versioned, independently authored per authority, validated and expressly Proposed; the existing generator/default-colony discrepancy is surfaced rather than silently rebalanced (§5, lines 95–99).
- Acceptance retains all 46 creator tools, history/reopen behavior, named hosts/backends/accessibility, actual artifact witnesses, capacity/performance/endurance/offline obligations and missing-host Pending status (§7). The independent V3 corpus/amendment/freeze and host 6L/6M dependencies remain acyclic and separately gated.

### Verification and evidence limits

**Current source-confirmed:**

- `crates/nyon-workshop-core/src/living/model.rs:53–86,313–329`: declared capacities and nonspatial `LivingWorldV2` fields match the proposal. No surface coordinates or face mechanics exist in that struct.
- `src/living_navigation.rs:62–79`: selection carries entity kind/full V2 ID and rows carry parent/world links.
- `src/living_camera.rs:7–60`: Galaxy/System/World scopes, current yaw ±45°, pitch 35°..70° and zoom 0.25..8 establish the actual camera limitation and existing zoom bound.
- `src/presentation/living.rs:1–2,35–106,258–292,1140–1155`: current projection excludes geometry from authority, has variable identity-bearing markers, uses identity-derived layout and draws world discs.
- `src/engine/render_frame.rs:9–25`, `src/engine/render.rs:185–213`, `src/engine/scene_renderer.rs:374–378`: the Classic scene seam, UI composition and exact seven-world guard are present. The proposal does not pretend the existing seam already supports Strategic meshes.
- `src/living_content.rs:6–126`: seven existing content variants, dedicated First Expansion, 12/24/4 Seeded, 4/8/3 curated state and validated generator version 1 manifest construction match the inventory.
- `src/preferences/mod.rs:52–59,170–203`, `src/sandbox_session.rs:380–397`: current preferences omit durable camera/audio fields, while the Session owns those presentation settings. Expanded restart durability is a proposed product, not inferred from these fields.
- `docs/qualification/2026-10-08-living-completion-ledger.md:284–290`: human Strategic experience direction is ledger-recorded; architecture/spec/plan approval remains pending. Original human transcripts were not supplied. Walking/gravity/mining/voxel deferral remains explicitly controller-relayed from the older source audit, not independently established human approval.
- `docs/superpowers/specs/2026-10-09-nyon-zig-platform-bindings-design.md:7–13,34–62,182–239,370–421`: platform approval is for planning, reviewed pure inputs feed host integration, shader scope is closed, and storage identities/durability/CAS remain protected. This approval does not cover the new Strategic shader corpus or presentation store schema automatically.

**Frozen identities:**

| Artifact | SHA256 |
| --- | --- |
| Reviewed proposal | `74556ab55576845592ea8c88ce1c14332f57693ee4cf2589b08e27cda975d002` |
| Frozen source inventory | `5a1d52be115333eed869615e0857c5f91969332d721647ecae767fd1acc192cd` |

The SHA256 command exited 0 and matched both assigned artifacts. A read-only identity script exited 0: all **19** source size/hash entries in the proposal writer report still matched; **26 of 27** entries in the older source inventory matched. `docs/qualification/2026-10-09-zig-migration.md` has concurrent evidence-document drift from its frozen inventory hash `adb9fdbd7b2070ed407935d84276c6a2ec733af01ca7ee3c36216d0df503d88b`. The parent reports an appended Task 3C scoped receipt. This report preserves the original inventory identity and does not represent it as refreshed. All proposal writer-report sources, including the Strategic direction ledger and platform design, matched their frozen bytes.

**Untested / outside this review:** no build, repository gate, runtime, renderer, shader compilation, performance/endurance, native/browser durability, hardware or live accessibility check was run. The assigned scope expressly excludes them. Face-frame arithmetic and source inspection establish design consistency only. External platform specifications were not independently browsed or reverified; backend/OS/browser behavior and SDK/tool availability remain the proposal's future qualification obligations. No source, formal spec, implementation plan, dependencies, assets, commits, publication or other agent work was changed. Only this assigned review report was created.

Memory was used only to route prior Strategic-first context (`MEMORY.md:58–65`); the current source ledger and inspected contracts determine this verdict.
