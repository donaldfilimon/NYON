# NYON Living Galaxy: qualification and completion

Date: 2026-09-04

Status: Approved acceptance contract on 2026-09-04; no implementation or runtime pass implied

Parent: [Product and architecture](2026-09-04-nyon-living-galaxy-design.md)

## 1. Completion means the whole product

The requested outcome is a working creative galaxy sandbox with coherent UI/UX, a stylized cosmic diorama, autonomous economy and expansion, diplomacy and conflict, unrestricted creator intervention, immutable history, durable saves, and no mandatory victory. Passing the initial rendering repair or a narrow test suite does not satisfy that outcome.

Every result records the source revision and any scoped dirty diff, artifact hash, test/fixture identity, platform/backend, actual outcome, and evidence location. Separate proposed tests from executed tests. Never replace an old deterministic golden merely because a new implementation disagrees with it.

## 2. Requirements and evidence ledger

At design creation, every delivery row below is **pending**. Existing infrastructure is a starting point, not proof of the redesigned product.

| ID | Requirement | Authoritative completion evidence |
| --- | --- | --- |
| LG-01 | Visible diorama, correct compositing | New artifact shows populated worlds/routes under ordinary and high-contrast UI; compositor regression |
| LG-02 | Readable responsive interface | Screenshots and input checks at all declared sizes/scales; no lost controls or overlap |
| LG-03 | Complete visible inspector | Content parity against model plus live selection of world, industry, civilization, route, fleet, hazard |
| LG-04 | Searchable reachable hierarchy/history | First/last/deepest item, filtered reveal, more than four branches, virtualization tests |
| LG-05 | Conventional form editing | Caret/selection/paste/validation/cancel/focus tests and live keyboard journey |
| LG-06 | Galaxy/system/world navigation | Correct fit/focus/pan/zoom, no autorandom reframing, removal recovery |
| LG-07 | V2 deterministic authority | Fixed fixtures, tick-order tests, insertion-order invariance, replay and platform digest parity |
| LG-08 | Real resource economy | Atomic recipes, storage caps, queues, reserve-preserving freight, scarcity and recovery fixtures |
| LG-09 | Autonomous construction/expansion | Civilizations act without creator commands; actual receipts, costs, settlement and intelligence fixtures |
| LG-10 | Diplomacy | Bilateral reasons, agreements, expiry, trade eligibility, war/truce boundaries |
| LG-11 | Fleet conflict/occupation | Simultaneous combat, retreat, contested capture, resource/asset preservation and dormancy |
| LG-12 | Continuous creator sandbox | Zero/one/many active civilizations keep stepping; no mandatory terminal victory |
| LG-13 | Honest events and strategy explanations | Displayed stories map to actual receipts; actor/creator provenance distinguished |
| LG-14 | History and what-if comparison | Undo/redo digest identity, sibling futures retained, matching-tick comparison |
| LG-15 | Durable save/Continue/recovery | Quit/reopen exact slot/branch/tick/digest, generation conflicts, interrupted write recovery |
| LG-16 | Portable import/export | UI-created artifact validated and reimported into a new slot; source unchanged |
| LG-17 | Legacy preservation | Classic scenario and WorkshopV1 fixture bytes, replay, goldens, and primary-slot precedence unchanged |
| LG-18 | Accessible operation | Pointer-only, keyboard-only, native/browser screen reader, contrast/motion/mute equivalence |
| LG-19 | Graphics/backend resilience | Live native/web backend matrix, low-quality path, acquisition failure and recovery |
| LG-20 | Performance/offline operation | Named-device measurements at representative and capacity fixtures; network-disabled full journey |
| LG-21 | Starter/content/experiments | Validated starter and branches produce real expected events; notebook records evidence |
| LG-22 | Audio and presentation isolation | Rate limits/mute/no-device behavior; visual/audio/settings changes do not affect state digest |
| LG-23 | Documentation and delivery | Current manual matches executed behavior; exact reviewed changes committed; remaining limitations stated |
| LG-24 | Unrestricted creator capability | Every supported creator command is reachable, free to the creator, validated/atomic, provenance-labeled, and keyboard/semantics qualified |
| LG-25 | Cross-authority Continue and lifecycle | One durable coordinator selects exactly one V1/V2 authority/slot/branch; migration, CAS failure, invalid-target recovery and capacity-reclamation fixtures |
| LG-26 | Canonical V2 wire identity | Shared full-byte golden corpus proves pack/archive kinds, strict encoding, hash domains, IDs, digests and native/web parity |
| LG-27 | Running creator queue | Tail-aware stale rejection, projected validation, same-boundary revision parent chain and all-or-nothing application fixtures |

## 3. Deterministic and subsystem tests

### Authority fixtures

Execute all exact examples in the [rules document](2026-09-04-nyon-living-galaxy-rules.md), including boundaries immediately before and at deadlines. Add invalid-input/capacity fixtures for every new command and imported record.

- Run identical seed, catalog, creator commands, and tick count through different render cadences and 1x/4x/20x pacing. Compare canonical state and receipt digests.
- Reverse input collection insertion order without changing logical command order. Results must remain identical.
- Save at tick 18000, load, and continue to 36000; compare uninterrupted simulation. Include construction, intel, treaties, war, occupation counters, in-flight objects, and decision cadence.
- Apply an invalid batch after a valid earlier batch; reject it without partial resource, ID, history, or queue changes.
- Queue two running creator envelopes against successive published pending-tail values. At the next boundary, prove their immutable revisions form the same ordered parent chain and digest on native and web. Reject a duplicate-tail or old-tick submission without changing the queue. Inject a replay invariant fault and prove the tick and all queued revisions remain uncommitted.
- Queue a valid command, then submit a projection-conflicting command: only the second rejects and the first remains queued. Submit an invalid command to an empty queue, then a valid command with the unchanged tail: the valid command accepts. After committing two valid same-boundary revisions, Undo and Redo each cursor independently and reproduce its exact parent/digest. Pin their application tick and never-reused document sequence in the full revision-ID golden, including a sibling-branch command.
- Trigger arithmetic/capacity/replay-budget faults. Preserve the last valid state and return typed recoverable errors.
- Make simultaneous settlements, AI resource requests, diplomacy decisions, and multi-faction combat independent of render order or thread scheduling.
- Destroy or remove references through valid creator transactions. Verify cancellation/retirement rules without dangling objects.

### UI and rendering fixtures

- Assert scene geometry is not obscured by a whole-window nonmodal panel.
- Derive drawing, clipping, pointer hit testing, focus, and semantic bounds from the same layout and verify equivalence.
- Exercise empty documents, maximum-length valid names, invalid names/numbers, deep hierarchy, long errors, full inventories, filtered selection, and more than four branches.
- Resize during text editing, an import preview, a removal confirmation, and history browsing; preserve draft, cursor, and focus.
- Verify shortcuts do not trigger while typing and scrolled-out controls cannot steal input.
- Validate every shipped WGSL file and instance/buffer ABI; exercise variable world counts including zero, one, seven, eight, and the declared cap.
- Verify High-to-Low degradation keeps text, selection, ownership, direction, and hazards visible.

### Creator capability matrix

Run every row while the selected civilization lacks the resources an ordinary actor would need. Creator commands remain free, but every structural/capacity rule still applies. For each, verify visible discovery, preview, pointer and keyboard submission, creator provenance, undo/redo identity, invalid atomic rejection, and save/reload replay.

| Capability | Required successful and rejected cases |
| --- | --- |
| Topology | Create/edit system, star, world, lane; reject invalid parent, duplicate endpoint, capacity excess |
| Archetype | Change supported star/world archetype; reject missing catalog identity |
| Deposits/inventory | Create/edit finite reserve and exact local inventory; reject mismatch/overflow |
| Colonies/ownership | Establish/transfer/unown colony; preview hub/assets/routes/fleets affected by transfer |
| Facilities | Place/enable/disable/scrap completed industry; reject slots/deposit/reference violations |
| Queues | Create/cancel/reorder one legal job; reject resources as irrelevant to creator but enforce slot/global reservation |
| Civilization policy | Create/edit/name/color/policy; reject invalid name/policy/cap |
| Relations | Set each directed base disposition and bilateral state; distinguish it from earned counters |
| Agreements/war | Force valid trade/nonaggression/peace/war override and atomically resolve incompatible records |
| Fleets/hulls | Create/edit/split/merge/remove fleet and hull composition; preserve IDs/HP/credits; reject transit/co-location errors |
| Orders | Issue/cancel supported explore/move/settle/defend/attack/return; reject route/intelligence/target errors where command requires them |
| Freight routes | Create/edit/suspend/reactivate/remove internal/trade/aid route; enforce endpoint/reference/capacity validity |
| Hazards | Schedule/edit/cancel one valid hazard; reject invalid lane, interval, catalog ID |
| Cascades | Remove dependent entity only through complete previewed disposition; reject incomplete cascade without any mutation |

### Persistence and compatibility fixtures

- Compare all preexisting V1 fixture bytes/digests and seed behavior before and after V2 integration.
- Verify unknown/new versions are rejected by V1 rather than silently interpreted.
- Verify loading a V1 item never writes a V2 item, modifies legacy keys, or changes the original file.
- Exercise V1's unchanged 16-slot/16-MiB-archive/1-MiB-pack limits and V2's independent 64-slot/32-MiB-archive/2-MiB-pack limits. The product import router sends each recognized kind only to its matching adapter; V2 rejects a V1 kind as WrongAuthority; WorkshopV1 retains all existing valid and invalid byte fixtures unchanged. V2 never writes `workshop-v1` or `nyon.workshop.v1`.
- Inject quota denial, invalid JSON, duplicate IDs, invalid graph references, catalog mismatch, integrity mismatch, interrupted writes, stale generations, and replay cancellation.
- Import into a new slot and prove the original slot and active session survive every failure path.
- Seed WorkshopV1 and Living V2 with different authority-local selections. Prove the Library coordinator alone chooses Continue. Exercise first-launch V1 migration, target-slot commit followed by coordinator CAS failure, stale coordinator generation, missing branch, corrupt target, and recovery-generation fallback without arbitrary cross-authority selection.
- On native and browser, corrupt the selected coordinator generation and recover only the prior independently valid generation. Corrupt both and open Library recovery with neither authority silently selected. Verify browser `generation-a`, `generation-b`, and `head` change in one IndexedDB transaction and stale CAS rejects.
- For every `LivingStoreV2` request, exercise Begin/Poll/Cancel before commit, cancellation racing a won commit, wrong owner/session epoch, 1024-unit progress, stale generation, transaction interruption, and typed recovery. Prove no result reports Cancelled after durable mutation and no failed request publishes a partial manifest, pack, slot, or generation.
- Fill all 64 Living slots: archive alone frees no capacity; an additional new/import rejects without mutation. Permanently delete one confirmed archived nonactive slot and prove exactly one create succeeds. For the selected slot, exercise coordinator-clear failure and subsequent slot-delete failure. Fill pack capacity; exercise pack archive/unarchive generation CAS and a stale archive/delete race; reject archival while referenced by an unarchived galaxy and deletion while referenced by any retained galaxy. Then archive and delete one unreferenced pack and register exactly one replacement.
- Decode and re-encode every checked-in V2 golden byte-for-byte on native and wasm. Verify the exact top-level kinds, field/discriminant order, endian framing, optional tags, kind ordinal table, truncation, domain-separated hashes, archive payload integrity, and ordinary exported-file hash. Mutate each dimension and require a typed rejection.
- A browser download handoff is not confirmed disk persistence; record the actual evidence exposed by the platform.

## 4. End-to-end release journey

Use a disposable new slot and record its identifier before starting. Do not replace the user's current galaxy or run competing processes against the same save.

1. Open Vale Confluence, paused, without overwriting another save.
2. Read its briefing and inspect a real shortage from visible information.
3. Add a solar array or a supply route using only normal product controls.
4. Step/resume and verify the resulting authoritative inventory/production change.
5. Run First Expansion and inspect its exact autonomous construction, scouting, and settlement witnesses through tick 411 without creator-issued civilization orders.
6. Observe a trade agreement and actual cross-civilization deliveries in the Commerce fixture.
7. Observe a valid war declaration, a fleet battle, and occupation or retreat in Frontier Friction. Verify the sandbox continues.
8. Inspect the recorded reason for one AI action and distinguish it from a creator override.
9. Pause and record branch, revision cursor, tick, and digest. Resume; while running, create a supported hazard or change a policy. Verify its accepted application tick. Pause later and record the new branch/cursor/tick/digest.
10. At that later tick, Undo the creator intervention and record the counterfactual parent digest materialized at the same tick. This is not expected to equal the earlier-time bookmark.
11. Redo at the same later tick and recover the exact recorded post-intervention digest. Undo again, create a different intervention, and retain both futures.
12. Compare both branches at the same tick and verify the displayed differences against materialized snapshots.
13. Select one branch, save, wait for confirmed durability, quit normally, reopen, and Continue into the same slot, branch, tick, and digest.
14. Export the standalone catalog pack and referencing archive using visible controls. Record both hashes and each platform handoff/completion state.
15. On a profile/library without that pack, choose the archive, observe Missing catalog, select and register the exact pack, and import into a separate slot. Prove a mismatched pack rejects. Verify catalog hash, revision graph, selected branch, tick, and digest.
16. Continue simulation and verify the same next 1000 tick results as the source branch.
17. Open an old WorkshopV1 archive and a Classic scenario through Legacy. Verify original behavior and no source overwrite.
18. Return to the new sandbox, remove/restore the last civilization through validated creator/history controls, and demonstrate continued simulation without a game-over state.

Run the full journey pointer-only and keyboard-only. Screen-reader runs cover creation, inspection, event navigation, history, save/recovery, and import/export. Muted and reduced-motion variants retain the same information and outcomes.

## 5. Platform and layout matrix

### Required layout cases

| Logical window | UI scales | Required checks |
| --- | --- | --- |
| 723x802 | 100%, 115%, 130% | Compact drawers, transport, complete forms, last outliner row, history |
| 1280x720 | 100%, 130% | Responsive transition, short-height scrolling, modals |
| 1440x900 | 100%, 130% | Docked layout, resizable panels, inspector, compare view |
| 1920x1080 | 100%, 130% | Full diorama, maximum visible data, performance capture |

Exercise Retina/high-DPI changes and window resizing without mixing physical pixels into logical layout. Restore the user's window and settings after a manual qualification pass.

### Runtime backends

| Host | Native | Browser |
| --- | --- | --- |
| macOS qualification host | Metal | Safari natural/forced WebGL2; Chrome natural/forced WebGL2 |
| Windows 11 qualification host | DX12 | Edge natural/forced WebGL2; Chrome natural/forced WebGL2 |
| Ubuntu 24.04 qualification host | Vulkan and forced GL | Firefox natural/forced WebGL2 |

Record OS/browser version, GPU/driver, requested and selected backend, artifact hash, catalog/seed, final state digest, screenshots, keyboard/screen-reader behavior, save/reload, fallback, and performance for each row. Missing hosts remain pending; source compilation cannot satisfy them.

Browser startup must visibly recover when WebGPU is unavailable or initialization fails. If both paths fail, show an actionable error rather than a blank canvas. Native device/surface recreation preserves the valid session and presentation preferences.

## 6. Performance, endurance, and offline acceptance

Targets are requirements, not measured claims:

- Authority step p95 at or below 5 ms at declared caps on each named native qualification host.
- Representative Living Galaxy journey p95 frame time at or below 16.7 ms at 1920x1080 on native and WebGPU.
- WebGL2 Low p95 at or below 33.3 ms at 1920x1080 on its named qualification host.
- Input-to-visible-response p95 at or below 100 ms outside intentionally bounded import/replay operations.
- Capacity fixture includes 64 systems, 512 worlds, up to 16 civilizations, 2048 industries/routes within valid per-colony bounds, 4096 freight shipments, 256 fleets, and 2048 total hulls. All objects remain inspectable when decorative traffic is aggregated.
- Run a 36000-tick mixed civilization fixture and a longer 100000-tick endurance fixture with bounded event/history memory. Check invariants and reproducibility; these are simulation-tick counts, not promised test durations.
- Replay/import works incrementally with cancellation and progress. Capacity rejection neither corrupts state nor stops unrelated valid activity.
- Run new/open/play/edit/branch/save/export/import with networking disabled. No fonts, assets, AI decisions, or narrative depend on network fetches.

Report requested versus effective simulation speed under load. Do not silently skip authoritative ticks to appear responsive. Rendering quality can decrease; simulation rules cannot.

## 7. Commands and evidence boundaries

Follow the repository's live `AGENTS.md` if its gate instructions evolve. At design time the required CPU/source gates are:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
git diff --check
```

For browser work:

```sh
./tools/build-web-webgpu.sh
./tools/build-web-webgl.sh
./tools/check-workshop.sh
cargo check --target wasm32-unknown-unknown --lib
cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings
```

The fuzz crate is its own workspace. Decoder/catalog changes also require its separate format/compile/lint gate and an explicitly recorded fuzz run when claiming fuzzing coverage:

```sh
cargo fmt --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all --check
cargo clippy --locked --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all-targets -- -D warnings
```

Build native release with the actual binary target. Build and inspect the exact artifact launched for acceptance. Do not run a second native process against an active save. Run adapter-backed parity/shader checks where required, and record `SKIP` as absence of GPU evidence.

Do not copy historical test counts into the completion report. Record actual current counts and outcomes from completed commands. A documentation-only validation pass is not a game test.

## 8. Delivery labels

1. **Design-reviewed:** this proposed package has explicit user approval and an internally consistent review.
2. **Implemented:** every requested behavior exists and the appropriate focused/workspace tests pass.
3. **Artifact-qualified:** native and both web artifacts are built, identified, and inspected.
4. **Runtime-qualified:** required live journeys, backends, input and accessibility paths pass.
5. **Release-qualified:** runtime, performance, recovery, offline use, legacy preservation, provenance, documentation, and exact reviewed changes all pass.

The goal is not complete at labels 1-3. An unavailable qualification host, an inaccessible control, a missing simulation system, or a failed acceptance row remains an explicit gap. Keep the whole objective active until the evidence proves the whole requested product.
