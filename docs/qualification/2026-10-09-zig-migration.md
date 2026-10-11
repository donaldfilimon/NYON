# NYON Zig 0.18-dev migration

Objective: complete NYON game refactored in Zig 0.18-dev. Whole-product status:
**Partial**. A headless Classic authority is the first implementation slice;
it does not satisfy the complete-game objective.

## Authoritative baseline and ownership

Work is in canonical `~/dev/active/NYON` on `main`, starting at
`bac2b6e313f8034309bd718d3b8cb4e0448ac20f`. Existing uncommitted Living changes
in `event_facts.rs`, `receipt.rs`, `simulation.rs`, `simulation/retirement.rs`,
and the October 8 completion ledger are a separate work stream. This migration
does not edit those files. No commit, push or PR is authorized by this task.

The accepted Workshop specification and master plan identified by `AGENTS.md`,
the Living rules and LG-01 through LG-27 qualification specification, the
approved Complete V2 then V3 program, and the newest completion ledger retain
their scope. The current Zig instruction replaces the earlier implementation
language requirement. Existing source/tests supply migration comparison
oracles, not permission to drop accepted behavior or leave existing gaps open.

Current compiler: `0.18.0-dev.120+9fe22a29b`, pinned in `zig/.zigversion` and
enforced by `zig/tools/check.sh`. The actual Rust oracle toolchain is
`nightly-2026-10-08` in `rust-toolchain.toml`; the September 1 wording in the
repository guide is stale. Existing guides remain untouched.

## Complete requirement inventory

| Requirement | Binding source / contract | Final proof required | Zig status |
| --- | --- | --- | --- |
| Language/runtime migration | Current user objective; all production entrypoints | Actual complete game builds/runs on pinned Zig, with no residual Rust production runtime | Partial |
| Classic authority | `src/game/{model,simulation,view}.rs`, `tests/{campaign,rules_v1_facade}.rs` | Frozen seven-world generator, 60 Hz phase order, command/AI/economy/combat/hazards/outcome parity, seed `4947573120260902`, base speed 23, digest `67D96E983D6C9330` | Authority gate passed; graphical/runtime view remains open |
| Scenarios/preferences/editor | `src/scenario`, `src/preferences`, `src/editor`, corresponding integration tests | Exact codecs and identities, 65,536-byte scenarios, 16 KiB preferences, strict validation, detached invalid drafts, failure preservation, primary-slot/read-only legacy precedence | Pure codecs and native store adapter reviewed; CLI/runtime/editor integration in progress |
| Classic player experience | `src/app`, `src/presentation`, `docs/PLAYER-MANUAL.md` | Preview/confirmation, editor, guide/onboarding, settings, pause/speed/camera/touch, persistence | Pure clock/session lifecycle reviewed and gated; controller/editor/visual/runtime port open |
| Workshop pure authority | Core `ids/pack/model/command/simulation`, accepted Workshop design | Strict packs, typed IDs, checked arithmetic, all 14 creator operations, 10 Hz six-phase economy, atomic batches/faults | Open |
| Workshop history/archive | Core `history/archive`, Two-System Forge and integration suites | Revision DAG/branch siblings/explicit redo, same-tick undo/redo identity, canonical packs/archives, replay; complete 18-step Forge and real alloy production | Open |
| Workshop Library/storage/client | `src/workshop`, `src/app/client_runtime/library.rs`, `src/ui/library`, Library addendum | Exact catalog, 16 slots, archive lifecycle, generation CAS, predecessor recovery, cancellation, import/export, focus preservation | Open |
| Living V2 wire/schema/genesis | Core `living/{ids,wire,catalog,model,genesis}`, Living rules and independent vector tools | Full schema/validator/corpus parity, isolated V2 domains, strict 2 MiB packs and 32 MiB archives | Open |
| Living V2 game mechanics | Core Living simulation/rules/commands/receipts/jobs/decisions/facts | Paid industry/hulls/repair, observation/choices, energy/travel/freight, diplomacy/retreat/simultaneous combat/claims/occupation/dormancy | Open |
| Complete bounded executor | Core Living jobs/history/archive/retirement, root sandbox/runtime | All ten phases, validation/hashing/receipts/journals/replay/fork/restore/compare/publication/retirement share real allowance 1..1024; at most 20 requests/publications; cancelled/stale work publishes nothing; extraction exactly once | Open; existing synchronous suffix must be replaced |
| Living creator tools | `src/living_creator.rs`, current creator schema/tests and accepted Living rules; draft creator addendum is context | All 46 tools reachable by pointer/keyboard/semantics; structural validation, preview/apply/cancel/stale, cascade review, undo/redo/save/reopen | Open |
| Content/notebook/history | Root Living content/chronicle and core notebook/history/facts | All production templates including Vale Confluence and seven authenticated experiments; First Expansion sequence to tick 361 and release journey to 411 | Open |
| Living durable storage/global Continue | Living store/selection/transfer/runtime modules | 64-slot versioned domain, native and real IndexedDB atomicity/CAS/recovery/quota/interruption, exact branch/campaign selection, notebook authentication, portable readback, deletion/reclamation | Open |
| UI/renderer/navigation/audio | Root engine/ui/presentation/Living navigation/camera/audio, shaders/atlas/loader | Full diorama/hierarchy/inspectors/history/editor; 44 logical-pixel targets; semantic/focus/layout parity; three navigation scales; Low/High resilience; mute/motion/contrast; presentation does not alter truth | Open |
| V3 completion | Complete V2 then V3 plan and eight-spec package/initialization design | Current independently derived corpus, integrated review and immutable freeze; separate versioned authority/storage/identity; direct Genesis/authenticated post-root history; societies/research/ecology/diplomacy/conflict; full Sessions/UI/content/qualification | Open; prior approval does not establish implementation |
| Strategic cubic planets | Newest completion ledger strategic-world section | Reviewable design followed by searchable galaxy/system/six-faced planet views, Continue/Templates/Demos, presets to 64 systems/512 worlds, authentic guided demos and persistent graphics/camera/accessibility/audio settings | Proposed architecture; implementation open |
| Whole-product acceptance | Living qualification LG-01..LG-27 and accepted Workshop acceptance | Final Zig artifact identities, independent review, complete live native/browser/input/accessibility/storage/offline/performance/endurance matrix | Open |

Workshop limits remain 16 factions; 64 systems/128 stars/512 worlds; 256 lanes;
1,024 deposits; 2,048 industries/routes; 4,096 shipments; 128 hazards;
128 operations and 64 KiB per batch; 64 branches/10,000 revisions;
16 slots/16 MiB archives/1 MiB packs. Built-in solar yields four energy/tick;
extractor consumes one energy/two reserve for two ore; foundry consumes two
energy/three ore for one alloy. Ion storms floor-halving route capacity.
Living additionally retains 16 civilizations, 256 fleets/2,048 hulls/16 hulls
per fleet and 1,024 creator/running-command limits per boundary.

Living phase order remains queued creator → expiry/hazards → arrival/freight →
retreat/combat/occupation/claims → production/repair → observation → diplomacy →
centralized intent reservation → dispatch → receipts/hash/commit.

Final acceptance must exercise all 46 creator tools and seven experiments,
all-tool cancellation/stale/durable Continue, capacity/generation conflicts and
device-full storage, portable transfers, network-disabled cold new/open/play/
edit/branch/save/export/import, keyboard/focus/screen reader/Retina/touch/pinch,
and muted/reduced-motion equivalents. Required viewports include 723×802 at
100/115/130% and 1280×720, 1440×900, 1920×1080 at 100/130%.
Required hosts/backends include macOS Metal/Safari/Chrome, Windows 11 DX12/
Edge/Chrome, and Ubuntu 24.04 Vulkan/GL/Firefox, with natural and forced
WebGL2 paths. Performance targets retain capacity authority p95 ≤5 ms,
native/WebGPU frame p95 ≤16.7 ms, WebGL2 Low ≤33.3 ms and input ≤100 ms,
36,000-tick mixed/replay and 100,000-tick endurance with bounded memory.
Unavailable host/manual evidence remains unverified.

## Implementation ordering

1. Pin compiler and port complete Classic model/generation/tick authority,
   preserving the Rust oracle and frozen values while adding Zig fault tests.
2. Port exact scenario/preferences codecs, replay identities and detached editor,
   then storage and actual Classic visual/runtime experience.
3. Port Workshop pure authority/history/archive and the complete Forge journey.
4. Port Living wire/catalog/schema/genesis and all mechanics. Complete its
   bounded execution path during the port, including shared replay and cleanup.
5. Port platform storage/coordinators/transfers/Sessions and full
   UI/rendering/input/semantics/audio/content. Keep exercising vertical journeys.
6. Complete V3 corpus/review/freeze and implement separate versioned Zig domains;
   complete the strategic-world experience after its design is reviewable.
7. Qualify the actual final Zig game against every accepted requirement and
   host/runtime acceptance row, then retire Rust production paths.

## Current verification procedure

From `zig/`, run `./tools/check.sh`. It verifies the exact compiler, formatting,
uncached Debug test executable with leak checking, ReleaseSafe tests/build,
native profile-injected CLI/process qualification, then differential Classic
traces against a freshly built current Rust oracle.
The parity runner compares every canonical digest, ordered command result and
ordered event/payload for six seeds in autonomous and scripted modes. The
scripted mode submits out-of-order launch/tuning commands and invalid actions.
It separately checks the frozen genesis digest. It reports both trace rows and
actual tick transitions, so absorbing finished campaigns do not inflate tick
coverage. These comparisons are migration evidence, not independent Living
vector or whole-product acceptance.

Rust oracle regression uses the four native commands in `AGENTS.md`; the new
example does not change production Rust code. Wasm/fuzz/independent Living
vectors/shader layout/source freshness/live platform acceptance remain separate
from the new Zig gate. Runtime data, secrets and existing saved games are not
used by the terminal slice.

Earlier Rust qualification in the October 8 ledger cannot establish Zig
completion.

## Observed Classic gate on 2026-10-09

The earlier scoped Task1 command `./zig/tools/check.sh` returned **exit 0** after scenario
fix round 1. Its fresh Debug executable passed **81 tests** with allocator leak
checking. ReleaseSafe tests and build also succeeded. Signed coordinate `-0`
now rejects like the current Rust codec; detailed scenario failures retain
canonical field/kind, owned validation reports and underlying parser causes.
Scenario encoding and simulation admission revalidate mutable values and reject
stale fingerprints, invalid IDs or initial remainders before installation.

Six seeds in autonomous and scripted modes matched **144,012 rows**, representing
**144,000 actual tick transitions** and **2,038 events**. All **323 codec cases**
matched canonical output bytes/identities or rejection classes. Trace SHA256 is
`23f714b8fc527920102100bcf7422d6aeb96e1b9f52f42c84c85d1ea869253aa`;
codec SHA256 is
`5d34cf289a76097efa3537b99f4a362b7e2410667071fb91967fa3a6d5473f6c`.
The runner rebuilt both artifacts and confirmed unchanged input/artifact hashes:

- Rust oracle: `20f2d7d1690dff43c2726987d6f82e926d8ff25232cd18671e2808643bce60de`.
- Zig executable: `2cd54445305c8dcf36672a9591be4fa73a6299ff2ed06d98f923c2f7eeef5c78`.
- Source inputs: `fad4814002286a8d8d4cba27b76d8aa2ce525a533ce22b3b5bdacb762b277730`.

Detailed failure payloads and installation preservation have focused unit
coverage; codec differential rejection-class comparisons do not independently
prove every diagnostic payload. Trace comparisons use canonical state hashes
and complete ordered command/event payloads, rather than complete state bytes.
Terminal interaction has separate bounded-input and command journey evidence.
Independent scoped specification/quality review accepted both original findings
and the checked admission changes. Task1 is complete within this bounded scope.
No graphical, browser/native storage, accessibility, host performance, Workshop,
Living, V3 or whole-game completion follows from this gate.

The earlier native Rust production gate returned exit 0 for formatting, strict
workspace/all-targets/all-features Clippy, workspace/all-targets tests and
release build; tests reported 1,850 passing across 102 success summaries, none
failed or ignored. The subsequently extended oracle example passed a current
formatting check and targeted strict Clippy; the fresh parity gate compiled its
current codec modes. These evidence scopes remain distinct. Existing Living
edits remain a separate work stream and no commit, push or PR was made.

## Observed scoped Classic store adapter on 2026-10-09

Task 2A native/raw stores and owned typed outcomes are implemented in
`zig/src/classic_store.zig`. After the two Windows lexical path findings were
fixed, formatting passed and standalone Debug, ReleaseSafe and ReleaseFast
suites each returned exit 0 with 41 passing tests (35 store tests plus 6 imported
model/generator tests), no skips. Independent scoped re-review approved both
fixes and reported no introduced findings. Native tests use owned disposable
profiles and cover capped readback, atomic replacement failures, absent-only
read-only legacy fallback, independent namespaces, OOM cleanup and typed
preference recovery. Scenario raw-save source asymmetry remains uncapped;
validated CLI replacement is a separate integration boundary.

Windows tests exercise lexical bytes/constructors on macOS. An installed Rust
1.98.0 prefix-parser harness matched 39 acceptance vectors; it is not selected
nightly or Windows filesystem execution. Unix opaque profile bytes are retained
by the resolver, but actual macOS creation of the 0xFF fixture failed with host
path rejection. Files are synced before atomic replacement; parent-directory
power-loss durability is not established. Browser persistence, actual Windows/
Linux filesystem execution, GUI apply and full runtime/store integration remain
Pending. Task 2B CLI/process integration is now independently reviewed and passed its
aggregate gate; its exact evidence is recorded below.

The source/SDK platform inventory is saved separately in
`docs/qualification/2026-10-09-zig-platform-binding-inventory.md`. It establishes
available local headers/libraries, not linked bindings or any GPU/runtime pass.

The current Living source decomposition is recorded in
`docs/qualification/2026-10-09-zig-living-inventory.md`. Original independent
identity-corpus verification (`python3 tools/living-v2-vectors.py verify`)
returned exit 0 with unchanged tool and corpus hashes. This is base-corpus
formula consistency; additive/nested full bytes, Zig mechanics, complete
bounded execution and real runtime adoption remain open.

## Observed Task 2 aggregate gate on 2026-10-09

`./zig/tools/check.sh` returned terminal exit 0. Debug passed 121 tests,
ReleaseSafe tests/build passed, and the fresh CLI qualification passed 57
process checks with no skips. They exercise positive save/load/reopen, invalid
input and cap/UTF-8 recovery, absence without writes, namespace isolation,
profile rejection and real replacement failure with sentinels preserved.
Independent Task 2B review approved both specification compliance and quality
with no findings. The reviewed candidate and all 16 protected source identities
remained unchanged through the gate.

Fresh Rust/Zig trace and codec comparisons still matched 144,012 rows,
144,000 actual tick transitions, 2,038 events and 323 codec cases. Trace/codec
hashes match the earlier Task1 evidence. Each runner builds and fences its own
artifact: CLI SHA256 is `783ba7023229126b26abffc9e5d43488d44797891e8340e5aaabe4208683fca8`,
parity Zig SHA256 is `4d421b4533a802308b39abee197f6aacdaabcb934db48bc87e186904381c1b1c`.
These are separate stage artifacts, not one combined runtime qualification.
Exact input/artifact/log hashes and exit/count receipts are in
`docs/qualification/2026-10-09-zig-task2-gate.json`.

Task 2 is complete for reviewed native/raw/outcome and explicit CLI candidate
integration. Actual browser storage, other-host filesystem execution, graphical
editor/apply/overwrite policy and power-loss durability remain open in the
complete-game acceptance. Save output I/O can fail after durable replacement;
that delivery failure does not imply old bytes were preserved. No commit,
push or PR was made. Task 3A clock/session is independently reviewed and gated;
Task 3B command controller is in progress.

## Observed Task 3A clock/session gate on 2026-10-09

After the test-only diagnostic print was removed, the fresh complete
`./zig/tools/check.sh` returned terminal exit0. Debug134 tests, ReleaseSafe
tests/build, nativeCLI57 checks, codec323 cases and144000 actual transitions
passed, with all28 candidate/protected identities unchanged. Independent
specification/quality review and the print-only scoped re-review approved the
candidate with no open findings. Exact stage/log/source hashes are in
`docs/qualification/2026-10-09-zig-task3a-gate.json`; CLI and parity remain
separate freshly fenced artifacts. Two interrupted runs have no accepted exit
receipt and are not counted as complete gates.

The pure session preserves source clock/speed/lifecycle behavior and owns step
reports. Allocation failure retains earlier completed ticks and the unchanged
failing tick, clears clock debt and enters an explicit sticky recovery hold.
Restart freshly admits the scenario before swapping. Original3A tests were
written after implementation; no behavioral test-first RED provenance exists.
That process gap is recorded rather than promoted to TDD evidence. New slices
require observed behavioral RED before GREEN. Actual controller/editor/host
composition, focus/camera/advisory resets and graphical3J remain open.

## Task3B verified scoped command controller

Current: independent spec/quality review approved, no findings. Full `./zig/tools/check.sh` terminal exit0 on frozen source:146Debug tests, ReleaseSafe test/build0,57nativeCLI process checks,323codec cases and144012trace rows/144000transitions/2038events. All30 source identities unchanged. Exact stage binary/input/log/source/review identities are in [Task3B gate receipt](2026-10-09-zig-task3b-gate.json). Stage binaries are separate freshly fenced artifacts; headless/controller effects do not close actual UI/onboarding/focus/browser/native-host/runtime acceptance. Behavioral RED→GREEN logs and three-mode13/13focused results are retained in the SDD workspace. Whole game/Task3/allLG rows remain Partial/Pending.

## Task3C verified detached editor

Current: fresh spec/quality review approved with no findings. Full `./zig/tools/check.sh` returned terminal exit0 on frozen source:162Debug tests, ReleaseSafe test/build0,57nativeCLI process checks,323codec comparisons and144012trace rows/144000actual transitions/2038events. All32source identities stayed unchanged. Exact stage/source/log/review receipts are in [Task3C gate receipt](2026-10-09-zig-task3c-gate.json), and the accepted full log is preserved in the SDD workspace. Focused Debug/ReleaseSafe/ReleaseFast each passed17/17 including16new editor tests; five actual behavioral RED stages,27allocation failure ordinals and100long-to-short retention cycles were reviewed. Intentional allocation-count test stderr causes a Zig build diagnostic; terminal exit0 and complete test results are recorded separately. Detached drafts, fields, Unicode/IME policy and modal capture ownership are covered. Store transactions, active apply, actual widgets/layout/host IME and all graphical/product acceptance remain Pending. Whole game is Partial; no commits, pushes or PRs.

## 2026-10-09 Classic3D transaction slice

Current scoped implementation/review: editor open/close, then-current load, captured save/overwrite, one-shot confirmations, apply installation and reset operations preserve authority and separate typed action/store diagnostics. Original independent review approved with a Minor native-facade reopen proof gap; FIX1 supplementary fresh NativeStore readback closes it and fresh scoped review approves with no remaining findings. Original artifacts remain preserved. No supplementary behavioral RED is claimed.

Current gate: `./zig/tools/check.sh` child exit0;185 Debug tests, ReleaseSafe test/build exit0,57 CLI process checks,323 codec records and144000 tick transitions/144012 matching rows/2038events. All34 source identities unchanged; logSHA `0cee3f932f1630eaec6b213c00dbd4ff1f6988615383e03fc0d702d31034a038`. Final focused3D Debug/Safe/Fast23/23 each, fmt0. Gate emits inherited allocation diagnostic output classified separately from its actual successful terminal result. See [gate receipt](2026-10-09-zig-task3d-gate.json) and [FIX1 review](2026-10-09-zig-task3d-fix1-review.md).

Partial: this closes3D headless transaction composition only. Actual3E consumers, GUI/host/browser, installed profiles, directory crash durability, manual/a11y/performance/offline and all full-game LG rows remain Pending.

## 2026-10-09 Classic3E session slice

Current scoped implementation: six-step intent guidance, immediate durable settings updates, typed control focus and source-specific guards compose AppCore and editor transactions without changing tick authority. Completion attempts persistence and editor opening independently; owned diagnostics survive allocation failure and inert editor dispatch. Independent spec/quality review approves with one deferred Minor allocation-count test print.

Current verification: frozen focused Debug/ReleaseSafe/ReleaseFast each28/28,20 deterministic session allocation ordinals,fmt0. Parent `./zig/tools/check.sh` terminal exit0;212Debug tests,ReleaseSafe test/build0,57CLI checks,323codec comparisons,144000tick transitions/144012rows/2038events;40source identities unchanged. Full logSHA58f070ca3ef1a4be4b1873e92f5aab3215d0c387bc98b71744a02b6f33807ee2 retained. Intentional stderr yields a Zig runner diagnostic; retained final focused mode results and aggregate terminal result are separate evidence. See [gate receipt](2026-10-09-zig-task3e-gate.json) and [independent review](2026-10-09-zig-task3e-review.md).

Partial: closes headless3E only. Actual advisory/camera integration, widgets/layout/input/IME, guide journey, accessibility, host/browser storage/runtime/performance and all full-game LG acceptance remain Pending. No commits, pushes, PRs or real profile changes.

## 2026-10-09 Classic3F1 advisory features/CPU

Current scoped: fixed12feature/7score/57weight CPU model and synchronous complete-fleet observation, no heap or authority mutation. Independent spec/quality Approved; optional distance fixture Minor deferred. Focused Debug/Safe/Fast9 each0/fmt0; literal historicalRustscorebits match local modes only. Parentgate0/221Debug/ReleaseSafe0/57CLI/323codec/144000ticks;44identitiesunchanged. [Gate receipt](2026-10-09-zig-task3f1-gate.json), [review](2026-10-09-zig-task3f1-review.md). Controller/per-step scheduling, actualGPU, historicalgoldenprovenance/cross-target/native/browser/manual/fullgame remain Partial/Pending.

## 2026-10-09 Classic3F2 advisory controller

Current scoped: fixedowned request/snapshot, full correlation, immediateCPU/latestqueue, fault-before-stale and stickyepoch fallback. FreshspecqualityApproved/optionaltestMinor deferred. Focused31three-mode(9controller/22imports) each0/fmt0. Parentgate0/230Debug/ReleaseSafe0/57CLI/323codec/144000ticks;46sourceidentitiesunchanged. [Gate receipt](2026-10-09-zig-task3f2-gate.json), [review](2026-10-09-zig-task3f2-review.md). Per-step AppCore composition, pendinghost dispatch/actualGPU/host/fullgame Pending.
