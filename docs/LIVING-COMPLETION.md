# Living Galaxy completion ledger

Source inspection: 2026-10-01, canonical NYON checkout, including existing dirty
`living/rules.rs`, `living/simulation.rs`, `sandbox_session.rs`, and associated
tests. This ledger records source evidence, not historical plan checkboxes.
The main table reflects third-phase source inspection; historical baseline
appendices below retain their original evidence boundaries. Executed checks and
first-snapshot live smoke observations are recorded in LIVING-VERIFICATION.md.
Later edits require fresh evidence; full product qualification remains unmet.

Authority: the approved Living Galaxy design, rules, experience, qualification
contracts dated 2026-09-04 and creator operations addendum dated 2026-09-17 in
`docs/superpowers/specs/`. The user excludes commits/publication from this pass;
LG-23's original commit requirement is therefore out of scope here.

**Current** means source supports the stated bounded observation, not full
acceptance. **Partial** means implementation exists with unmet requirements.
**Proposed** means required implementation is absent. **Blocked** requires a
specific unavailable prerequisite; no platform is assumed unavailable merely
because this review did not exercise it. Full runtime qualification and performance remain unverified; bounded live observations are explicitly recorded separately.

| ID | Status | Current source evidence and remaining completion gap |
| --- | --- | --- |
| LG-01 | Partial | `presentation/living.rs` projects worlds, systems, lanes, ownership, hazards, fleets and freight beneath `ChromeOnly` chrome. First-snapshot native/WebGPU/WebGL2 smoke observations exist in LIVING-VERIFICATION. Fresh third-phase natural WebGPU and forced WebGL2 initialized. A real CSS/backing logical-pixel mismatch was observed and root fixed source afterward; rebuilt pointer/layout qualification and full visual acceptance remain. |
| LG-02 | Partial | Logical-pixel scaled layout, paginated inspector/Chronicle and creator/history/content/transfer drawers exist. Full declared size/scale matrix, resize during editing and zero-overlap live qualification remain. |
| LG-03 | Partial | Selected-world inventory, colonies, facilities, jobs and receipt inspection are projected from authority. Selectable projections now cover all 15 entity kinds through a searchable hierarchy and authoritative detail rows, with focused navigation/presentation tests. Full live inspector parity and access remain unqualified. |
| LG-04 | Partial | World paging, searchable creator tools, branch/bookmark/history controls and notebook records exist. Searchable/filterable full entity hierarchy is implemented with selection and camera reveal tests. Deepest/filtered live reveal and capacity virtualization performance remain to qualify. |
| LG-05 | Partial | `living_creator.rs` drafts and session text-editing controls provide editable fields, validation preview, Apply and Cancel. Native measured caret/selection and scrolling draft viewport, clipboard editing and requested focus restoration are implemented. Pointer caret hit testing uses the rendered projection and measured boundaries, with two focused scaled-position/4096-byte viewport tests passed; drag wiring is present but its App tests are pending. Initial focus installs a caret without typing and preserves unchanged draft selection in source. Resize and complete keyboard-only/native/browser editing journeys require full qualification. |
| LG-06 | Partial | Galaxy/system/world navigation actions and read-only projected selection exist, with stable full-ID geometry/colors. Bounded logical-pixel pan/zoom, fit, two-contact touch gestures and focus-scoped camera state are implemented; live collision-free focus, touch and removal recovery remain unqualified. |
| LG-07 | Partial | Canonical history/archive reconstruct genesis, revision modes, retained sibling provenance and allocator marks; focused authority regressions exist. Full fixed-fixture/insertion-order/native-wasm state+receipt parity and strict visited-entity replay budgeting remain incomplete. |
| LG-08 | Partial | Atomic recipes, capped resources, jobs, freight and central shared-snapshot reservations exist; queued facility uniqueness and scarcity selection receive regression work. Complete resource/forecast/recovery/capacity fixture matrix remains. |
| LG-09 | Partial | Internal 200-boundary scheduled economic forecast, policy/emergency tiers, scored economy/fleet selection, authorized supply/Trade/Aid, defense/reinforcement, scouting and dormant ark recovery exist. Conservative full-slot duplicate-producer replacement has an explicit optional scrap witness. Existing route adjustment now preserves the route ID, cadence and in-flight manifests, with focused regressions. Strict whole-path visited-entity work budgeting, exhaustive role/target candidates, comprehensive demand/output classification and broader nonessential replacement remain incomplete. Authority role/privacy/reservation tests and economics/delivery regressions are worker-reported green; they do not prove exhaustive classifier or runtime qualification. |
| LG-10 | Partial | Bilateral snapshot proposals, agreements, war/truce and dormant actor rules are implemented with focused tests. Complete boundary fixtures and displayed bilateral structured reasons remain. |
| LG-11 | Partial | Combat, retreat, occupation, capture, return credits and dormant ark recovery exist. Full multi-faction simultaneous combat/capture/itinerary/resource-preservation fixtures and live inspection remain. |
| LG-12 | Partial | Empty and multi-civilization simulation, validated removals and nonterminal sandbox sessions exist. Paired zero/one/many civilization runs through 100,000 ticks completed on an earlier authority snapshot, with exact final digests recorded in verification. This is bounded endurance evidence, not indefinite or current-build performance proof. Current capacity faults and all removal/history restoration journeys remain to qualify. |
| LG-13 | Partial | Chronicle displays authoritative receipts, kind/ID/tick and creator/scheduled/decision provenance. An additive decision sidecar now defines actor/action/target/score/tier/snapshot/intelligence and structured reasons, with independent full-byte golden and 5 focused tests passed. Authority emission and paired sidecar replay are implemented; inspector/Chronicle rendering integration is in progress. A pure latest-500 Chronicle cache now validates paired receipt integrity, aggregates repetitive autonomous events in exact 100-tick windows, retains creator/important events individually and bounds summaries to 2048 printable bytes. Seven focused cache tests passed. Additive ordinary-event facts bind actual actor/entity/resource/reason references to exact ordinals/IDs; six focused core tests include independent new facility-produced and sparse deposit-draw witnesses. Session publication/emission integration and complete historical reconstruction/live follow remain unqualified. Successful production adds an approved ordinary event, preserving state/archive identities but changing historical receipt/event identities at affected boundaries; section10.1 records this boundary. |
| LG-14 | Partial | Canonical retained siblings, immutable revisions, exact undo/redo/bookmark/history view and cancellable matching-tick comparison are integrated into session controls. Separate exact-archive notebook schema, bounded replay validation, native/browser attachment storage and save/Continue sequencing are implemented in source. Optional portable evidence metadata now replay-validates exact receipt digest/event ID/ordinal/kind/provenance; native save/Continue restores visible references without a receipt cache. Two durable-journey tests and independent nine-test attachment/proof/UI review passed. Full digest matrix, more-than-four-branch live journey, five remaining experiment workflows, transfer live qualification and strict whole-path work budgets remain. |
| LG-15 | Partial | Separate native/browser LivingStoreV2 and resident LivingClient provide save/open, exact catalog recovery and neutral Continue coordination. Independent coordinator 6/6, store 15/15 and client 12/12 passes include interruption/CAS/cancellation/capacity, exact notebook binding and retained recorded Continue restoration. Fresh WebGL2 actual IndexedDB save, interrupted-browser reopen of exact tick168 with notebook, and separate-slot archive import were observed. Root also reported actual native import, save/Continue and AX quit/reopen of the same tick168 projection. Quota, deliberate live corruption recovery and full platform journeys remain unverified. |
| LG-16 | Partial | Native OS dialog/bounded read/durable write and browser file input/download adapters, archive validation preview and new-slot commit are implemented. Prepared coherent catalog/archive/notebook file sets, freshness checks, per-file handoff/confirmation states and cancellation hold sequencing are implemented with host/fake-platform regressions. Fresh UI-created export/reimport and live cancellation/ownership evidence remain required. |
| LG-17 | Partial | Classic and WorkshopV1 remain separate from V2 identities/storage; first-snapshot full gates preserved frozen contracts. Later shared runtime changes require fresh complete gates and legacy live journey; no old snapshot proves current compatibility. |
| LG-18 | Partial | Shared semantic actions, typing-safe shortcuts, focus activation, drawer holds, contrast/reduced-motion settings and audio controls exist. Complete pointer/keyboard and native/browser screen-reader equivalence plus modal focus qualification remain. |
| LG-19 | Partial | First-snapshot native/WebGPU/WebGL2 initialization and transport smoke observations exist. Current updated backend matrix, degraded quality, acquisition failure and session-preserving graphics recovery remain unverified. |
| LG-20 | Partial | 10 Hz transport with 1x/4x/20x, bounded catch-up, holds and fault presentation is implemented. Named-device representative/capacity measurements, network-disabled complete journey and strict 1024 visited-entity work slices remain. |
| LG-21 | Partial | Vale Confluence, Commerce, Frontier Friction, Interrupted Corridor, First Expansion, Blank and reproducible Seeded constructors/tests and content picker exist. Introduction and evidence-backed notebook controls exist. Complete accepted witness deadlines, guide/notebook provenance and live journey remain to qualify. |
| LG-22 | Partial | Original procedural native/browser ambience/music/effect cues, persisted channel levels, mute and rolling four-start limiter are implemented. Hardware failure/audibility, session/node cleanup, no backlog and digest-isolation matrix remain under review or unverified. |
| LG-23 | Partial | Completion/verification ledgers preserve source versus executed evidence and first-snapshot artifact hashes. Current player manual/artifacts must match later verified behavior; full independent diff review and all remaining limitations required. Commits/publication are explicitly out of scope. |
| LG-24 | Partial | Discoverable creator forms and previews cover 46 operations, including 18 approved additive edit/cancel/split/merge/enable operations; exact reviewed stable/derived cascades, refunds and reservations are implemented. Independent root qualification exercises all46 forms through preview/apply, exact undo/redo, reload and missing-reference/local rejection. Literal system64, world512, star128, civilization16, lane256, fleet256, deposit1024, hazard128, globalhull2048 and per-fleet hull16 capacity regressions verify atomic rejection, reclamation and history restoration. Generic-free versus explicit economic cancellation, paid reservations and ownership job-disposition guards also have regressions. Nine creator qualification tests pass. Detailed per-operation axes are in LIVING-CREATOR-MATRIX.md; full scalar/reservation/cascade capacity matrix and live platform qualification remain. |
| LG-25 | Partial | Neutral integrity-checked native a/b/head OS-lock CAS and transactional browser coordinator select exact authority/slot/branch/view. Native coordinator 6/6 and cross-authority qualification 7/7 pass, including zero-based WorkshopV1 migration, explicit neutral None, invalid Living recovery, normal/cancelled archive reconciliation and same-number slot authority isolation. Destructive confirmation/blocker matrix and actual IndexedDB cross-authority journeys remain. |
| LG-26 | Partial | Canonical pack/archive codec preserves strict V2 identities, embedded genesis, revision modes and branch allocator marks; independent unchanged vector verification and authority regressions exist. Full pack/archive byte corpus native/web parity and adversarial matrix remain incomplete. |
| LG-27 | Partial | Tail-aware queue/projection rejection, atomic same-boundary parent chains and canonical replay mode persistence are implemented with regressions. Full pending-tail golden corpus, mixed cursor undo/redo/fault matrix and native/web parity remain required. |

## Original authority findings and repair status

1. `LivingGalaxyAuthorityV2::drop_civilization` directly mutates committed state
   without command, revision, receipt or validation. Restrict fault injection to
   private unit-test helpers; gameplay removal must use validated creator cascades.
2. `fork_paused` clears the parent's archive list and assigns only that parent to
   the child. A second fork drops addressability of prior sibling futures. Keep
   a document-wide immutable branch graph with unique durable allocator marks.
3. `play_recorded` turns all future same-tick revisions into running queue entries.
   A paused edit recorded after production at nonzero tick replays before that
   production. Record/reconstruct the accepted application mode and order without
   changing canonical identity formulas; test mixed running and paused groups.
4. `dispatch_fleets` allows foreign-origin non-return departures whenever every
   hull has credit. Rules require refuelling for fresh unrelated excursions;
   credits cover return and the specific dormant-ark recovery exception only.
5. `generate_and_reserve_intents` iterates actors while mutating the shared state;
   its call structure does not implement generation from one immutable visible
   snapshot followed by central reservation. Diplomacy likewise needs explicit
   post-update snapshot proposal collection. Stable iteration alone is insufficient.
6. `reserve_economy` implements ark production for expansionist/bonus actors, not
   the general scored economic candidate contract. First Expansion specifically
   requires a Neutral foundry accepted at 50, ark accepted at 100, and scouting/
   settlement receipt witnesses. Build the accepted fixture before claiming it.

Findings 1-4 were repaired in source during this pass; the worker reports their
focused regressions passed. Shared-snapshot central reservations and the First Expansion fixture now
address the bounded portions of findings 5-6. The general forecast/scoring and
complete economic candidate matrix remain open. These original findings
were source observations/inferences, not executed failing tests.
Workers may repair them during the same session; this baseline does not claim
their original code remains unchanged after inspection.

## Original missing interface dependencies and sequence (historical baseline)

First fix authority replay/atomicity and shared-snapshot intent semantics, then
implement validated creator capabilities and complete receipts. A Living history
module must expose immutable revision/branch graph, exact active view/bookmarks,
incremental replay, undo/redo and matching-tick materialization. Canonical archive
decode must embed validated genesis, reference an exact catalog and preserve
allocator high-water marks while excluding checkpoint caches. These interfaces
precede save adapters and portable transfer.

Add independent `LivingStoreRequestV2`, `LivingStoreResultV2`, slot/generation/
catalog/archive descriptors and `LivingStoreV2` bounded Begin/Poll/Cancel jobs.
Native uses V2-only atomic generations; browser uses its own transactional
IndexedDB identity. Enforce 64 slots, 256 packs, 32 MiB archive, 2 MiB pack,
1024 work-unit slices, generation CAS, durable completion versus cancel races,
archived capacity retention and safe permanent reclamation. Then add neutral
two-generation Library selection with exact authority/slot/branch/state reference,
V1 initial migration, recovered-generation CAS and explicit invalid-target recovery.
Preserve prior resident session on every open/import/coordinator failure.

The session owns 10 Hz 1x/4x/20x transport, bounded catch-up, receipt retention,
dirty state and asynchronous jobs. Its Living projection feeds scene, inspector,
navigation, Chronicle and semantic controls from one authority snapshot. Shared
runtime files require one integration owner. Build validated content and player
journeys after those controls exist; do not script receipt witnesses into UI.

## Evidence still required

Record actual exits and artifact hashes for the native, independent fuzz, wasm
and both-backend artifact gates in `AGENTS.md`. Run unchanged vector verification
before identity/vector changes. No baseline or gate success is implied here.
Focused authority tests must cover the findings above plus every creator operation,
scarcity/reservations, queue tail rejection and atomic faults. Persistence tests
must exercise real adapter transactions as well as host models.

Separate live native, WebGPU and WebGL2 create/inspect/run/intervene/branch/compare/
save/reopen/export/import/recovery journeys, actual IndexedDB, pointer/keyboard/
screen reader, responsive logical-pixel layouts, network-disabled operation and
named-device performance. Required Windows/Linux hosts and browser combinations
remain **unverified** until attempted; mark **Blocked** only with the concrete
unavailable host/tool outcome. Browser download handoff is not disk durability.
Screenshots do not establish replay, accessibility or persistence. GPU skips do
not establish parity. Completion remains unmet while any applicable row lacks
its accepted evidence.

## Follow-up source review: implemented slices

Later source inspection on 2026-10-01 supersedes the original baseline findings
where explicitly stated below. This reviewer did not execute the new tests;
their existence is source evidence only. Overall completion remains Partial.

- LG-01 now has an initial read-only `src/presentation/living.rs` projection for
  worlds, systems, lanes, active hazards, fleets and freight. `build_sandbox_frame`
  draws it into the underlay and the session uses `ChromeOnly`. This is a bounded
  scene implementation, not the complete approved diorama or live compositing
  qualification. Mark LG-01 **Partial**, replacing its original Proposed status.
- LG-02/LG-18 now have Run/Pause, Step and 1x/4x/20x controls with shared typed
  pointer/semantic actions and logical bounds. The rest of the accessible
  workspace remains absent. Scale handling still resolves at fixed 1.0.
- LG-07/LG-14/LG-27 now retain private revision modes for replay, preserve prior
  flat archived views on repeated forks and remove the public corruption bypass.
  New tests cover paused nonzero edits, mixed running/paused boundaries and
  repeated forks. Canonical archive persistence of modes/history is still absent.
- LG-11 now restricts foreign-origin launch to credited return or the dormant
  ark settlement recovery exception and consumes credits on departure. New
  foreign-origin regression tests exist. Full itinerary/retreat qualification remains.
- Session transport now uses integer nanosecond accumulation at 10 Hz with
  1x/4x/20x speeds, a twenty-boundary update cap, explicit suspension/holds and
  fault display. Cadence/hold tests exist; actual application integration requires
  verification in addition to direct session tests.

Historical integration findings sent to the owner (items 1-3 were subsequently repaired in source; latest review below supersedes item 4):

1. `App` calls `set_sandbox_suspended(false)` every normal redraw. The inspected
   setter clears fractional debt unconditionally, so normal 60 Hz 1x/4x updates
   cannot accumulate a boundary. Clear debt only on suspension transitions and
   add an integration regression that repeats the setter before short updates.
2. Native suspension and guide-modal suspension share one boolean; a redraw's
   guide=false assignment can release the native hold. Keep causes independent
   or use scoped holds with explicit lifecycle ownership.
3. Capped updates discard excess elapsed wall opportunity and the readout reports
   only requested speed. Qualification requires visible requested versus effective
   speed under load. Numbered authoritative ticks are not skipped by this code;
   reduced effective speed nevertheless needs explicit measurement/presentation.
4. The circular layout is regenerated from entity index each extraction; topology
   changes reframe existing markers. This is an initial overview, not accepted
   stable camera/navigation. Fleets/freight lack identities, ownership, direction
   and semantic inspection in the projection; worlds have no labels drawn here.

The original shared-snapshot intent generation, general economy/content,
canonical archives/stores/coordinator and complete workspace findings remain.

## Latest source review and evidence boundary

The suspension setter now clears debt only on transitions; App retains native
window suspension separately and combines it with guide activity. Transport now
reports effective speed when catch-up caps work. These fix the earlier integration
findings in source; direct-session evidence does not prove a live redraw journey.

Scene coordinates now derive from identity bytes rather than collection indices.
Existing entities therefore retain placement after insertion/removal at fixed
bounds, but placement has no collision resolution: systems sharing the first four
ID bytes coincide, worlds sharing the first two bytes have identical orbit angles,
and many worlds on a radius capped at 42 logical pixels necessarily overlap.
`hit_world` chooses the nearest marker and equal-distance ties retain iteration
order; overlapping entities have no reveal/cycle/navigation mechanism. Ownership
colors still derive from civilization index modulo four, so adding/removing a
civilization can recolor unrelated ownership and more than four civilizations
reuse colors. Fleet/freight markers contain positions only and overlap for shared
locations. This is an initial overview, not capacity-qualified inspectability.

Transport controls and semantics share bounds; scene hit radius (22) exceeds its
visible world radius (15), and the hit helper is not connected to player selection.
UI scale is fixed at 1.0. Underlay and ChromeOnly composition exists in source;
no live artifact inspection occurred in this independent review.

Evidence reported by other task owners: root projection suite 2/2 at its build
snapshot; authority worker focused Living play suite 12/12. Those are attributed
reports, not commands executed by this reviewer or a complete repository gate.

## Final independent source review

Later source supersedes historical risk descriptions above: placement and ownership
colors now mix full identities; previous/next world selection gives overlapping
worlds a reachable control path. Collision-free diorama capacity and full navigation
remain incomplete. Scaled frames, paginated world inspector and latest-16-receipt
Chronicle now exist, as do occlusion/focus/drawer guards and resident-session
preservation on close. Canonical actor/target/reason stories remain incomplete.

Authority now collects immutable-snapshot intent candidates and atomically reserves
their deltas, with global 64-branch/10000-revision admission. First Expansion has
its approved constructor and witness assertions. Worker reports focused play
15/15 passed; no blanket native/web gate result is available at this update.

Private replay_base retains the historical authority boundary to preserve old
autonomous IDs. It must not be mistaken for canonical genesis-plus-revisions
reconstruction or an exportable archive: canonical archive/history codecs do not
exist, and checkpoints must remain excluded from canonical exports. General
200-tick forecast/scored economic decisions and structured actor receipts remain
open. Diplomacy still mutates declarations actor by actor; shared post-update
proposal collection/resolution requires independent qualification. Persistence,
coordinator, creator edit/cascade matrix and remaining starter/content work remain
unimplemented. The full approved product is incomplete.

## Persistence implementation and direct evidence

This review worker subsequently implemented `src/living_store.rs`, its native/
web adapters and shared manifest, plus `src/library_selection.rs` and native/web
coordinators. Root owns exports, platform factories and client integration.
Storage identities remain `living-v2`, `nyon.living.v2`, and neutral
`nyon.library.v1`; no Workshop wrappers or public store widening were introduced.

Native uses bounded individual content-addressed pack/archive payloads, bounded
256 KiB two-generation manifests, OS file locks for cross-handle CAS, fsync/rename
commit and exact-catalog validation. Reads allocate only requested bounded
payloads/catalogs. Descriptors carry ephemeral per-item missing/length diagnostics;
listing is not canonical replay validation. Previous independently valid slot
payload recovery returns its generation, and a subsequent save compares that
recovered generation. Archived slots retain capacity; confirmed deletion frees
one entry. Pack deletion checks all retained slot catalog references.

Begin does not mutate storage; owner-scoped jobs start on poll. IDs are unique
across adapters within the process. Native I/O runs on workers. Cancellation
before commit wins preserves storage; an in-flight commit returns CancelPending
until poll establishes its outcome, never a guessed Cancelled result. Terminal
release reclaims bounded job lanes. Browser writes payloads, manifest and head in
one IndexedDB transaction after exact raw-snapshot CAS. Browser archive replay
yields to the event loop and requires fresh budget credits from caller polls.

Directly executed evidence at recorded build snapshots:

- `cargo test --test library_selection --test living_store`: exit 0, coordinator
  5/5 and Living store 10/10, including exact tick-61 reopen, foreign job IDs,
  newest-payload corruption recovery/CAS, 64-slot capacity/reclamation, 256-pack
  capacity/reclamation with stale delete CAS, per-item missing diagnostics,
  interrupted-before-head recovery, cancellation while waiting for the OS commit
  lock, and safe known-reference garbage collection preserving both recovery
  manifests and unrelated files. This latest run is recorded in
  `/tmp/nyon-persistence-tests9.log`. A prior 8/10 run exposed colliding test
  temporary directories; a process-local atomic suffix now isolates each test.
- `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`: exit 0
  after the latest diagnostics, cancellation, and reclamation changes; log
  `/tmp/nyon-persistence-wasm-clippy6.log`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: exit 0
  at this build snapshot; log `/tmp/nyon-persistence-native-clippy4.log`.
  These checks do not establish live persistence or full platform qualification.

Native quota failure, further cancellation races, browser transactions and
quota/recovery, cross-authority migration, interrupted locks on actual process
exit, and live transfer/session journeys are unverified. Payload garbage
collection preserves both recovery generations in native regression coverage;
actual browser reclamation remains unverified. Native cleanup occurs after the
durable commit and ignores deletion errors, so failed cleanup can leave payload
files without retry tracking. Permanent physical reclamation under cleanup
failure remains Partial. Authority replay budgets count bounded atomic operations rather
than normative visited-entity work units; JSON/graph validation and a capacity
tick can exceed the strict 1024-unit target. The strict budget/performance row
therefore remains Partial despite incremental timeline replay.

### Notebook persistence source slice, 2026-10-01

`LivingStoreRequestV2::WriteNotebook` uses the slot generation CAS and publishes
a separately bounded 1 MiB content-addressed notebook attachment plus a cloned
archive in a new generation. `ReadNotebook` resolves an exact retained generation
and returns an optional attachment. Metadata changes carry matching attachments;
archive saves detach the new head while retaining the prior generation's notebook.
Both recovery manifests protect referenced notebook payloads during reclamation.
Native validation runs on the existing worker; browser archive/notebook replay
consumes polling credits and yields between batches. The authority's strict
visited-entity work-unit limitation also applies here.

`LivingClient::save_with_notebook` prepares and validates the exact archive and
notebook before catalog registration, then sequences archive commit, notebook
commit and neutral selection CAS before acknowledging Saved. Open reads the exact
attachment and validates its archive binding and witnesses before installation.
Cancellation drains active storage/coordinator lanes and drops private replay
preparation. Existing metadata-free saves remain compatible. This is source
evidence; latest expanded regression and platform gates are still running.

Notebook integration focused evidence subsequently passed:
`cargo test --test living_client --test living_store`, exit 0, 7/7 client and
12/12 native store tests (`/tmp/nyon-notebook-client-tests5.log`). Sparse observation
and genesis-only fork independent Rust vectors passed 3/3; independent original
Living vectors remained unchanged. This does not close browser persistence or
live player-journey qualification.

Exact neutral Continue subsequently gained bounded retained-view restoration:
validate the stored head's notebook first, restore the recorded branch/revision/
tick, then require its exact digest before candidate installation. This closes
the source gap where a partially completed save's newer head unnecessarily hid
an older valid recorded view. Focused client suite passed 8/8, exit 0
(`/tmp/nyon-exact-continue-tests2.log`); missing/future/drift cases remain recovery.
Independent authority review found no blocker in the inspected notebook store
and client sequencing. Archive and notebook commits remain separate transactions,
so partial saves keep resident notes dirty and cannot acknowledge combined
atomicity.

Cross-authority lifecycle/cancellation qualification subsequently passed:
`cargo test --test living_qualification --test library_selection`, exit 0,
qualification 7/7 and coordinator 6/6 (`/tmp/nyon-neutral-qualification-tests5.log`).
This exposed and repaired an authority-specific validation error: WorkshopV1
slots start at zero, while LivingV2 slots start at one; the neutral record now
preserves both native identity ranges. Independent regressions cover modal hold
retention while cancellation's durable outcome is unknown, terminal Idle hold
release, and discarding late imported bytes. These are host/fake-platform
transaction tests, not live native file-dialog or browser download evidence.

Independent native field review (2026-10-01): measured boundary projection visits glyphs once and uses partition searches, replacing repeated suffix measurement. The focused field regression passes 1/1 with a 4096-byte draft and unchanged semantic value/authority digest. This establishes host rendering-model behavior; native visual editing, IME and platform accessibility remain unverified.

Independent persistence follow-up (2026-10-01): exact globally selected Living
permanent deletion now CAS-clears Continue before beginning authority-local
deletion. Failed coordinator CAS preserves the slot; subsequent deletion failure
leaves the archived slot recoverable with Continue unset. Same-number Workshop
selection remains isolated. Focused host client 12/12 and store 13/13 pass.
Additional tests cover notebook cancellation after worker start, both premanifest
and prehead interruption, and fake quota rejection after archive commit without
Saved acknowledgement or coordinator mutation. Actual quota/IndexedDB behavior
remains unverified. Private manifest version 2 now binds generation and payload under a store-specific
integrity domain. Legacy unversioned manifests require an intact exact head;
corrupt/missing-head legacy recovery rejects explicitly while preserving bytes.
Generation tampering and legacy read/write upgrade regressions pass.

### Trade explanation and observation checkpoint (2026-10-01)

Current focused behavior: accepted bilateral agreements and full inventory deliveries publish typed, receipt-bound facts. Legacy unknown dispatch classification remains unknown; returned and captured cargo do not become delivery witnesses. When requested facts exceed the fixed 2 MiB envelope, a canonical receipt-bound range marker identifies unavailable detail while preserving every ordinary event. Chronicle separates capacity omissions from unknown facts and labels repeated delivery quantities per event.

Root Chronicle checks passed 8/8, including 4,096 events with separate detailed/omitted aggregates. The host Session journey recorded separate Commerce formation/delivery observations, saved and left, and reopened through a fresh Continue client with exact replay evidence (1/1 passed). These are individual observation proofs; full linked experiment completion, live IndexedDB, and current native/browser qualification remain Partial.

A standalone bounded simultaneous-combat cursor is under qualification. It does not yet make all conflict work or the resident tick path bounded. Retreat, occupation, settlement arbitration, later intent/dispatch stages, and full publication preparation still require completion. Whole-tree builds and tests are being rerun for this checkpoint; earlier artifact hashes do not qualify these changes.

### Local linked Trade verification and camera recovery (2026-10-01)

Current host workflow: the Trade notebook exposes Verify matching trade observations. Its private asynchronous replay validates the exact retained archive and recorded evidence, checks resident scope, bookmark and metadata generation, and publishes derived linked proof without saving or changing authority or metadata. The proof labels Agreement and matching delivery observed and exposes both exact bookmarks; no dispatch-causation claim. Independent local-job tests passed 8/8. Latest empty-pair status edit is pending a focused rerun.

A public regression reproduced stale focus after removing a selected facility. Recovery now fits its surviving scope once and clears invalid focus targets, preserving later user framing. Root initial-fit tests passed 4/4. Standalone retreat also passed independent 4/4 and combined core420/25; occupation, settlement arbitration, complete step orchestration and frame-time qualification remain Partial.

The earlier 1,251-test integrated checkpoint and its artifact hashes apply to that frozen source only. New local verification, derived Supply work and label changes require fresh full gates and live platform journeys.

### Frozen integrated checkpoint: 1,290 passing tests (2026-10-01)

All documented automated gates reached actual exit 0 for this frozen checkpoint: native formatting, strict workspace Clippy, full workspace tests, release build, separate locked fuzz lint/format, both browser builds, root/core wasm checks, strict wasm Clippy, Workshop artifact checks, and the unchanged independent Living vector corpus. The workspace run passed 1,290 tests across 96 suites, with zero failed or ignored.

The checkpoint includes local linked Trade/Supply notebook verification, save/Continue retention, camera initial fit and invalid-focus recovery, measured label placement, and standalone retreat/combat/occupation/settlement composition. Complete bounded resident simulation, the remaining three notebook workflows and live native/WebGPU/WebGL2/IndexedDB/accessibility/performance qualification remain Partial. These automated results do not establish platform acceptance. Exact hashes and logs are in the execution ledger.

### Native portable round trip and short performance pilots — 2026-10-01

- Frozen native artifact c4d2f94cdd2d346036bfdcf32527b23e3d5f479c7c110f7a7da897f983788898: Prepare exact export, real native Save panel, unique file `NYON-native-alternate-20261001.nyonliving.json` under repository `.playwright-mcp`, and Galaxy copy saved acknowledgement. Native Open panel selected that exact file; preview validated5systems/8worlds/3civilizations/tick100; Apply imported galaxy created/opened a separate save with exacttick100/inventories and Run controls. This verifies one native Galaxy round trip against the built-in catalog; optional Catalog/Notebook transfer and browser file round trips remain separate.
- Named-host resident Step plus Chronicle release pilots actual exit 0, executable SHA300e03829f0a9130e1b291714b5278deb8c2c14b6a461c916c224e2f75033c14, AppleM5/macOS27.2build26B5091g: Vale100samples p50.129083ms,p95.481083ms,max11.436375ms; topology10samples p50.750250ms,p95/max6.747958ms. The topology has64systems/512worlds/16dormant civilizations and no active industry/freight/combat. These are short pilots, not fullcapacity/frame/browser/endurance acceptance. Exact scope and results in docs/LIVING-PERFORMANCE.md.
- Root benchmark source review clear; benchmark2tests and strict scoped Clippy actual 0. Independent Chronicle restoration source review clear. Original independent vectors and all16preexisting additive modes verified actual 0 before new Corridor optional-locator wire work. Root reviewed concrete route/hazard locator shape; admission/public workflow implementation is in progress.

### Frozen integrated checkpoint: 1,332 passing tests — 2026-10-01

Native workspace28 full tests actual terminal exit 0 and durable0:1,332passed across100suite results,0failed/ignored (`/tmp/nyon-current-workspace-tests28.log`). Strictworkspace28 actual 0 and format29actual 0. This frozen source includes latest authenticated Chronicle replay cache restoration, revised exact-boundary reopen regressions, occlusion-aware labels, optional Corridor attempt locators and current-observation UI, phase7 private staged integration, and validated capacity benchmark fixtures. Independent staged18 execution and rootUI source review also clear.

Fresh native release11, both browser build12, separate fuzz format/lint4 and follow-up wasm/artifact gates are in progress. Do not promote those unfinished gates or current-source live qualification to passed. Browser inspection tabs disappeared before the new offline attempt; its first Network.enable command failed, so no offline qualification or network-setting change was established. Earlier completed native-to-WebGPU-to-WebGL2 import evidence remains tied to frozen1,290artifacts.

Full accepted Living Galaxy remains Partial: resident step still synchronous, bounded dispatch/forecast/final sealing and reclamation incomplete; linked Corridor/war/portable experiment workflows and full live platform/accessibility/recovery/performance evidence remain. Compilation, focusedcore, short pilots and actual UI journeys retain distinct boundaries.

### All automated gates and rebuilt browser observations — 2026-10-01

- Frozen1,332test/100suite checkpoint completed EVERY documented automated gate with actual terminal exit 0: format29; strictworkspace28; fullworkspace28; native release11; both web build12; root/core wasm check12; strict wasm14; Workshopartifact10; independent fuzzformat/clippy4; unchanged originalvectors3. NativeSHA2566a8e6f8276d280fe752a0ad20255004623813c27fada7a9b7506da03e107af82; WebGPUb82dd4db057e43d72b6ec9a767eac8132bbf9b0cd8f1eaa1fd60c513c8aac61b; WebGL2137b5c21c6d8d95ef0537692b20e872ca139230b842a50041e57932c17a8879b. Exactmatching native copy retained at target/qualification/NYON Living V2 Checkpoint 1332.app.
- Rebuilt WebGPU opened exact IndexedDB100/inventories. Actual Chronicle drawer now showed its replay-regenerated Creator intervention at100 (one verified latest boundary), fixing empty reopen presentation. On fresh tab4, successful CDP network-offline emulation, semantic Step100→101, and Save acknowledged Galaxy and Continue selection saved. Restored ordinary network access before reload. WebGPU reload and fresh forced WebGL2 tab5 both opened exact101 withenergy156/ore111/alloy130 and12fleets/1shipment. Loaded-runtime offline operation is verified; offline cold startup/service-worker recovery is not. Neutral Loading caption now visibly exact selected saved galaxy on both rebuilt backends.
- Earlier stale-tab offline attempt failed on its first command and established no offline evidence/change; this later fresh-tab journey supplies the bounded observation. Browser tabs4/5 marked for continuation; temporary network throttling restored.
- The sourcefreeze was explicitly released AFTER all gates. New immutable Corridor paircache and measured reservation hotfix/standalone dispatch draft are later deltas, not qualified by1,332checkpoint artifacts. Focused tests/review pending; no residentswitch.
- Measured capacity diagnostics remain above5ms: quiet-start40sample authorityp95 synchronized77.730ms/staggered42.759ms, resident108.625/85.795ms; backgroundcompile appeared afterstart, documented. Sampling localized611/756 (~81%) delayed intention-generation stacks to Facility/Route reserve_delta linear lookups. Narrow sorted-key binary-search optimization and old-linear equivalence regressions are in progress. No100kcompletion or fullcapacity acceptance inferred from short/terminated diagnostic runs.

### Fresh native Chronicle and reservation review — 2026-10-01

The hash-matched 1332-test native checkpoint was launched after the preceding process was confirmed stopped. It reopened the exact saved tick 100 galaxy (Aster Vale energy 160, ore 111, alloy 130). Opening Chronicle exposed one Creator creator_intervention row at tick 100, confirming the latest-boundary reseed in the actual fresh native process. This observation does not qualify later source changes.

Reservation binary-search optimization passed 38 owner focused tests and strict core lint with actual terminal exit 0. Independent executions passed the 8,192-case old-linear equivalence oracle and the logarithmic key-probe bound (one test each, actual exit 0). Source review cleared preservation of proposal/deletion ordering and partially applied private candidates on rejection. Runtime speedup remains unmeasured; the prior 5 ms performance target remains unmet. Standalone dispatch remains Partial and is not resident-path integration.

### Replay-derived Corridor observation pairs — 2026-10-01

Core cache owner and independent focused suites both passed 7/7, actual terminal exit 0; owner pure join passed 3/3, strict core lint exited 0, and related legacy Notebook and Supply suites passed 5/5 each. The private nonserialized cache authenticates zero-dispatch restrictions and every supported MissingInput reason, including Ore when Energy is first. Original vectors and all 17 additive independent verifier modes exited 0 in the refreshed root run.

Root Session now installs/replaces the immutable observation cache after exact metadata and retained-history checks. The Corridor notebook exposes Verify matching corridor observations and exact attempt/wait bookmark links. Source review is clear. Root runtime/UI focused checks and the new host save/Continue journey are pending; this is not yet a verified current-build visible workflow. The paired observations do not establish sole causation or complete the controlled Corridor experiment.

A live rebuilt1332 browser screenshot exposed a scene label overlapping the top inventory readout. The reviewer confirmed readouts are platform Status records omitted from the existing label exclusions; the narrow fix is in progress. A requested viewport override did not change actual reported 1084x610 geometry, so no 800x600 responsive qualification is claimed.

### Corridor presentation and native-store journey checks — 2026-10-01

Root UI focus passed 3/3 with actual terminal exit 0 (/tmp/nyon-corridor-pair-ui-tests.log). The pair test derives and installs the private replay cache, validates semantic pages, activates both displayed PlatformUiAction links and checks exact resulting attempt/wait bookmarks; selection/verification preserve authority. Host runtime/native-store journeys passed 2/2 with actual terminal exit 0 (/tmp/nyon-corridor-native-journey.log). Positive asynchronous verification retains the existing prepared-export bookmark/generation fence and exact canonical metadata, save and fresh Continue restore the tick 20 pair; lone restriction fresh Continue has zero pairs and exact tick 11 authority. The mock transfer captures bytes only and establishes no durable file-transfer result. Independent host rerun remains pending.

Final workspace formatting and independent fuzz formatting checks exited 0. Full workspace tests/strict lint, native release and both browser builds are underway on the frozen source. Their completion is not inferred from focused checks. Standalone dispatch owner 10 tests and strict core passed actual 0; independent dispatch execution and full resident integration remain pending.
