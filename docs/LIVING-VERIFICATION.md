# Living Galaxy verification evidence

Local execution on 2026-10-01, canonical checkout, Mac17,2 / Apple M5.
This file distinguishes verified snapshots from later work. It is not release
acceptance. The full LG-01 through LG-27 boundary remains in
[the completion ledger](LIVING-COMPLETION.md).

## First implemented snapshot

After authority return-credit/replay/history-admission repairs, paced resident
session integration, read-only scene/inspector/Chronicle and selection controls:

| Command | Exit | Evidence |
| --- | --- | --- |
| `cargo fmt --all --check` | 0 | `/tmp/nyon-native-stable-0.log` |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 | `/tmp/nyon-native-stable-1.log` |
| `cargo test --workspace --all-targets` | 0 | `/tmp/nyon-native-stable-2.log`: 909 passed, 57 suites, zero ignored |
| `cargo build --release` | 0 | `/tmp/nyon-native-stable-3.log` |
| separate fuzz workspace formatting | 0 | `cargo fmt --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all --check` |
| separate locked fuzz Clippy | 0 | `/tmp/nyon-fuzz-stable-clippy.log` |
| `./tools/build-web.sh` | 0 | `/tmp/nyon-web-final-0.log`: both backend bundles |
| root wasm library check | 0 | `/tmp/nyon-web-final-1.log` |
| core wasm check | 0 | `/tmp/nyon-web-final-2.log` |
| root wasm library strict Clippy | 0 | `/tmp/nyon-web-final-3.log` |
| `./tools/check-workshop.sh` | 0 | `/tmp/nyon-web-final-4.log` |
| independent Living vector verification | 0 | `/tmp/nyon-web-final-5.log`; unchanged corpus also verified before edits |
| cross-build fingerprint, `--skip-tests` | 0 | `/tmp/nyon-web-final-6.log`; existing frozen RulesV1 fingerprint, not full Living parity |
| vertex layout validation | 0 | `/tmp/nyon-vertex-layout.log`: 36-byte vertex layout |

Baseline workspace tests passed 887 tests in 55 suites. Existing dirty tracked
and untracked work was preserved before edits under
`/tmp/nyon-preserved-20261001-014938`; it remains in the canonical checkout.

The first snapshot release binary SHA-256 was
`36394d05906594cdee9d5fd9488593824f411ea36b51b75f434e34a232c1ee2c`.
WebGPU wasm was
`046aa599bc179af32ca809fc9d01f3ca3d52ec661dcd77e03595e2f1e4edf772`;
WebGL2 wasm was
`b0b31cef6543a2e86960942a0d2bffe587b42bdf48ba3b08bcfde25670ed8e2a`.
Later source changes require fresh builds and gates; these hashes do not qualify
the later canonical history, storage or creator work.

## Live first-snapshot observations

Native: exact release binary copied into an ignored local validation app wrapper;
Apple M5 Metal adapter and first presented frame confirmed. Existing Workshop
startup recovery was dismissed without changing the stored archive. Living opened
paused at tick zero, Step advanced to one, next-world selection changed the
inspector, Run advanced through authoritative freight and combat events, and
returning to the menu preserved the paused resident on re-entry. The old installed
NYON.app was not used as evidence for this artifact.

Google Chrome, local HTTP server on 127.0.0.1:8037: explicit WebGPU initialization
and forced WebGL2 initialization were separately observed. Each backend opened
Living paused at zero through keyboard focus/activation; period stepped and P ran
the simulation. Visible diorama, transport, world inventory and Chronicle matched
the semantic mirror. WebGL2 was later paused at tick 1932 without a transport fault.

These are smoke checks. They do not qualify creator edits, branch comparison,
save/reopen, actual IndexedDB, export/import, screen-reader speech, complete
responsive layouts, offline operation, graphics recovery, or performance at caps.
No screenshots or host storage tests are promoted to those claims.

## Ongoing implementation

Canonical history/archive, independent Living storage, neutral Continue selection,
creator forms and approved content are being implemented after the first verified
snapshot. Their evidence must be recorded separately after source stabilization.
No commits, pushes, PRs, signing, distribution or public hosting were performed.

## Second-phase persistence snapshot and independent review

After the separate Living store, neutral Library coordinator, canonical archive,
and initial creator/content work were added, the following checks were executed
independently on 2026-10-01. These are source-snapshot checks, not full LG release
qualification and not live platform persistence evidence.

| Command | Exit | Evidence |
| --- | --- | --- |
| `cargo test --test library_selection --test living_store` | 0 | `/tmp/nyon-persistence-tests9.log`: coordinator 5/5, store 10/10 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 | `/tmp/nyon-persistence-native-clippy4.log` |
| `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings` | 0 | `/tmp/nyon-persistence-wasm-clippy6.log` |

The storage regression suite exercises exact catalog/archive reopen, generation
CAS, bounded capacities, per-item missing-payload diagnostics, prior archive
recovery, interrupted-before-head writes, cancellation while waiting for an OS
lock, foreign job rejection, and known-reference reclamation preserving both
recovery manifests and unrelated files. An earlier parallel run collided on
clock-derived temporary-directory names; an atomic suffix corrected test
isolation, followed by the passing rerun above.

Subsequent native/browser portable transfer, supporting procedural audio, and
App/runtime integration were independently reviewed against the accepted
experience and operations contracts. Their current compilation or host unit
coverage must not be promoted to picker operation, audible hardware behavior,
actual IndexedDB, download completion, or screen-reader qualification. Browser
export reports only download initiation. The review identified export-pair and
per-file status requirements, transfer hold admission/cancellation ownership,
and audio session-change cleanup as integration items requiring closure and
fresh evidence. These findings were sent to the implementation owner; ongoing
edits and gates may supersede this source snapshot.

Native payload cleanup currently follows durable publication and treats deletion
failure as best effort; failed cleanup lacks retained retry tracking. Archive
replay budgets count atomic operations, while schema/graph validation and a
capacity-sized tick are not subdivided into the normative visited-entity work
units. Both remain explicit Partial qualification boundaries. No second-phase
live native, WebGPU, or WebGL2 end-to-end journey has been recorded here.

### Additive notebook persistence and independent observations, 2026-10-01

- `cargo test --test living_client --test living_store`: exit 0,
  `living_client` 7/7 and `living_store` 12/12, no ignored tests, recorded in
  `/tmp/nyon-notebook-client-tests5.log`. These are native host filesystem and
  client sequencing tests: exact attachment save/Continue, cancellation before
  I/O, invalid witness rejection, metadata preservation, new archive detachment,
  stale generation, byte limit, and interrupted notebook head publication.
- `cargo test -p nyon-workshop-core --test living_additions_vectors`: exit 0,
  3/3 in `/tmp/nyon-observation-vectors-tests.log`. Independent Python formulas
  verify decision, additive creator command, genesis-only fork and sparse
  operational-defense observation full bytes. Original vector verification
  remained unchanged before/after the observation extension.
- Latest wasm strict Clippy reached the newly added root notebook job enum's
  `large_enum_variant` warning in `living_notebook_jobs.rs`; the owned store and
  client replay builders use boxes. This run exited 101 and is not a pass.

These checks establish automated host and wire evidence only. Browser notebook
IndexedDB transactions, native and both browser portable notebook transfer,
keyboard/screen-reader notebook workflows, representative-capacity latency, and
complete platform player journeys remain unverified in this slice.

Notebook source strict gates subsequently passed at the snapshot before the
recorded historical Continue restoration change:

- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: exit 0,
  `/tmp/nyon-notebook-client-native-clippy4.log`.
- `cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`: exit 0,
  `/tmp/nyon-notebook-client-wasm-clippy3.log`.
- Exact wrong-archive attachment binding regression:
  `cargo test --test living_store notebooks_are_exact_generation`: exit 0, 1/1,
  `/tmp/nyon-notebook-binding-tests.log`.

The earlier large notebook-job enum warning was repaired with boxed replay
builders. Bundle builds and live notebook IndexedDB/transfer were not performed
by these commands.

Exact historical Continue follow-up:
`cargo test --test living_client`, exit 0, 8/8, no ignored tests
(`/tmp/nyon-exact-continue-tests2.log`). A recorded neutral reference now restores
the exact retained branch/revision/tick with bounded replay and requires its
recorded digest after validating the notebook against the original stored head.
The newer head/notebook remain retained. Future tick and digest drift still enter
explicit recovery. This is automated native host evidence, not live browser
Continue qualification.

Cross-authority/cancellation follow-up:
`cargo test --test living_qualification --test library_selection`, exit 0,
qualification 7/7 and coordinator 6/6, no ignored tests
(`/tmp/nyon-neutral-qualification-tests5.log`). The new regressions found the
neutral coordinator incorrectly refusing WorkshopV1's first slot ID zero; its
validation now admits zero only for WorkshopV1. Normal and abandoned-then-confirmed
Workshop archive mutations clear their matching neutral selection. Deleting a
Living slot with the same numeric ID preserves the Workshop selection. Explicit
neutral None wins over the legacy V1 marker, and a missing recorded Living slot
enters recovery without falling back to V1. Fake platform transfer regressions
prove unknown-outcome cancellation retains time holds, Idle cancellation drains
and releases time, and late imported notebook bytes do not mutate the resident.
These adapter fakes establish runtime sequencing, not real OS file placement or
browser download completion.

Final owned qualification snapshot strict check:
`cargo clippy --workspace --all-targets --all-features -- -D warnings`, exit 0
(`/tmp/nyon-qualification-final-clippy.log`). Independent additive wire tests
subsequently passed 5/5 (`/tmp/nyon-notebook-vectors-tests.log`), adding a 1488-byte
notebook canonical fixture and construction replacement witness with optional
scrap identity. The notebook fixture uses synthetic catalog/genesis/archive IDs:
it establishes exact wire order and domain-separated integrity, while real
attachment/replay validation is established by separate core/client/store tests.
Original Living vector verification remains unchanged. These facts do not imply
native/browser runtime acceptance.

Independent native field follow-up (2026-10-01):
`cargo test --lib long_native_field_tracks_both_caret_edges_and_preserves_semantics`,
exit 0, 1/1, no ignored tests (`/tmp/nyon-field-independent.log`). The review
confirmed a single glyph-walk boundary table and binary partition searches for
4096-byte draft projection, full semantic value preservation, clamped visible
selection/caret and no authority mutation. This is host rendering-model evidence,
not live native editing, IME, screen-reader or capacity latency qualification.

Independent artifact gate review found that a regular top-level file named
`snippets` could pass when the loader imported no snippets; this was reported to
the owner for a real-directory check. Imported snippet identities, exact file-set
matching and descendant symlink/unreferenced directory rejection were otherwise
consistent with generated IndexedDB glue. No rebuilt artifact or gate pass is
claimed by this read-only review.

Independent persistence review follow-up:
`cargo test --test living_client --test living_store`, exit 0, client 12/12 and
store 13/13, no ignored tests (`/tmp/nyon-persistence-delete-review-tests.log`).
New coverage includes permanent-delete coordinator-before-store ordering,
coordinator CAS failure, deletion failure after Continue withdrawal, exact
cross-authority isolation, notebook cancellation after native worker start, both
manifest/head publication interruption points, and a fake quota rejection after
archive commit. The quota fake establishes partial-save acknowledgement policy;
it does not establish real native disk-full or browser IndexedDB quota behavior.

Private manifest integrity repair:
`cargo test --test living_store --test living_client`, exit 0, store 15/15 and
client 12/12, no ignored tests (`/tmp/nyon-manifest-integrity-tests2.log`).
Private manifest format version 2 hashes
`NYON-LIVING-STORE-MANIFEST-V2\0 || generation LE64 || canonical typed payload`.
Unversioned legacy manifests retain their original checksum reader but are admitted
only when their generation exactly matches an intact head. Corrupt/missing heads
with only legacy candidates enter explicit recovery and preserve files. Read-only
legacy loads do not rewrite; successful mutations emit version 2. Tests reject
tampered new and legacy generations and cover intact legacy reads and write upgrade.
Native and browser recovery source use the same restriction.

`cargo clippy --workspace --all-targets --all-features -- -D warnings`, exit 0
(`/tmp/nyon-manifest-integrity-clippy.log`). No final browser bundle or live
IndexedDB recovery claim follows from these native host checks.

`cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings`, exit 0
(`/tmp/nyon-manifest-integrity-wasm-clippy.log`), compiles/lints the actual browser
manifest recovery restriction. `git diff --check` also exits 0. Both browser
bundles/static gates and real IndexedDB behavior remain separate evidence.

Independent creator capability follow-up:
`cargo test --test living_creator_qualification`, exit 0, 4/4
(`/tmp/nyon-creator-matrix-tests5.log`). Two loops exercise all46 operation forms:
valid public preview/apply plus exact undo/redo/archive reload, and missing target
or invalid local ordinal refusal with unchanged full archive/state. Additional
regressions fill system64 and per-fleet hull16 capacities and distinguish generic
free removal from explicit economic cancellation before ownership transfer.
The sweep exposed RemoveAgreement/ForcePeace missing-participant acceptance;
authority repaired it before the successful run.

Core focused gate exit 0 (`/tmp/nyon-creator-core-review-tests2.log`):
creator7/7, additive creator9/9, queue18/18, independent additions vectors6/6.
No complete all-capacity/scalar/cascade/live matrix is claimed; see
LIVING-CREATOR-MATRIX.md for individual axes and missing cases. Strict workspace
Clippy initially stopped at presentation crest parity lint; owner fixed it and
a fresh check is pending.

Fresh strict workspace Clippy follow-up exited 101 on the in-progress phase3
helper `wire.rs::record_at_mut` being unused
(`/tmp/nyon-creator-matrix-clippy2.log`); authority owner was notified. Latest
independent shipment codec/vector suite passes6/6
(`/tmp/nyon-shipment-kind-vectors2.log`) including immutable dispatch kind,
sparse return-wait target and canonical explicit null/false rejection.


## Third-phase independent input and cursor review, 2026-10-01

Current bounded evidence: `cargo test --lib native_pointer -- --nocapture` exited
0 with 2 tests passed. The tests use actual scaled glyph measurements for caret
placement and drag-anchor extension, and a 4096-byte projected field to verify
original byte offsets and drag-edge clamping. Nonfinite pointer input is rejected.
`git diff --check` exited 0 at this helper snapshot. The separate UTF-8 setter
regression run exited 101 before tests: shared source `sandbox_session.rs:621`
called non-const `LivingGalaxyAuthorityV2::state` inside a const function. This
failure was reported and corrected by the owning workers. The focused rerun
`cargo test --lib pointer_byte_boundary_preserves_invalid_selection -- --nocapture`
then exited 0 with 1 test passed (`/tmp/nyon-pointer-boundary-test.log`). It verifies
invalid middle-byte/out-of-range offsets preserve UTF-8 caret and selection.

Independent source review found root focus synchronization preserves the existing
caret/selection for an unchanged field key/value and installs an editor on initial
focus. Native drag state checks contact ID, field focus and resident presentation
Rc identity, and resets on presentation/focus changes. Root App integration tests
remain pending; this is source evidence, not live native editing acceptance.

The motion-layout oracle now normalizes ink only for the ReducedMotion toggle's
actual text/icon glyph ranges; all geometry and other ink remain exact. Independent
contrast coverage of eight normal/high-contrast/focused/selected states passed in
`/tmp/nyon-living-contrast-tests2.log`, using emitted colors under linear and sRGB
interpretations. The preceding full workspace snapshot failed the old toggle-ink
oracle; no full workspace success is claimed from its focused correction.

Independent phase-3 cursor source review found stable fleet-before-freight arrival
ordering, immutable dispatch-kind credit checks, return-wait recipient tracking,
and one cached node/edge visit per return-path search step. Scheduled lifecycle
and arrival cursors share the caller budget and retain unpublished candidates.
Worker-reported cursor 9/9, economics/delivery 23/23 and strict core Clippy passes
are separate from this read-only review. Forecasts, later phases and canonical
hashing still prevent whole-boundary strict visited-entity budgeting; LG work
budget qualification remains Partial. Native/WebGPU/WebGL2 full journeys and
updated-bundle accessibility/performance acceptance remain unverified here.


## Fresh third-phase browser observations, 2026-10-01

Playwright-owned Chrome 155 on macOS 27.2 (26B5091g), Mac17,2, exercised the
actual loopback application. Root confirmed both backend builds exited 0 before
this journey. Tested wasm hashes: WebGPU
`d02216ef38186a709e4017f83e9ad2752326d99cd09fc50e53092456fe192c0b`;
WebGL2 `2eec6155bdceb9261023a925dbfd7ce711d8f54433827ed912ecc702882821c9`.
These artifacts precede the subsequent CSS viewport and deletion fixes.

Forced `?backend=webgl2` displayed **WEBGL2 LOW initialized successfully**.
Keyboard focus/Space on semantic controls opened Vale, dismissed its introduction,
searched Creator tools, previewed a solar array (23 to 24 facilities), applied,
and ran/paused at tick 168. Aster Vale changed from energy/ore/alloy 0/100/120
to 92/133/136; the inspector displayed the added solar array, output 8 energy
every 10 ticks, and ready extractor/foundry prerequisites. Undo at tick 168
materialized the counterfactual; Redo restored the displayed 92/133/136 values.
Undo plus Branch here preserved an alternative at tick 168 with 76/145/130.
Matching-tick comparison completed, but the chosen comparison row produced
identical left/right digests; a differing-branch comparison remains unverified.

A confirmed Save displayed **Galaxy and Continue selection saved**. Subsequent
browser lifecycle interruption unexpectedly replaced the tab with about:blank;
neither agent initiated it and its cause is unknown. Reacquiring and navigating
to the application loaded **Opened exact saved Living galaxy and notebook**,
tick 168, 76/145/130. This proves recovery after browser session interruption,
not a verified normal quit path. Read-only actual IndexedDB inspection confirmed
`nyon.living.v2`, source slot 1 generation 2, attached notebook and store head 5
after import. No existing database/profile was deleted or corrupted.

Visible Prepare export and three Export controls produced successful Playwright
download handoffs, captured under `.playwright-mcp/qual-webgl2-Galaxy.*`. Product
status correctly retained **disk completion unconfirmed**; download capture
returned no failure. Archive SHA-256:
`dd2641e26d310aadc75f44e36b2345fa0c6a933cd0dff6176f7d1cf1b7cd1b2d`;
catalog file SHA-256:
`f16a7c3c5e6939693c04632ca24677620f59765db8d1a368d59c6e7bb2637e53`;
notebook file SHA-256:
`aed6e5b253d0ef52299c00bd0b3a6f4522680c9c561085b1a92783e0d55a99d1`.
Archive selected branch `9ffa641702707241198bfc3334db3212`, null revision,
tick 168, final digest
`45ebf90b78b0d96636d7fdeec578deada59ee92d00da89148f770eaae403672e`,
catalog `2b391f0585ee573c90c4f2346b108a7d3b23d489e06cc7e732d1e43b93ea7c42`.
Import Galaxy through the file chooser displayed a validated 4-system/8-world/
3-civilization tick-168 preview; Apply created separate slot 2 generation 1,
whose IndexedDB descriptor references the identical archive bytes. This profile
already had the exact catalog; missing/mismatched-pack import remains unverified.

A separate natural-backend tab displayed **WEBGPU initialized successfully**,
secure-context navigator.gpu, and reopened tick 168. Visible creation of First
Expansion started paused tick 0; running/pausing reached tick 2911. This run
overshot tick 411 and does not prove its exact deadline receipt witnesses.

A real logical-pixel defect was observed: CSS/client canvas 1440x900, backing
2880x1800, navigator DPR 1, while Run semantic bounds were y=1744..1788. Screenshot:
`.playwright-mcp/nyon-webgl2-dpr-mismatch.png`. Root implemented a subsequent CSS
viewport fix; it is absent from these artifacts and requires rebuilt pointer/
responsive qualification. Offscreen semantic-mirror locator clicks were replaced
by keyboard activation; no corrected-ratio pointer pass is claimed. Full offline
journeys, exact 1000-tick transfer parity, screen-reader speech, graphics recovery,
remaining fixtures, responsive matrix and full pointer-only journey remain pending.


## Native transfer/Continue and additive facts checkpoint, 2026-10-01

Root-reported live native evidence on release-2 hash
`9e302783cc61e8adab04e818a13ba17c9ce6c52b8a535f45c317a322f0a2442f`:
a native tick-1 save confirmed durability; actual rfd NSOpenPanel selected the
browser-exported archive through Go To. The private import preview showed
4 systems, 8 worlds, 3 civilizations, tick 168 while preserving the tick-1
resident. Apply created a separate save and installed paused tick 168 with
energy/ore/alloy 76/145/130, 14 fleets and one shipment. Native Save & leave
returned to menu; Continue restored these values. Save & leave followed by actual
AX Quit left no NYON process in root's shell inventory; reopening restored the
same paused values. This is a bounded native import/save/Continue/quit/reopen
journey, not full native fixture, pointer-only, accessibility or performance
acceptance. The generic status mentioning notebook does not prove a separately
imported notebook attachment. Native export opened a Save panel but canceled
during automation; no native output artifact is claimed.

Root also executed `/tmp/nyon-browser-export-native-replay.log` with exit 0:
the captured browser archive replayed natively to exact tick 168, final state
digest `45ebf90b78b0d96636d7fdeec578deada59ee92d00da89148f770eaae403672e`
and two bookmarks. This is captured archive/native state replay parity; it does
not establish the next 1000 ticks or arbitrary native/wasm receipt parity.

Independent additive facts qualification at its source snapshot:
`/tmp/nyon-event-facts-tests2.log` exited 0 (5/5), covering complete independent
wire/domain witness, rejected-collector atomicity, entire binding corruption,
resource/reason bounds, actual 2 MiB admission and unchanged legacy ordinary
receipt bytes. `/tmp/nyon-chronicle-event-facts-tests.log` exited 0 (7/7), including
verified actor/entity follow, typed reasons, bad-pair atomicity and unknown legacy
facts. `/tmp/nyon-event-facts-vectors-tests.log` exited 0 (7 additions-vector
tests, 33 wire tests); the empty-state streaming hash matches an independent
complete canonical-state domain witness. Strict core all-target Clippy exited 0
in `/tmp/nyon-event-facts-core-clippy.log`. Original independent corpus verified
before and after with exit 0. These checks preceded the subsequent successful
facility-event producer; its separate new-tag witness and producer gates must
be recorded independently.

Native export follow-up: root completed the real Save panel using keyboard
selection/typing instead of the automation setValue path. Status became
**Galaxy copy saved**. Captured `.playwright-mcp/qual-native-restored168.nyonliving.json`
is 15536 bytes with SHA-256
`dd2641e26d310aadc75f44e36b2345fa0c6a933cd0dff6176f7d1cf1b7cd1b2d`, exactly
matching the earlier WebGL2 export. This is actual portable byte parity for this
saved tick-168 archive and native save-panel completion; it does not extend to
all states, future ticks or receipts.


Successful facility-event additive guard: `/tmp/nyon-production-facts-tests.log`
exited 0 (6/6) before producer emission, pinning exact `facility_produced` phase-5
ordinary framing and a separately sealed actual-resource/deposit-draw witness.
The independent Python new fixture is 649 bytes; the preceding 841-byte facts
fixture remained byte-identical. Original corpus verification before and after
exited 0. This preserves existing tags/domains/state bytes; new successful events
intentionally change receipt/event identities at affected boundaries, as approved
and explicitly documented in proposal section10.1. Producer/end-to-end replay
gates are separate. Private forecast collectors discard explanation serialization
while retaining exact ordinary payload/count/order/capacity; they cannot seal an
explanation sidecar. Their full scheduled-forecast parity regression is owned by
authority and pending here.

Native portable-set follow-up: direct inspection of root's captured files confirms
catalog length 2659 and SHA-256
`f16a7c3c5e6939693c04632ca24677620f59765db8d1a368d59c6e7bb2637e53`, identical
to WebGL2. The native notebook exported before attachment import is 446 bytes,
zero bookmarks/experiments, SHA-256
`dbe7b1f86b10c99f227a7b56eaef6ab0399b13ec35ae3825c8a3299eddc7a27c`;
its difference from the browser notebook is expected. Root then used actual
Import Notebook picker with the original browser attachment; status confirmed
**Exact notebook imported** and History showed two recorded tick-168 bookmarks.
A visible comparison completed with live tick 168 unchanged. This proves that
specific native notebook import/display, not differing-future statistics or
subsequent durable notebook reload. All three native output controls reached
confirmed-save status through the real Save panel.

### Native tracked staging reclamation — 2026-10-01

Current bounded source/test evidence: native Living writes now durably journal each newly created content reference before its payload write. The private journal is limited to 512 references and 256 KiB, binds its base generation and references under `NYON-LIVING-PRIVATE-STAGING-V1\0`, and admits reclamation only for its exact validated references. Recovery preserves objects referenced by either decodable recovery manifest, including a newer manifest whose head commit did not win. Unknown directory entries are never swept. Malformed journals or ambiguous manifests fail closed and preserve files. Reclamation and removal of tracked partial payload temporary files are synced before the journal is discarded. Existing mismatched content files are rejected rather than overwritten.

Executed focused commands: `cargo test --lib staging_tests`, `/tmp/nyon-staging-tests5.log`, exit 0, 3/3; `cargo test --test living_store`, `/tmp/nyon-staging-store-tests2.log`, exit 0, 16/16. The latter includes real injected pre-manifest notebook interruption, subsequent known-orphan reclamation, preservation of the exact slot generation and absent notebook attachment, and successful retry. The final directory-sync ordering adjustment was included in the subsequent 3/3 helper run. Earlier failed attempts were an unavailable `SemanticRole::Tab` in concurrently edited session source, an incorrect integration-test application-root path, and a parallel temporary-root collision; those failures are retained in their logs. This proves deterministic injected interruption handling and bounded private-reference cleanup, not power-loss hardware qualification or reclamation of unknown files left before this journal existed.

The independent GPU extent trace found native surface configuration receives physical window dimensions while App passes a separate logical viewport to rendering. No confirmed physical/logical source mix explains the older padded CUA screenshot; its cause remains unknown pending a fresh native window observation.

### Independent authority foundation review — 2026-10-01

Read-only review of the current immutable boundary captures, atomic ordinary/facts publication, private forecast collector mode, and `LivingRecipeJobV2` found no blocking regression for validated catalog inputs. `LivingBoundaryCaptureV2` retains state, catalog, queue and creator journals through `Arc` handles and checks those publications plus branch/cursor/high-water marks before reuse. Authority stepping seals the ordinary, decision and event-facts receipts before publishing the candidate and clears those sidecars appropriately on paused edits/history changes. Phase five still executes construction completion, hull completion, hubs, solar, extractors, foundries and repairs in that order. Due facilities advance their stored clock once per attempt; disabled and blocked attempts consume no recipe inventory.

The recipe component visits input/output amounts, deposit checks and blocked reasons separately and publishes only a completed private scalar result. It preserves existing multi-deposit semantics: availability is the minimum among matching ore deposits, while depletion targets the first stable-ID matching deposit. A future production index must preserve both facts and update the minimum after first-row depletion. Successful `facility_produced` facts describe the actual input/output catalog amounts and completed aggregate world deposit draw; blocked facts describe the actual persisted reasons. The separate tag changes receipts on boundaries containing successful production; previously encoded tags and state/archive identity formulas remain unchanged.

Remaining Partial boundaries: production still builds entity lists and scans collections synchronously; its recipe helper is driven to completion inside `produce`; forecast still executes all 200 scheduled boundaries synchronously; state validation, commit journal publication and other unsliced phases prevent a whole-boundary 1024-visited-record claim. Retaining a capture can also make later `Arc::make_mut` journal updates clone whole vectors. Public recipe construction presently assumes validated unique catalog amount lists, and its error path lacks the fault latch/failed-visit accounting of the other cursors; no such fault arises from valid bounded catalog inputs in the reviewed component. These are interface/future-integration review observations, not evidence that a complete phase-five job is installed. No native/browser runtime claim follows from this source review.

Independent focused execution for this review: `cargo test -p nyon-workshop-core --lib recipe_cursor`, `/tmp/nyon-independent-recipe-review.log`, exit 0, 2/2. Tests exercise budget 1/64/1024 parity, exact visits, cancellation without exposed payment, every scarcity reason, and unchanged resources when blocked. These two tests do not prove a fully sliced production phase or whole-boundary budget.

### Browser semantic event ownership foundation — 2026-10-01

`platform_web.rs` now queues DOM actions and edits in one chronological stream. Every callback retains its synchronized `SemanticInputOwner`; an identical semantic tree with a different resident/draft/modal owner rebuilds the callbacks. App checks ownership before dispatch. Native browser pointer placement is preserved because editable clicks no longer select the complete input value, and rebuild selection offsets are clamped using UTF-16 code units. Focus/caret restoration occurs only within the same owner context. `cargo check --target wasm32-unknown-unknown --lib` exited 0 in `/tmp/nyon-semantic-queue-wasm-check.log`; this is compile evidence only. Fresh bundle, actual browser pointer/caret and edit/action-order journeys remain unverified for this change.

### Independent repair component review — 2026-10-01

Current bounded component evidence: `LivingRepairJobV2` moves owned fleet/order/path inputs without cloning nested paths, builds world/definition indexes through separately counted visits, and rebuilds fleets/hulls in stable source order. Source review found own-dock eligibility, maximum-health clearing, paid/unpaid repair priority and next-due clocks consistent with the retained test-only synchronous reference. Each inventory payment, hull update and output row is visited separately. Failed work counts its attempted visit, latches the fault and cannot finish; cancellation/NotReady cannot expose partial resources. Rules still drives the subphase polls synchronously, so full phase five and whole-boundary budget remain Partial.

Independent execution: `cargo test -p nyon-workshop-core --lib repair_cursor`, `/tmp/nyon-independent-repair-review.log`, exit 0, 2/2. Tests compare exact reference state across budgets and scarce energy, and prove clock overflow retains private payment without publication. This source/test slice adds no live platform qualification.

### Independent pack durability follow-up (2026-10-01)

Native reclamation now journals the exact payload references of the manifest being replaced before replacement. Recovery preserves objects named by either valid recovery manifest and reclaims only journal-owned, integrity-checked retired objects after their last manifest reference disappears. This closes the interruption window between durable head publication and best-effort retired-file cleanup. The journal is bounded to 1024 references and 256 KiB; unknown directory contents remain untouched. The focused staging suite passed 4/4 (`/tmp/nyon-retired-journal-suite.log`), including a prior-manifest preservation then interrupted retirement/reclamation witness. Hardware power-loss behavior remains unverified.

Pack metadata uses a persisted private global generation high-water mark. A fresh registration and metadata change allocate above that mark; idempotent registration retains its existing generation; deletion preserves the mark. Absent legacy metadata derives its initial mark from existing descriptors without rewriting on reads; an explicitly present mark below an existing descriptor is rejected. Native delete/re-register/archive/reopen rejects the deleted incarnation's held generation (1/1, `/tmp/nyon-pack-aba-test2.log`). Overflow atomicity and legacy/corrupt-mark source regressions passed 2/2 (`/tmp/nyon-pack-highwater-unit2.log`). This cannot reconstruct already-deleted incarnations from older stores that never recorded their generation high-water mark.

The wasm library check passed (`/tmp/nyon-pack-highwater-wasm.log`). This is compile evidence for the adapter path, not real IndexedDB transaction or browser lifecycle qualification of this follow-up.

The final owned persistence snapshot passed the complete native client and store suites: 14/14 client tests and 19/19 store tests, exit 0 (`/tmp/nyon-pack-durability-final-suites.log`). This includes the interrupted re-registration/reopen allocator regression and the corrected notebook orphan lookup. Retirement references are flushed in one bounded batch before manifest replacement, rather than syncing once per prior reference. The final staging suite passed 4/4 (`/tmp/nyon-pack-durability-final-tests.log`; that invocation filtered the separate integration suites, which are proved by the full-suite log above).

### 2026-10-01 09:25 EDT: 100,000-tick native authority endurance

The previously launched `cargo run --release --example living_endurance -- 100000`
completed with actual exit 0 (session 62427), recorded in
`/tmp/nyon-current-endurance-100000.log`. Each fixture advances paired authorities,
checks reproducibility, and validates periodic/final boundaries. Zero civilization
completed in 780 ms; one civilization in 8,128 ms; three civilizations in
13,358,357 ms. The latter finished with 8 worlds, 16 fleets and 4,096 shipments,
state digest `49997da8d1d83d4100f20d584f8cd2ccebe0fe1bc5bda38250254063fc9f13b2`
and receipt digest
`255078cf1b30b3bc4f882d99e3d2d4aab63b0146fabf257a88adb23969b5b938`.
This qualifies that captured native authority snapshot only. It does not qualify
subsequent route/recipe edits, wasm parity, rendering, or the performance target.
The long runtime itself remains a performance concern.

### Notebook attachment and actual-attempt decision disclosure

The public Session attachment regression reproduced acceptance of a notebook
validated against tick2 in a resident with retained tip0 (actual101). The repaired
core proof preserves complete canonical revision records including modes and
branch provenance; Session compares it before changing metadata. Attachment,
Notebook UI and qualification checks passed14/14, actualexit0, in
`/tmp/nyon-experience-notebook-attachment-green.log`. Independent mode/stale graph/
genesis tests passed3/3, actualexit0, in
`/tmp/nyon-independent-notebook-proof-tests4.log`. Selected historical views with
the same retained present graph remain supported. Synchronous graph comparison
is not whole-path work-budget proof.

A validated512world fixture reproduced a decision sidecar39,310,926bytes exceeding
its2MiB limit at50, preserving49 state. Actual-attempt disclosure now preserves
full scoring and conflict fallback while omitting alternatives never attempted
after a category wins. The focused coreunit gate passed84/84, actualexit0, in
`/tmp/nyon-authority-disclosure-tests2.log`, including successful capacitystep50
and ordinary state/receipt/events/entity identity parity with a private test
reference. All-rejected receipt growth and internal search ordinal admission
remain separate unqualified capacity limits. Updatedfullgates/artifacts pending.

### Durable notebook receipt observations, core slice (2026-10-01)

The additive optional notebook evidence section is implemented in the core validator. Old absent-evidence bytes remain unchanged; a section retains its next ordinal even after its records are removed. It accepts at most 128 ordered records and 32 ordered unique event references per record within the 1 MiB notebook cap. Each record resolves an exact named bookmark and reconstructs its receipt, requiring equal tick, state digest, receipt digest, event ordinal, event ID and kind. Autonomous settlement additionally requires an empty selected creator-revision chain; recovery and negotiated/delivery references enforce the reviewed autonomous provenance restrictions. Other references are observations, including verified creator interventions, and do not certify complete experiments, file transfer, parties, quantities or causation.

The core replay now reconstructs the exact paused creator receipt through the authority-owned replay path. This was necessary for actual paused intervention references to survive portable validation; the independent maximum-128-record positive fixture exposed the earlier missing receipt. Evidence tasks do not retain extra full-state clones after verification. Logical archive replay operations and synchronous schema/history scans still do not meet the whole-path visited-entity budget contract.

Executed focused core check: 8/8 existing additive vector tests, 4/4 existing notebook tests and 5/5 new evidence tests passed, exit 0 (`/tmp/nyon-notebook-evidence-tests5.log`). The new tests cover independent populated canonical bytes and integrity, preservation of old absent bytes, mixed running/paused cursor boundaries, wrong receipt/event identity/kind/ordinal, provenance rejection, caps/order/high-water retention, cancellation, and an actual First Expansion autonomous settlement observation. This is core validation proof; Session save/reload, browser IndexedDB and live notebook journeys require their own subsequent evidence.

Focused strict core library plus evidence-test Clippy passed, exit 0 (`/tmp/nyon-notebook-evidence-clippy3.log`), after authority-owned production wiring lint fixes. The unchanged original independent V2 corpus reproduced every row (`/tmp/nyon-notebook-evidence-final-original-vectors.log`), and the new populated notebook fixture verified against the separate Python formula (`/tmp/nyon-notebook-evidence-independent-fixture.log`). These are bounded source/vector checks, not a replacement for the workspace and live platform gates.


### Independent notebook/vector/fuzz checkpoint — 2026-10-01

- Notebook integration review reran `cargo test --test living_notebook_journey --test living_notebook_attachment --test living_notebook_proof --test living_notebook_ui`: actual exit 0, 9 passed across 4 suites, 0 failed, 0 ignored; `/tmp/nyon-review-notebook-integration.log`. Native host save/Continue, portable validation, restored full authenticated references without receipt caches, legacy high-water retention, same-tick edit rejection, and retained-history attachment are exercised. No actual browser persistence or live assistive speech claim follows.
- Independent original formulas: `python3 tools/living-v2-vectors.py verify`, actual exit 0; `/tmp/nyon-review-original-vectors.log`. Original vector/tool paths have no Git diff in this checkpoint.
- Independent additive formulas: all 13 verifier modes (`verify`, creator, genesis-forks, observations, decision-scrap, notebook, notebook-evidence, shipments, decision-route, decision-route-adjust, event-facts, production-facts, empty-state), each actual exit 0; `/tmp/nyon-review-all-additions-vectors.log`. These compare independently calculated canonical bytes/digests against their fixtures, not Rust-generated expected values.
- Separate fuzz workspace formatting actual exit 0: `cargo fmt --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all --check`; `/tmp/nyon-review-fuzz-format.log`.
- Separate locked fuzz workspace strict Clippy actual exit 0: `cargo clippy --locked --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all-targets -- -D warnings`; 32.17 seconds, `/tmp/nyon-review-fuzz-clippy.log`. This is compilation/lint qualification, not executed fuzz campaigns.

Source continued changing for the phase-six observation cursor during this checkpoint. None of these results establishes final source freeze, full platform acceptance, or the complete visited-entity work bound.


Independent full workspace checkpoint: `cargo test --workspace --all-targets` completed with actual exit **0**, **1208 passed / 86 suites / 0 failed / 0 ignored**, `/tmp/nyon-review-workspace-tests.log`. Suite count includes zero-test example targets. The documented Workshop layout capacity/paging test emitted its expected over-60-second warning and completed successfully. This run started while authority observation-cursor work was in progress; it qualifies the captured Cargo-built checkpoint, not later edits or final source freeze. No browser rebuild was performed for this checkpoint.


Independent post-freeze phase-six parity rerun: `cargo test -p nyon-workshop-core --lib observation_cursor_preserves_exact_snapshots_and_nested_visit_budgets`, actual exit **0**, **1 passed / 0 failed / 0 ignored / 87 filtered**, `/tmp/nyon-review-observation-cursor-parity.log`. The strengthened fixture compares the entire observation state to the retained legacy reference at budgets 1, 64, and 1024, checks equal actual visit totals across budgets, verifies invalid budgets consume no visits, and verifies cancellation/premature finish cannot publish. It covers remote Scout visibility after an escort, non-Scout-only dock visibility, travelling Scout exclusion, unseen remembered snapshots, and disabled battery aggregates. The outer boundary still drains this cursor synchronously; no whole-boundary timing/budget claim follows.


Independent full post-phase-six source-freeze checkpoint: `cargo test --workspace --all-targets` actual exit **0**, **1209 passed / 86 suites / 0 failed / 0 ignored**, `/tmp/nyon-review-workspace-tests-post-phase6.log`. This fresh run includes the observation cursor parity/budget/cancellation regression and replaces the earlier pre-freeze workspace checkpoint for the reviewed source. Suite count includes zero-test example targets. This remains automated native host evidence, separate from live browser/native acceptance and later source edits.


Independent opaque Step scaffold rerun: `cargo test -p nyon-workshop-core --lib staged_step_`, actual exit **0**, **4 passed / 0 failed / 0 ignored / 88 filtered**, `/tmp/nyon-review-staged-step-tests.log`. Parity runs 55 boundaries at preparation budgets 1/64/1024 and compares complete state, ordinary/decision/fact receipts, queue cursor, journal/modes and allocator high-water marks against the existing synchronous implementation. Additional tests reject stale prepared success without consuming a newly admitted tail, reject cancellation and premature completion without publication/fault, and verify MAX-tick authentic failure latching and stale failure refusal. The scaffold's remaining completion is explicitly synchronous and off resident transport; this adds no full-boundary responsiveness claim.


Independent creator recovery producer checkpoint: `cargo test -p nyon-workshop-core --lib creator_recovery_proof_`, actual exit **0**, **3 passed / 0 failed / 0 ignored / 92 filtered**, `/tmp/nyon-review-creator-recovery-producer.log`. Coverage includes paused and running proof generation, exact recorded replay proof/receipt equality, no new creator reactivation events, projected queue admission without proof publication, guaranteed rejected command preserving prior proof, newer boundary clearing, transient batch/later-running-loss rejection, and new-actor/preowned-Dormant exclusions. This tests derived evidence without changing civilization status mechanics or canonical wire shapes.

A review invocation initially targeted `living_recovery_evidence` under the core package; Cargo exited 101 because that suite belongs to the root package, before running tests (`/tmp/nyon-review-creator-recovery-notebook.log`). The corrected root suite and separate core notebook suites are recorded below once their actual terminal results are observed; the invocation error is not a game failure.


Independent recovery consumer checks completed:

- Correct root target `cargo test --test living_recovery_evidence`, actual exit **0**, **6 passed / 0 failed / 0 ignored**, `/tmp/nyon-review-creator-recovery-journey.log`. Genuine paused and running creator recovery remains portable without an autonomous reactivation event; tests reject unrelated/transient transitions, new actors, initially Active or preowned-Dormant actors, mismatched same-boundary revision proofs, and loss of the final colony.
- `cargo test -p nyon-workshop-core --test living_notebook_evidence --test living_notebook`, actual exit **0**, **9 passed / 2 suites / 0 failed / 0 ignored**, `/tmp/nyon-review-creator-recovery-notebook2.log`. Existing autonomous evidence, mixed paused/running receipt binding, legacy absent-byte preservation, forged-reference rejection, caps/cancellation and comparison metadata still pass after adding creator recovery admission.

Together with the independent 3/3 producer run above, these are **18 passed across 4 focused suites**, not a fresh full workspace or live platform gate. Source review found no blocking false-proof/publication discrepancy. Exact source and derived receipt proof remain separate from status-rule redesign, full experiment completion, capacity work budgets, native dialog/audio hardware and real IndexedDB qualification.


### Final-colony proof independent rerun attempt

The reviewer independently read the refined final-owned-colony binder and the two-colony survival regression, then attempted `cargo test -p nyon-workshop-core --lib creator_recovery_proof_`. This attempt exited 101 before executing tests because the concurrently changing phase-1 test module contained seven unresolved `super::model` references (E0433). Log: `/tmp/nyon-review-creator-recovery-final-colony.log`. This is a moving-source compilation failure, not evidence of a failed recovery assertion; the refined checkpoint needs a fresh focused rerun after the owner freezes it.


### Frozen phase-1 and final-colony focused qualification

After the authority freeze and compilation fix, independent `cargo test -p nyon-workshop-core --lib staged_step_` exited 0: 7 passed, 0 failed, 0 ignored, 91 filtered out. This includes source/hash budget parity, phase-1 local creation and 1,024 scalar edits, explicit creator fallback recovery, private insertion fault parity, stale capture, cancellation, and maximum-tick failure. Log: `/tmp/nyon-review-phase1-staged.log`.

Independent `cargo test -p nyon-workshop-core --lib creator_recovery_proof_` also exited 0 after the prior compile-only failure: 3 passed, 0 failed, 0 ignored, 95 filtered out. Log: `/tmp/nyon-review-creator-recovery-final-colony2.log`. The final-colony survival refinement is now exercised rather than source-reviewed only. These focused checks do not establish live platform or whole-executor budget acceptance.


### Frozen phase-1 full workspace checkpoint

Fresh `cargo test --workspace --all-targets` reached terminal exit 0 after the authority phase-1 freeze: **1,227 passed across 88 suites, 0 failed, 0 ignored**. The suite count includes zero-test example targets. The known long `short_modal_pages_reveal_every_field_and_maximum_reference_blockers` capacity test completed successfully; its 60-second warning was not a failure. Log: `/tmp/nyon-review-workspace-post-phase1.log`. This qualifies the frozen phase-1 and final-colony recovery source checkpoint, before the proposed supply/trade fact implementation. It is automated native test evidence, not a fresh release/browser build or live platform acceptance.


### Scheduled phases 2/3 frozen focused qualification

After the authority owner froze lifecycle/arrival integration and cleanup, independent `cargo test -p nyon-workshop-core --lib staged_step_` reached terminal exit 0: **10 passed, 0 failed, 0 ignored, 91 filtered out**. Added coverage verifies mixed-budget expiry/arrival/delivery/state/receipt parity, cancellation and new queue tails during both scheduled phases, and sticky lifecycle arithmetic failure preserving the last boundary. Existing seven Step tests also pass on this checkpoint. Log: `/tmp/nyon-review-staged-scheduled.log`. This focused result does not supersede the prior full workspace checkpoint or establish a complete bounded executor; phases 4–10 and final validation/sealing remain synchronous.


### Combat component independent focused checkpoint

Independent `cargo test -p nyon-workshop-core --lib combat_cursor_` reached terminal exit 0: **3 passed, 0 failed, 0 ignored, 101 filtered out**. Tests cover legacy-reference simultaneous damage/destruction parity, disabled/dead battery and transit axes, budgets/cancellation and sticky arithmetic failure. Log: `/tmp/nyon-review-combat-cursor.log`. This qualifies the standalone component checkpoint; it is not full phase-4 integration. After this run the owner reported a narrow failure-path retention change moving dead-battery blocked reasons into retained ownership before fallible destruction emission; this result must not be represented as independently exercising that subsequent change.


### Integrated typed trade observations independent focused qualification

Independent `cargo test --test living_trade_ui --test living_trade_evidence --test living_chronicle` reached terminal exit 0: **16 passed across three suites, 0 failed, 0 ignored**. Chronicle: 8; trade evidence: 7; native host UI journey: 1. The suites exercise actual Commerce parties/delivery quantities, storage waiting followed by complete acceptance, captured/returned/legacy manifests, same-world and capacity-missing Notebook rejection, 4,096 actual delivery capacity preservation, executor slice parity, explicit omission presentation, and Record formation/delivery → save-and-leave → fresh Client Continue → exact replay evidence. Log: `/tmp/nyon-review-trade-integrated.log`. This is native automated/host journey evidence, not live native GUI or real browser IndexedDB qualification, and records separate observations rather than linked experiment completion.


### Final combat retention and complete-envelope guard independent checks

After the dead-battery blocked-reason retention adjustment, independent `cargo test -p nyon-workshop-core --lib combat_cursor_` again reached terminal exit 0: **3 passed, 0 failed, 0 ignored, 101 filtered out**. Log: `/tmp/nyon-review-combat-cursor-final.log`. The corrected source retains nested reasons before fallible HullDestroyed emission.

Independent `cargo test -p nyon-workshop-core --test living_trade_facts direct_constructed_sidecar_checks_full_envelope_even_when_integrity_payload_fits` reached terminal exit 0: **1 passed, 0 failed, 0 ignored, 8 filtered out**. Log: `/tmp/nyon-review-full-envelope-guard.log`. The regression constructs a sidecar whose integrity payload still fits while the complete envelope exceeds 2 MiB, independently reseals its payload, and verifies rejection. This exercises the direct-object verification boundary, rather than relying only on bounded decode or collector admission.


### Linked Trade, first fit and retreat independent focused checkpoint

Independent `cargo test --test living_trade_completion --test living_initial_fit --test living_camera` reached terminal exit 0: **14 passed across three suites, 0 failed, 0 ignored** (linked Trade 7, initial fit 3, camera 4). Log: `/tmp/nyon-review-linked-and-fit.log`. Linked checks cover actual Commerce without baseline, reversed delivery direction, sibling branches, forward/reversed creator prefixes, same-tick/wrong-party/expiry, removed/replaced agreements, and preexisting cargo with no dispatch causation claim. Geometry checks cover first input before rendering, centered asymmetric scope fitting, retained framing across resize/steps, responsive extents and rejected inputs preserving authority. This is automated proof/geometry evidence; live UI/persistence of the linked claim retain separate boundaries.

Independent `cargo test -p nyon-workshop-core --lib retreat_cursor_` reached terminal exit 0: **4 passed, 0 failed, 0 ignored, 104 filtered out**. Log: `/tmp/nyon-review-retreat-cursor.log`. It covers exact equal-cost path/destination ties across slice budgets, eligibility/local/direct/disconnected axes, long paths and 512-element replaced-order retirement, and cancellation/deadline failure retaining unpublished ownership. This qualifies the standalone retreat checkpoint, not complete conflict/staged integration.


### Local verification independent checkpoint and fit attempt

Independent `cargo test --lib living_notebook_jobs` reached terminal exit 0: **8 passed, 0 failed, 0 ignored, 179 filtered out**. Log: `/tmp/nyon-review-local-notebook-jobs.log`. Includes exact metadata/canonical/dirty preservation on local verification, stale/cancel rejection, export capture and handoff/cancellation guards.

The separate `cargo test --test living_initial_fit` attempt exited 101 before tests: concurrently changing core Notebook source omitted the new supply_completions field in its ValidatedLivingNotebookV2 initializer (E0063 at notebook.rs:797). Log: `/tmp/nyon-review-invalid-focus-fit.log`. This is a moving-source compilation failure, not a failed fit assertion. A fresh initial-fit rerun is required after that initializer checkpoint compiles.


### Measured scene labels and recovered focus checkpoint

The assigned label implementation replaces byte-width estimation and renderer re-culling with one atlas-measured placement pass: full emitted SDF-cell extents, selected-first ordering, alternative nearby anchors, full canvas containment and visible marker-body avoidance. Scene projection stores its canvas bounds; no authority or marker/hit geometry algorithm changes. Three new atlas/geometry tests inspect actual emitted glyph quads, alternate edge/body anchors, proportional advances, deterministic priority and uncontainable-name omission.

`cargo test --test living_scene_labels --test living_presentation` reached terminal exit 0: **12 passed across two suites** (3 labels, 9 presentation), 0 failed/ignored. Log: `/tmp/nyon-measured-scene-labels.log`. A final rerun captures the last canvas-metadata synchronization line separately.

Fresh `cargo test --test living_initial_fit` reached terminal exit 0: **4 passed, 0 failed, 0 ignored**, including removal of a focused entity and one-time surviving-scope refit. Log: `/tmp/nyon-review-invalid-focus-fit2.log`. This supersedes the earlier moving-source E0063 fit attempt. Scoped `cargo clippy -p nyon --lib -- -D warnings` reached exit 0 (`/tmp/nyon-measured-label-clippy.log`); scoped rustfmt check of presentation/app/new test files reached exit 0 (`/tmp/nyon-measured-label-format.log`). These are automated geometry/scoped-lint results, not live visual or full platform acceptance. Independent review of this implementation is still required.

Final measured-label checkpoint rerun `cargo test --test living_scene_labels --test living_presentation` reached terminal exit 0: **12 passed across two suites, 0 failed/ignored**. Log: `/tmp/nyon-measured-scene-labels-final.log`. Source is frozen for independent review and combined gates.


### Independent occupation/settlement checkpoint — 2026-10-01

- `cargo test -p nyon-workshop-core --lib occupation_cursor_`: actual exit 0; 4 passed, 0 failed, 0 ignored, 112 filtered out. Log: `/tmp/nyon-review-occupation-cursor.log`.
- `cargo test -p nyon-workshop-core --lib settlement_cursor_`: actual exit 0; 3 passed, 0 failed, 0 ignored, 113 filtered out. Log: `/tmp/nyon-review-settlement-cursor.log`.
- Source review covers captured ownership, complete-or-zero refund credit, stable third-party/arbitration ordering, arrival-window snapshots and counted temporary retirement. Standalone component evidence only; no additional full-core gate was started alongside the owner gate.


### Independent latest invalid-focus recovery — 2026-10-01

`cargo test --test living_initial_fit`: actual exit 0, 5 passed, 0 failed or ignored. `/tmp/nyon-review-invalid-focus-fit3.log`. Includes removal after selection changes to a surviving world, recovered fit once, later pan retention and immutable committed bookmark. This supersedes the earlier queued result for session 91679.


### Independent Supply completion and host journey — 2026-10-01

- `cargo test -p nyon-workshop-core --test living_supply_completion`: actual exit 0; 5 passed, 0 failed or ignored. `/tmp/nyon-review-supply-completion.log`. Covers both observed topologies, exact actors/endpoints/systems/quantities, stale/forged receipt rejection, same-system/resource rejection, temporal and endpoint mismatch, sibling branches and reversed revision prefixes.
- `cargo test --test living_supply_ui`: actual exit 0; 1 passed, 0 failed or ignored. `/tmp/nyon-review-supply-ui.log`. Covers local Verify, save, resident destruction and fresh Continue retaining visible Supply proof through native host storage/replay. This is host integration evidence, not live GUI or IndexedDB acceptance.
- These terminal results supersede the queued Supply-test note in the accompanying source review.


### Independent conflict composite — 2026-10-01

`cargo test -p nyon-workshop-core --lib conflict_composite_`: actual exit 0; 2 passed, 0 failed or ignored, 116 filtered out. `/tmp/nyon-review-conflict-composite.log`. Covers complete-reference output/retreat ordering, shared child budget, cancellation in each child and sticky failure. Standalone component evidence only; no duplicate whole-core or workspace gate started.


### Independent phase-five/six cleanup qualification — 2026-10-01

- `cargo test -p nyon-workshop-core --lib cleanup_`: actual exit 0; 4 passed, 0 failed/ignored, 118 filtered. `/tmp/nyon-review-phase-cleanup.log`.
- `cargo test -p nyon-workshop-core --lib production_cursor_`: actual exit 0; 1 passed, 0 failed/ignored, 121 filtered. `/tmp/nyon-review-production-cleanup-parity.log`. Complete production reference ordering/parity across visit budgets.
- `cargo test -p nyon-workshop-core --lib observation_cursor_`: actual exit 0; 1 passed, 0 failed/ignored, 121 filtered. `/tmp/nyon-review-observation-cleanup-parity.log`. Exact observation snapshots and nested budgets.
- `cargo test -p nyon-workshop-core --lib production_fault_`: actual exit 0; 1 passed, 0 failed/ignored, 121 filtered. `/tmp/nyon-review-production-cleanup-fault.log`. Sticky arithmetic failure after private inventory change, identical total visits across budgets and no publication.

Standalone components/cleanup evidence; no duplicate whole gate or resident integration claim.


### Independent private composition and root Notebook refinements — 2026-10-01

- `cargo test -p nyon-workshop-core --lib staged_step_`: actual exit0;14 passed,0 failed/ignored,115 filtered. `/tmp/nyon-review-staged-phase6-composite.log`.
- `cargo test --lib unicode_display_tests`: actual exit0;3 passed,0 failed/ignored,187 filtered. `/tmp/nyon-review-word-chunks-final.log`.
- `cargo test --test living_notebook_ui`: actual exit0;3 passed,0 failed/ignored. `/tmp/nyon-review-notebook-actions-first.log`.
- `cargo test -p nyon-workshop-core --lib corridor_observation`: actual exit0;3 passed,0 failed/ignored,126 filtered. `/tmp/nyon-review-corridor-pure-join.log`. Pure helper only; producer admission/replay integration unimplemented at this checkpoint.

No whole gate duplicated. Caption geometry calculation identified scale1.5 truncation at320x480 separately from these passing tests.


### Narrow short-title geometry — 2026-10-01

Independent source/embedded-atlas calculation for320x480: short title uses one line at scales1 and1.5; measured advances159.118652/238.677978 <=264 pixels; Body line heights20.25/30.375 <=title heights44.01/66.015. Full scope text remains in semantic description and paged visible facts. This supersedes the prior longer-title geometry defect for these requested sizes; no additional full gate or live UI acceptance claimed.


### Independent Corridor producer terminal result — 2026-10-01

`cargo test -p nyon-workshop-core --lib corridor_restriction_producer_tests`: actual exit0;2 passed,0 failed/ignored,134 filtered. `/tmp/nyon-review-corridor-producer.log`. Covers actual partial/zero ion-storm attempts and rejection of shortage, suspension, ownership, capacity and expired-hazard cases. This supersedes the earlier queued producer run; atomic authority publication/replay admission remains a separate evidence boundary.


### Occlusion-aware scene labels — 2026-10-01

`cargo test --test living_scene_labels --test living_presentation`: actual exit0;13 passed across2 suites (4 label tests,9 presentation),0 failed/ignored. `/tmp/nyon-occluded-label-tests.log`. New regression renders actual atlas glyph quads and verifies no intersection with a control, alternative anchoring, unchanged selection and full-occlusion omission without entity removal. Existing empty-exclusion API parity and full glyph containment remain green. `rustfmt --edition2024 --check src/presentation/living.rs src/app.rs tests/living_scene_labels.rs`: actual exit0; `/tmp/nyon-occluded-label-format.log`. Scoped strict Clippy pending separately at this writing. No authority/camera/hit geometry change or live visual acceptance claimed.


Occlusion scoped strict result: `cargo clippy -p nyon --lib -- -D warnings` actual exit101; `/tmp/nyon-occluded-label-clippy.log`. Core dependency compilation rejected three type_complexity diagnostics in moving Corridor publication return tuples in simulation.rs. No label lint result can be claimed from this failed dependency gate. Root integration owner notified; source preserved.


Occlusion scoped strict rerun after the integration-owner tuple alias fix: `cargo clippy -p nyon --lib -- -D warnings` actual exit0; `/tmp/nyon-occluded-label-clippy-final.log`. This supersedes the dependency-blocked first lint. Label source remains frozen at focused13/13 and format exit0.


2026-10-01 independent Chronicle restoration: `cargo test --test living_chronicle`, session 74685, actual exit 0, 10 passed / 0 failed / 0 ignored. Log `/tmp/nyon-review-chronicle-restoration.log`. Covers latest-only replay restoration and exact bookmark replacement; no full historic Chronicle reconstruction claim.

2026-10-01 independent measured-reservation optimization: focused oracle session52868 actual exit0, 1 passed/144 filtered; key-count session72414 actual exit0, 1 passed/144 filtered. Logs `/tmp/nyon-review-reservation-binary.log`, `/tmp/nyon-review-reservation-logbound.log`. Tests cover8192 exact linear-reference outcomes/partial mutations and1024-row logarithmic key-visit bound; no performance remeasurement claim.

2026-10-01 independent Corridor pair-cache suite session45675 actual exit101:6 passed/1 failed. `/tmp/nyon-review-corridor-pair-cache.log`. Failure: simultaneous-shortage fixture illegal hub energy_next_due100 vs genesis expected10 before construction. No full green claim; owner notified to repair valid fixture then rerun.

2026-10-01 corrected independent Corridor pair-cache suite session35973 actual exit0:7 passed/0 failed/ignored. `/tmp/nyon-review-corridor-pair-cache2.log`. Valid Running inventory intervention19 replaces illegal genesis clock; allbudget replay/canonical parity, genuine Energy-first/Ore-second resource matching, isolated/cancel rejection pass. Supersedes prior fixture failure, not whole experiment or Session UI acceptance.

### Readout label regression checkpoint — 2026-10-01

`cargo test --test living_scene_labels` returned **101**: four passed and one failed. The new regression reproduced the reported normal-scale inventory overlap, but incorrectly required the old overlap at UI scale 1.5 too, where existing enlarged chrome already excludes that anchor. Only the defect-reproduction assertion was narrowed to scale 1; complete glyph/readout nonintersection still runs at both scales through production `measured_labels_for_ui`. Fresh test is pending in `/tmp/nyon-readout-label-tests2.log`; scoped strict lint is pending in `/tmp/nyon-readout-label-clippy.log`. Final production and test files formatted successfully; scoped formatting check and `git diff --check` returned 0 before the assertion-only correction. No fresh live browser claim.
