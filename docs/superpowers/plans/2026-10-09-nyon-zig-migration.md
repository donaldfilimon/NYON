# Complete NYON in Zig 0.18-dev

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Canceled on 2026-10-19:** Donald canceled further Zig rewrite work. Unfinished Zig tasks and continuation instructions in this document are superseded; retain its historical evidence and existing source. See [the current implementation decision](../../../REWRITE-DECISION-2026-10-19.md).
<!-- END 2026-10-19 rewrite decision -->

Specification: the complete requirement inventory in
`docs/qualification/2026-10-09-zig-migration.md` and its binding accepted
Workshop, Living V2, approved V3 and strategic-world references. Existing
source/tests remain comparison oracles during migration. The user explicitly
requested implementation and Subagent-Driven Development; this is the execution
ledger's task map, not a substitute for the complete-game scope.

## Global Constraints

- Work in canonical `~/dev/active/NYON` on its current `main`. Preserve every
  existing Living edit. No commits, pushes, PRs or new production dependencies
  without the user's corresponding authorization.
- Compiler is exactly `0.18.0-dev.120+9fe22a29b`. Use isolated build caches;
  never delete shared caches or claim a cache failure is a source failure.
- Classic retains seven worlds, seed `0x4947_5731_2026_0902`, base fleet speed
  23, 60 Hz ordering and digest `0x67D9_6E98_3D6C_9330`.
- Preserve scenario 65,536-byte and preferences 16 KiB caps, exact canonical
  formats/identities and detached invalid drafts. Failed operations preserve
  active authority and data. Legacy persistence is read-only fallback after an
  absent primary slot; a present or errored primary wins.
- Workshop pure authority must remain independent of platform/UI/storage;
  Living V2 and Workshop identities remain isolated. Full wire schemas,
  capacities, mechanics, tools, histories and artifacts retain accepted scope.
- Complete Living bounded execution rather than copying its unfinished
  synchronous suffix: every phase and shared runtime/replay/retirement path
  consumes the actual shared 1..1024 budget; cancelled/stale jobs cannot publish.
- Camera/settings/audio/backend/narrative/UI do not change canonical truth.
- Debug, ReleaseSafe, differential/oracle, independent vectors, build artifacts,
  native/browser/manual/accessibility/performance and unavailable-host evidence
  remain distinct. A scoped green check cannot establish full-game completion.

## Task 1: Port and review complete Classic authority and pure codecs

Own `zig/{build.zig,.zigversion,.gitignore,README.md}`, `zig/src/{root,main,
model,generator,economy,navigation,simulation,scenario,preferences}.zig`, their
test modules, `zig/tools/check.sh`, `zig/tools/check-classic-parity.py` and
`examples/zig_classic_oracle.rs`.

Implement the complete current Classic model/generator/tick path including
queued commands/preview, shared-snapshot AI, exact integer growth/movement,
simultaneous combat, hazards, outcomes, canonical state fingerprint and replay
identity. Port complete scenario validator/detached drafts/canonical codec and
preferences model/codec, including current serde input acceptance and strict
numeric behavior. Supply a terminal authority interaction and bounded trace
tool; graphical/runtime/storage work remains assigned below.

Test frozen genesis and independently specified source vectors, command and
event order, AI ties/defense, combat/absorption, hazards, malformed data/caps,
ownership and exhaustive allocation-failure preservation. Compare current Rust
and Zig canonical digests/reports over six seeds and autonomous/scripted
traces; compare canonical codec bytes, scenario identities and rejection
classes. Fence source/artifact identity and identify proof limits. Run Debug
and ReleaseSafe tests and fresh build. Review specification and quality before
marking this task complete. Existing implementation at skill start is a
candidate awaiting this review, not grounds to skip it.

## Task 2: Port Classic persistence and store outcomes

Own new `zig/src/classic_store.zig` and `classic_store_tests.zig`, then wire them
through `root.zig` and the CLI via a separate integration dispatch.

Read `src/scenario/store.rs` and `src/preferences/store.rs` for exact target
paths, error categories, atomic write semantics and absent-primary-only legacy
behavior. Port platform-neutral request/outcome/fallback logic and actual
native filesystem adapter with injected profile paths for qualification.
Retain complete record validation before installation, exact failure messages
or typed equivalents, invalid/current-primary precedence, read-only legacy
fallback and no copy-on-read. Explicitly preserve separate scenario/preference
namespaces and active state after failures. Do not overwrite Donald's real
profile during tests; use owned disposable test directories.

Test positive encode/save/load, absent/present/invalid/error primary cases,
legacy immutability, size bounds, atomic replacement/failure paths and durable
readback. Native adapter evidence does not close browser storage. Native and
browser adapters must eventually be present in the final runtime (Task 6).

## Task 3: Port Classic application/editor/presentation experience

Port the current Classic app core, fixed clock, onboarding/settings, input
router/durable exit, detached editor, presentation/UI model, launch
confirmation and guide. Compose the Task 1 authority/codec and Task 2 store
interfaces without letting editor/advisory mutate authority. Use bounded
reviewable implementation slices derived from the current module boundaries.
Qualify positive/cancel/invalid-load/apply/recovery journeys and state/settings
isolation. The final graphical host is supplied by Task 6; terminal interaction
alone does not complete this task.


Use the source-cited `docs/qualification/2026-10-09-zig-classic-app-inventory.md`
as the concrete decomposition and acceptance inventory. Preserve these ordered
reviewable slices rather than dispatching the complete shared host at once:

| Slice | Owned responsibility | Interface prerequisite |
| --- | --- | --- |
| 3A | Fixed rational clock and allocator-owned Classic session skeleton | Task1 checked scenario/simulation |
| 3B | ID command controller and immutable preview/tray | 3A tick/report/sequence policy |
| 3C | Detached editor fields, invalid text and IME | Task1 owned drafts and admission |
| 3D | Confirmed load/save/apply/revert/default/regenerate transactions | Reviewed Task2 stores, 3A and3C |
| 3E | Preferences/onboarding/settings/action-stable focus | Task2 outcomes,3A/3B/3D |
| 3F | Exact CPU advisory and GPU completion controller policy | Focused source inventory; no placeholder scores |
| 3G | Camera/math, immutable scene and logical picking | 3B/3E/3F; reviewed math interface |
| 3H | Shared retained UI/editor layout and offline guide | 3C/3E/3G |
| 3I | Normalized input/interaction/router, guide hold and pure durable-exit | 3A..3H; host readiness remainsTask6 |
| 3J | Full Classic vertical journeys through real graphical host | Task6 actual native/browser delivery |

The focused advisory contract is now source-cited in
`docs/qualification/2026-10-09-zig-classic-advisory-inventory.md`. Slice3F contains
fixed12→4→1 CPU features/model, fixed-value request/controller, AppCore projection
and laterTask6 adapter interoperability. Execute independently reviewed3F1 immutable features/model →3F2 fixed-value
controller →3F3 per-step session/projection integration. Each has its own
source freeze, full scoped gate and fresh review before its dependent slice. Preserve exact f32 score-bit witnesses,
all-fleet features, cadence30, one flight/one latest queued request, full metadata
comparison and matching-fault-before-latest-stale fallback order. CPU publication
is synchronous. GPU adapter readiness and device initialization generation are
separate from scoring acceptance; a skipped GPU test proves no GPU parity.

Binding source distinctions: selected speed survives Settings/Editor/guide holds;
clock caps at250ms and15steps per speed pass (0/1/2/4multipliers); direct gameplay
queueing may accept a command whose immutable preview rejects. Touch target
selection alone never launches. Saving/loading editor draft never redefines the
original or active scenario; confirmed apply builds a replacement before swap.
Preferences save failures retain live changed settings. Editable numeric syntax
is separate from strict canonical wire syntax. Classic currently has no battle
autosave; shared durable session close jobs remainTask6. Existing sparse Classic
semantics do not close the required full accessibility acceptance.

Review each slice before its dependent integration. Complete headless3A..3I,
then the independent Workshop/Living prerequisites andTask6 host, then3J. Keep
Task3 Partial until3J qualifies the complete player experience; its later host
dependency does not justify dropping graphical/manual requirements.

## Task 4: Port complete Workshop authority/history/archive and Forge

Port the pure island under `crates/nyon-workshop-core/src/` excluding Living,
including typed IDs, strict pack/schema/validation, all creator operations,
six-phase 10 Hz mechanics, atomic checked batches/faults, revision DAG/undo/redo,
branches, canonical archives and replay. Derive subtask ownership from the
source dependency order and review each interface before dependent execution.
Preserve every accepted capacity and wire contract. Complete all 18 Forge
steps to real alloy production, full history/archive parity, independent
identities and malformed/overflow/failure cases.


Concrete decomposition and source witnesses are in
`docs/qualification/2026-10-09-zig-workshop-core-inventory.md`. Review and execute
4A IDs/state/canonical bytes →4B strict pack/catalog →4C creator wire/revision
records →4D all14creator operations →4E six-phase simulation →4F history/replay/
branches →4G authenticated archive/bounded restore →4H complete coreForge.
Keep command wire declarations independent of command execution to break the
source declaration cycle without changing the serialized revision schema.

Freeze independently derived Workshop fixtures before using Zig output as an
oracle. Existing Rust literals are external comparison witnesses; their
independent derivation provenance is currently unknown. Preserve separate
spec-derived vector and Rust differential evidence. Extend the Forge storm step
with explicit active/expired floor-halved logistics assertions. Existing core
save/Continue merely reconstructs an archive: actual process quit, durable
selected branch Continue and portable handoff remain required inTask6/8.

## Task 5: Port Living V2 and complete its bounded execution

Port the full isolated wire/catalog/model/genesis/rules/commands/receipts/
history/archive/notebook/authority island and all ten phase mechanics. Preserve
all independent vector bytes and capacity limits. Complete all validation,
hashing, journal, facts, decision, terminal Ready, exactly-once extraction and
publication/retirement paths under a shared actual budget. Port replay/fork/
restore/compare through the same executor. Derive smaller explicit tasks from
the accepted bounded-executor plans; review before each dependent slice.
Test cancellation/stale/failure/cut points, maximum capacities, all-budget
equivalence, authenticated notebook/history, runtime allowance and owner
cleanup. Do not label the inherited synchronous suffix complete.

Concrete current-source contracts and proposed slice ownership are recorded in
`docs/qualification/2026-10-09-zig-living-inventory.md`. Execute reviewed
5A corpus/source freeze → 5B strict declarations/wire/IDs/catalog → 5C state/
Genesis/copy/hash/retirement → 5D creator wire/admission → 5E all 46 creator
cursors/cascades → 5F scheduled phases 2..7 → 5G intents/reservations/dispatch →
5H all-three-receipt sealing/journals/real Ready/commit → 5I the same executor
for replay/history/fork/restore/compare → 5J authenticated notebook/content →
5K shared runtime/publication/retirement handoff → 5L whole-island qualification.
Split the large creator and scheduled-phase slices at their reviewed checkpoints.

Do not repeat the older ledger's facts-sealing status as current: current source
has an incremental facts child, while decision/recovery/corridor bindings,
source-sized journal assembly and resident adoption still contain synchronous
work. The complete path must expose real terminal Ready and borrowed,
noncomputing extraction, with preallocated publication ownership. Archive and
notebook constructors, finalization, comparison output and retirement share the
same actual budget. Preserve accepted queued tails and durable allocator marks;
no stale preparation may publish a row or cue. The original independent-formula
base corpus currently verifies unchanged, but full nested/additive byte coverage
and Zig parity remain separate required gates. The creator addendum is Draft;
current rules/schema/tests retain all 46 supported tools without promoting its
proposed decisions to normative status.

## Task 6: Port full platform runtime, UI, storage and production content

Port native and browser hosts, graphics backends/fallback/quality, shaders and
atlas use, layouts/input/picking/camera/navigation, semantics/accessibility,
audio/preferences, Workshop and Living Library/Sessions/coordinators/native and
real IndexedDB stores/transfer/recovery/Continue and all production templates,
creator tools and seven experiments. Preserve exact catalog/versioned-domain,
CAS/durable-job/focus/portable-transfer contracts. Qualify complete vertical
journeys, all 46 creator tools, native/WebGPU/WebGL2, failure/recovery/invalid
and cancelled flows, visual/layout/Retina/touch/pinch/keyboard/screen reader,
offline cold startup and authentic content. Resolve platform bindings from a
reviewable design before adding production dependencies. Current source/SDK
evidence and proposed alternatives are in
`docs/qualification/2026-10-09-zig-platform-binding-inventory.md`; local
installation does not authorize new production libraries or prove any host.

## Task 7: Complete approved V3 and strategic cubic world experience

Complete the current independent V3 corpus, integrated review and immutable
package freeze, then implement separate V3 authority/identities/archives/
storage/direct Genesis/authenticated post-root history and complete society/
research/ecology/diplomacy/conflict mechanics and player experience. Preserve
the accepted initialization decisions. Preserve strategic architecture approval
and written-spec/plan review gates before implementing six-faced planets/searchable galaxy/system/planet views,
Continue/Templates/Demos/presets/guided demonstrations/persistent settings at
accepted capacities. Prepare concrete reviewable design artifacts for those
gates; a later explicit user decision may supersede the recorded approval
sequence. Prior V3 rules approval is recorded in the current source
ledger and does not substitute for corpus/review/runtime evidence.

## Task 8: Qualify complete Zig game and retire Rust production paths

Audit every inventory row and LG-01..LG-27 against actual final Zig artifacts.
Run required platform/backend/live/manual/input/accessibility/storage/portable/
offline/capacity/performance/endurance/memory acceptance and independent review.
Retain unavailable host evidence as unverified. Retire residual Rust production
paths only after all replacements are exercised; retain necessary reference
fixtures/tooling separately. Update player/developer documentation to match
actual execution. Goal completion requires every accepted requirement proved,
not just the first port or a successful build.
