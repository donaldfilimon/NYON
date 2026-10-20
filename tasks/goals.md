## Ship verified repository guidance

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Canceled on 2026-10-19:** Donald canceled further Zig rewrite work. Unfinished Zig tasks and continuation instructions in this document are superseded; retain its historical evidence and existing source. See [the current implementation decision](../REWRITE-DECISION-2026-10-19.md).
<!-- END 2026-10-19 rewrite decision -->

status: in_progress
- AGENTS.md enhanced with Living V2 guard, storage, atlas, and slow-test warnings
- Doc comments added to 5 pub items in src/game/model.rs (WORLD_COUNT, DEFAULT_SEED, RulesV1, World, Fleet)
- All gates passed: native + wasm fmt/clippy/test/build-web/check-workshop
- Plan agent assessments delivered (research only; 5 improvements, dependency constraints, risk notes, 3-month slice)

## Finish NYON in Zig (historical; canceled 2026-10-19)

status: canceled
updated: 2026-10-11
compiler: 0.18.0-dev.146+35accc06e

The complete user-supplied goal, implementation sequence, invariants, tests and
acceptance boundaries are preserved in
[Finish NYON in Zig](../docs/superpowers/plans/2026-10-11-finish-nyon-in-zig.md).
The session tracked "continue and complete NYON game in zig 0.18" until Donald
cleared goal tracking at 08:40 EDT. His subsequent "continue" retains repository
implementation work; no new session goal was created. Library tests and
compilation alone never complete the repository goal.

- [ ] Finish the ten-task coordinated galaxy plan: shared admission, leased
  snapshots, canonical histories, atomic global ticks, bounded rollback,
  cleanup faults, replay, robustness, CLI and qualification. Publish with one
  descriptor switch; never sequentially publish planets as a global tick.
- [ ] Complete reviewed strategic/city gameplay: seeded galaxy generation,
  civilizations, production/workforce/needs, infrastructure, interplanetary
  logistics, research, ecology, diplomacy and fleets; creator/ruler/combined.
- [ ] Complete deterministic tactical ground combat: units, terrain, cube seams,
  targeting, simultaneous resolution, objectives, retreat, occupation and
  autonomous actions under the same authority rules.
- [ ] Complete new-version durable sessions: atomic saves, generation conflicts,
  branches, undo/redo, replay, comparison, import/export, exact selected-branch
  Continue and abandoned/interrupted/corrupt-operation recovery. Never access,
  migrate or overwrite existing profiles.
- [ ] Qualify the macOS AppKit/Metal game journey from new galaxy through
  production, diplomacy, combat, save, process exit and exact Continue; retained
  UI, navigation, inspectors, city/tactical controls, text, audio/accessibility.
- [ ] Qualify Windows Win32/DX12, Linux X11/XWayland/Vulkan, browser Wasm with
  WebGPU/WebGL2, real IndexedDB, offline operation and graphics fallback.
- [ ] Replace owned shader, atlas, build, validation and packaging tooling in
  Zig; keep foreign adapters within reviewed platform responsibilities.
- [ ] Complete templates, tutorials, demos, difficulty, searchable help and
  campaigns; independent gameplay, lifecycle, capacity and delivery evidence.
- [ ] Meet named-hardware targets: authority p95 <=5 ms, native/WebGPU frame
  p95 <=16.7 ms, WebGL2 Low <=33.3 ms, input p95 <=100 ms. Missing hardware
  evidence stays Unverified.
- [ ] Verify the full build/generation/verification/runtime/package graph with
  Rust unavailable, then remove all obsolete Rust game/library/tooling/source
  and configuration paths once equivalent Zig replacements qualify.

Preserve uncommitted work and instructions; one writer owns shared authority
files, with fresh independent review after each task. Parallel investigation or
isolated implementation requires explicit ownership. No commits, pushes,
publication, deployment changes or new production dependencies are implied.
The [delegated design/retirement authorization](../docs/qualification/2026-10-11-delegated-game-design-authority.md)
records Donald's 08:19–08:20 delegation and 08:23 explicit Rust removal direction.
Concrete specs/plans and independent reviews remain required; approvals under
delegation are recorded as controller decisions, not personal artifact review.

Current scoped evidence (2026-10-11): the five-task isolated seeded-manifest
slice passed fresh independent reviews. Aggregate Debug/ReleaseSafe each pass
48/48; CLI suites each pass 10/10. Shared integration remains Partial: the full
Zig gate stopped at a source-stability guard after both 628-test suites passed,
and exact coordinated-v0 genesis/head equivalence remains pending. See the
[qualification record](../docs/qualification/2026-10-11-seeded-galaxy-qualification.md).

The other coordinated-galaxy owner was last verified reviewing Task6 after
reviewed admission, leased snapshots and canonical histories. Full-scale authority design and plan
passed scoped reviews for planning; joint session publication/provider contracts
and actual compiler layouts still gate implementation. Rich-city composition
and owned Zig atlas tooling remain in reviewed-design preparation. No full-game
acceptance row is complete merely because these slices pass.


## Rust stable upgrade and feature recovery

status: in_progress
updated: 2026-10-19
compiler: Rust stable 1.99.1, edition 2024

This section supersedes unfinished Zig instructions above. Historical outcomes are not current Rust acceptance evidence.

- [x] Compile retained workspace, both wasm backends and separate Rust tools/fuzz workspace on 1.99.1 (no test execution).
- [x] Reconcile the compiler decision and explicitly cancel the Zig continuation goal.
- [ ] Inventory every retained Zig implementation, Rust destination, fixtures and acceptance gaps.
- [ ] Port cube topology, roles and navigation; planet economy; coordinated galaxy/history/replay; seeded manifests; expanded-session metadata; app/renderer/store/tool integration, in dependency order.
- [ ] Review and improve separate Rust designs/plans for parity, full-scale authority/durable sessions, rich cities/strategy, tactical combat and delivery/tooling/content.
- [ ] Runtime and platform acceptance: deferred while tests and CI gates are paused.

Preserve dirty work and all Zig source. Compilation qualifies source compatibility only. No commits, pushes, new production dependencies or deployment changes.
