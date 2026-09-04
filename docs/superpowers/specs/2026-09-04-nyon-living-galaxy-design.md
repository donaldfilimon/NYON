# NYON Living Galaxy: product and architecture design

Date: 2026-09-04

Status: Approved for implementation by the user's explicit NYON Living Galaxy Completion Plan request on 2026-09-04; implementation and qualification remain pending

Product direction: Full Living Galaxy approved in conversation; execute through the dependency-ordered program plan and its independent review gates

Canonical checkout: `/Users/donaldfilimon/Public/NYON`

## 1. Product promise

NYON is a creative galaxy sandbox with living rival civilizations, presented as a stylized cosmic diorama. The player shapes worlds, observes autonomous economies and politics, intervenes, and compares alternate histories. The central loop is **create, observe, understand, intervene, compare**.

Economy, expansion, diplomacy, and fleet conflict share one simulation. They are not separate minigames or menu modes. No conquest threshold, civilization collapse, or empty galaxy ends the creator's session. The creator has unrestricted powers, subject to validation and capacity limits. Civilizations spend real resources and obey the same published rules.

The default does not include a resource-constrained player faction. A playable-emperor mode, population needs, ecological simulation, research trees, tactical fleet piloting, multiplayer, accounts, cloud services, and model-based narration are outside this release. These exclusions do not remove any of the four approved civilization systems.

### Direction decisions

| Decision | Chosen direction |
| --- | --- |
| Product structure | One new unified sandbox; legacy modes remain compatibility entry points only |
| Main experience | Creative galaxy sandbox, not a win-driven empire campaign |
| Autonomous actors | Living civilizations with economy, expansion, diplomacy, and conflict |
| Presentation | Stylized cosmic diorama, not photorealism or free-flight full 3D |
| Creator authority | Unrestricted editing, no resource budget; proposed default carried forward |
| Simulation | Offline, local, deterministic integer authority |
| Progression | Learning and optional experiments, not locked creator capabilities |
| Completion | Verified whole product, not a good-looking screenshot or a green subset of tests |

## 2. Approach and alternatives

**Recommended: evolve the existing client and add a versioned Living Galaxy authority.** Reuse the existing native/web shell, SDF text infrastructure, validated content concepts, and asynchronous storage patterns. Keep V1 interpretation intact. Add isolated V2 state, commands, replay, and civilization rules within the existing pure-core crate. Add a variable-capacity diorama renderer rather than forcing the seven-world campaign renderer to handle a different game.

**Alternative: extend WorkshopV1 in place.** Fewer initial types, but it would reinterpret archived histories, conflate passive ownership with autonomous civilizations, and invalidate old digests. Rejected.

**Alternative: replace the engine or rebuild as an unrestricted full-3D game.** Potentially broad rendering capabilities, but loses the existing cross-platform and deterministic foundation and changes the selected diorama direction. Rejected for this program.

This is a multi-subsystem program. The documents below define the complete release while allowing independently reviewed implementation slices. Finishing a slice does not finish the goal.

- [Civilization rules and strategic interactions](2026-09-04-nyon-living-galaxy-rules.md)
- [Player experience, interface, visual and audio design](2026-09-04-nyon-living-galaxy-experience.md)
- [Verification, acceptance journeys, and completion ledger](2026-09-04-nyon-living-galaxy-qualification.md)

## 3. Current evidence and the first repair

Read-only inspection on 2026-09-04 found the canonical checkout on `main` at `4bc236a`, with substantial preexisting modified and untracked work and no configured remote. These observations are not ownership permission for those changes.

The native app first showed a populated Workshop hierarchy with a visually blank center and incomplete visible inspector; a later observation showed the main menu. No navigation or gameplay commands were issued during those observations.

Source explains a concrete compositing defect: `build_workshop_frame` puts the world scene in the primitive underlay, while `PlatformUiFrame::draw` puts a full-window background with alpha 0.96 into the overlay. The renderer draws that overlay after the underlay. High contrast uses a fully opaque background. This establishes an occlusion path; a repaired live artifact must still demonstrate the visible result.

Other grounded gaps:

- Workshop uses bitmap primitive text despite existing bundled Inter SDF font assets and a dedicated UI renderer.
- Its visual outliner takes at most 14 entries, branch controls take at most four, and the timeline drops controls when horizontal space runs out.
- Its inspector has rich semantic sections but the ordinary visual path only exposes a small inventory summary.
- The campaign scene renderer requires exactly seven worlds and is not a variable-size Workshop renderer.
- WorkshopV1 factions have names and colors but no colony, diplomacy, fleet, or autonomous decision state.
- Workshop save/history and archive operations exist below the UI; portable import/export and slot-management controls are incomplete.
- Classic's manual and guide incorrectly describe fleet launch as consuming defense. The source consumes half current energy. Correct the communication without changing Classic's rules.

Evidence entry points: `src/app.rs`, `src/ui/platform.rs`, `src/engine/render.rs`, `src/engine/scene_renderer.rs`, `src/ui/workshop.rs`, `crates/nyon-workshop-core/src/{model,simulation,archive}.rs`, and `docs/PLAYER-MANUAL.md`. These are evidence locations, not instructions to rewrite all those files.

## 4. Architecture and public boundaries

Keep the current two-member workspace. New pure authority lives under a `living` module in `nyon-workshop-core`; do not move or trait-generalize the frozen V1 implementations just to make the new code fit.

### Authority

The new public boundary consists of `LivingGalaxyStateV2`, `LivingRulesV2`, `LivingCommandV2`, `LivingCommandEnvelopeV2`, `LivingTickReceiptV2`, `LivingHistoryV2`, and version-specific validated catalog/archive types. The rules companion defines their behavior. Names are proposed interfaces, not existing APIs.

State includes topology, resources, construction, colonies, civilization policies and intelligence, relations and agreements, fleets, freight, scheduled hazards, and the counters needed for deterministic replay. Ordered collections, checked arithmetic, bounded inputs, explicit phase order, and atomic candidate-state commit remain mandatory.

Creator actions and autonomous intents have distinct provenance. UI input is validated into recorded creator commands. Autonomous decisions are computed in the pure authority from a stable phase snapshot, not from UI advisory output. Both must pass the same structural and capacity constraints; only civilization actions spend resources.

### Session and presentation

The client session owns pacing, UI drafts, loading jobs, selected objects, camera state, event filters, and requested history navigation. Those values do not enter the simulation digest. A paused simulation cannot accumulate catch-up time while help, a file picker, or a modal is open.

The one-way flow is:

```text
Validated creator commands + deterministic civilization decisions
                         |
                         v
                Living Galaxy authority
                         |
              immutable snapshot + receipts
                  /                 \
                 v                   v
       inspector / chronicle     diorama scene frame
                 |                   |
                 v                   v
        shared semantic UI       backend rendering
```

Use a separate `LivingSceneFrame` for arbitrary supported entity counts. Extend the client render selection through an explicit scene variant; retain Classic's existing `SceneFrame` path and tests. New render structures carry size/offset/stride assertions and shaders are validated before pipeline creation.

### UI and file transfer

One measured layout model drives drawn controls, hit testing, focus order, native semantics, and the web semantic mirror. Text and information must not exist solely in accessibility metadata or solely on the canvas.

Portable transfer is an explicit client service: choose an import source, read bounded bytes, validate, preview, and commit to a new slot; choose an export destination and confirm completion. Native uses OS file dialogs, browser uses a user-initiated file input/download. Cancellation is a normal result. Never require terminal commands or clipboard-sized JSON for ordinary saving and sharing.

### Persistence adapters

Add a version-specific `LivingStoreV2` surface with `LivingStoreRequestV2`, `LivingStoreResultV2`, and V2-only slot, generation, catalog-hash, and archive descriptors. It owns no Workshop types and invokes only Living V2 validators. The implementation may share private bounded-job, atomic-generation, file-transaction, and IndexedDB helper code, but must not widen or route through the public `WorkshopStore` API.

The asynchronous contract is `begin(owner_epoch, request) -> job_id`, `poll(owner_epoch, job_id, work_budget) -> Pending(progress) | Ready(result) | Failed(error)`, and `cancel(owner_epoch, job_id) -> Cancelled | CommitAlreadyWon(result)`. Job IDs belong to exactly one adapter and client-session epoch; cross-owner polls/cancels reject. Begin copies and preflights bounded input but publishes no mutation. A poll processes at most 1024 declared work units. Cancellation before the named commit point leaves storage unchanged. Once durable commit wins, cancellation reports the committed result rather than claiming cancellation. Dropping or replacing a client session cancels its uncommitted jobs; storage recovery decides any interrupted transaction whose commit outcome was unknown.

| V2 request | Validation and result | Durable commit point |
| --- | --- | --- |
| `ListSlots`, `ListPacks` | Bounded descriptors only; corrupt entries return per-item typed diagnostics | Read-only |
| `LoadSlot{id}` | Select valid generation, resolve exact pack, validate/replay archive; return authority and descriptor | Read-only |
| `CreateSlot{name,archive}` | Validate name, V2 archive/pack reference, capacity, replay; return slot/generation | Archive generation and manifest/IndexedDB transaction together |
| `SaveSlot{id,expected_generation,archive}` | Compare generation, validate/replay, retain prior valid generation | New generation and manifest/IndexedDB transaction together |
| `RenameSlot` / `SetArchived` | Compare generation and validate lifecycle transition | Updated manifest/metadata transaction |
| `DeleteArchivedSlot` | Compare generation; require archived/nonactive at the client boundary; return deletion receipt | Slot generations and manifest reference durably removed |
| `PutPack{canonical_bytes}` | V2 kind/size/canonical/hash/schema validation, content-hash deduplication and capacity; return hash/metadata generation | Pack bytes and manifest/IndexedDB transaction together |
| `SetPackArchived{hash,expected_generation,archived}` | Compare pack-metadata generation; reject archive while referenced by an unarchived galaxy | Updated manifest/metadata transaction and new pack generation |
| `ReadPack{hash}` / `ReadArchive{id,generation}` | Return bounded canonical bytes and ordinary file hash for export | Read-only |
| `DeletePack{hash,expected_generation}` | Compare pack generation; require archived and zero references across all retained slots | Pack bytes and manifest/IndexedDB reference durably removed |

The typed error set distinguishes `NotFound`, `StaleGeneration`, `Limit`, `TooLarge`, `WrongAuthority`, `NonCanonical`, `Integrity`, `MissingCatalog`, `Referenced`, `InvalidLifecycle`, `Cancelled`, `CommitAlreadyWon`, `Storage`, and `CorruptRecovery`. Import preview and export destination handoff remain client workflows composed from these operations; they are not hidden store transactions. Native durability requires the transaction/rename and manifest durability steps. Browser durability ends at successful IndexedDB read-write commit, not a later download.

Living V2 supports at most 64 slots and 256 registered packs, with the 32 MiB archive and 2 MiB pack byte limits defined by the rules companion. WorkshopV1 remains frozen at its existing 16 slots, 256 packs, 16 MiB archive, and 1 MiB pack limits. Its trait, request/result types, validators, manifest wire kind/version, `workshop-v1` path, and `nyon.workshop.v1` database identity remain unchanged. The product-level import router performs a bounded top-level kind/version preflight and submits bytes only to the matching authority adapter. `LivingStoreV2` also rejects V1 kinds as `WrongAuthority`; the existing Workshop store and validators are not changed to recognize V2. If foreign bounded bytes reach that lower-level legacy API, its unchanged Workshop codec remains responsible for rejection during load and cannot mutate Workshop authority state. No V2 operation writes V1 storage, and loss of one store does not make the other store or Classic entry point unavailable.

## 5. Versioning, saves, and history

New galaxies use Living Galaxy rules version 2, versioned canonical catalog/archive envelopes, and distinct hash domains. Keep Classic RulesV1 and WorkshopV1 decoding, encoding, and replay unchanged. Do not merely bump the existing global V1 constant.

The Library identifies the version of every saved item. Existing Workshop archives open in their original authority; existing Classic scenario saves remain scenario saves, not resumable battles. The main new-game route creates Living Galaxy sessions. Legacy entry points live under Library / Legacy and do not compete with the main product flow.

No automatic legacy conversion, overwrite, or copy-on-load occurs. This release does not promise V1-to-V2 conversion. A future explicit conversion must create a new artifact and preserve source provenance, rather than replay old commands under new rules.

V2 uses the `living-v2` subdirectory inside the existing platform NYON application-data root and the browser IndexedDB database `nyon.living.v2`, separate from V1. Saves retain atomic replacement, generation conflict detection, two valid recovery generations, and explicit Continue selection. A failed load/import cannot replace a valid in-memory session. A failed save keeps it dirty and available.

One small version-independent **Library coordinator** is the sole authority for global Continue selection. Native stores its two recoverable generations as `library-selection-v1.a` and `library-selection-v1.b` plus an atomic head marker in the application-data root. Browser uses a separate `nyon.library.v1` IndexedDB database with `generation-a`, `generation-b`, and `head` records updated in one read-write transaction. Each generation is independently canonical and integrity-checked. Readers try the head, then choose the highest valid prior generation; compare-and-swap uses that recovered generation number. Thus both platforms retain exactly two valid recovery generations.

The coordinator payload contains `kind=NYON_LIBRARY_SELECTION`, format version 1, monotonically increasing generation, authority discriminator (`living_v2` or `workshop_v1`), slot ID, branch ID when the authority supports branches, and the last confirmed revision/tick/digest used to diagnose drift. It contains no galaxy payload. A cleared selection is a valid new generation with `authority`, slot, branch, revision, tick, and digest all `null`, not deletion of the coordinator.

At the first V2-capable launch, an absent coordinator is initialized from the valid WorkshopV1 selected slot, if one exists; otherwise Continue remains unset. Once initialized, authority-local selected fields may support their own legacy library UI but never decide global Continue. Successfully opening a WorkshopV1 or Living V2 galaxy explicitly changes global Continue after the item is loaded and its selection is durably committed; opening a non-resumable Classic scenario does not. Opening or importing a galaxy first durably commits the target authority's slot, then compare-and-swaps the coordinator. There is deliberately no cross-store transaction: coordinator failure leaves a recoverable unselected target slot and preserves the prior active session and Continue record.

Startup reads only the coordinator, then initializes the selected adapter and validates its named authority/slot/branch. Success opens exactly that item. An unavailable store, archived or missing slot, missing catalog, corrupt archive, stale generation, branch mismatch, or digest mismatch opens Library recovery with the exact recorded reference and typed error. Other authority adapters and Classic remain reachable, but startup never chooses one automatically. The coordinator's own invalid newest generation falls back only to its prior valid generation; it does not consult authority-local recency. It never chooses arbitrarily between two authority-local selections or silently falls back to a different save.

Creator interventions create immutable revisions. Pure simulation ticks and deterministic autonomous decisions are replayed, not stored as millions of fabricated creator edits. Every archive embeds one validated canonical genesis manifest plus its seed and generator provenance. A procedural generator or curated starter constructs that manifest only when creating a galaxy; replay never depends on rerunning generator code. The root branch begins at that manifest, before creator revision zero.

Checkpoints accelerate navigation but never become an alternate source of truth and are excluded from canonical exported archives. A local derived checkpoint is keyed by authority/rules/catalog, archive prefix identity, branch/revision/tick, and state digest; any mismatch discards it and replays from genesis. Bookmarks identify an exact branch, revision cursor, and tick. Editing a historical state creates a sibling branch and never erases the original future.

Comparison materializes two paused branch states at the same requested tick under their own commands. It reports measured differences, not a claim that a single run proves causality. If either branch cannot be evaluated within the bounded replay budget, show progress/cancellation rather than freezing the UI or silently comparing different ticks.

## 6. Delivery sequence

1. **Foundation and readable diorama.** Establish ownership of existing changes; repair scene compositing; implement shared responsive SDF layout, camera framing, complete visible inspector and reachable hierarchy/history controls. Qualify with current Workshop fixtures before introducing V2.
2. **V2 authority and persistence.** Add versioned models, commands, catalogs, deterministic phase order, replay, separate storage, import/export, and compatibility fixtures.
3. **Economy and autonomous expansion.** Add colonies, construction, explicit recovery production, intelligence, fleets, settlement, and explainable economic/expansion decisions. Ship a complete observable loop.
4. **Diplomacy and conflict.** Add agreements, bilateral relationship reasons, bounded wars/truces, simultaneous combat, occupation, and civilization dormancy without ending the sandbox.
5. **Experience and expressive presentation.** Integrate the curated starter, contextual creation, chronicle, experiments, history comparison, stylized worlds/settlements, route/fleet presentation, and bounded audio.
6. **Whole-product qualification.** Run the requirements matrix, full journeys, migration-protection fixtures, accessibility, recovery, native/browser backends, and performance checks. Repair failures before release claims.

The initial UI slice is necessary but not a substitute for steps 2-6. No placeholder diplomacy buttons, nonfunctional shipyards, scripted fake AI events, or screenshots of unreachable views count as delivery.

## 7. Working-tree and review boundary

Before implementation, inventory exact dirty paths and ask the owner of overlapping active work to establish a stable baseline. Preserve untracked files. Do not reset, clean, relocate, or package unrelated work. Stay on canonical `main` unless isolation is explicitly needed and agreed.

This design package may be committed independently using only its exact paths. It does not authorize staging the existing mixed code diff, changing the old accepted design's status, updating RulesV1 goldens, or claiming the current program is complete.

The user approved implementation of this package through the NYON Living Galaxy Completion Plan on 2026-09-04. The dependency-ordered child plans already exist; reconcile them against the current baseline and execute with concrete tests before code changes. Approval does not turn historical observations into current evidence. The requested outcome remains the verified whole Living Galaxy product.
