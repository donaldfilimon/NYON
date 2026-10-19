# NYON expanded Zig rewrite roadmap

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Canceled on 2026-10-19:** Donald canceled further Zig rewrite work. Unfinished Zig tasks and continuation instructions in this document are superseded; retain its historical evidence and existing source. See [the current implementation decision](../../../REWRITE-DECISION-2026-10-19.md).
<!-- END 2026-10-19 rewrite decision -->

Status: user-approved overall direction and execution request, 2026-10-11.
This records the approved chat plan; it is not a replacement for each new
subsystem's reviewed specification and detailed implementation plan.

## Global constraints

- Zig exactly `0.18.0-dev.146+35accc06e`.
- No Rust in the final game, libraries, generators, shader tooling, build or
  required verification pipeline. Read existing Rust only as reference during
  migration; retire it after the replacement is qualified.
- Clean compatibility break, new versioned identities and saves. Do not read,
  convert, overwrite or migrate existing user profiles in the new game.
- Creator, ruler and combined roles; strategic cube planets, detailed city
  development and tactical ground combat are required features.
- Single fixed authoritative clock and deterministic command ordering. Pure
  authority has no graphics, windowing, browser, audio or storage dependencies.
- macOS first; Windows, Linux, WebGPU and WebGL2 remain final delivery targets.
- Explicit allocator ownership, bounded queues and shared-budget incremental
  work; failure/cancellation/stale completion must preserve active authority.
- Preserve existing work and instruction files. No commits, pushes, publication,
  secret/deploy changes or new production dependencies without authorization.

## Task 1: establish baseline

Inspect ownership and actual source; preserve existing work. Reuse reviewed
Zig modules and establish the exact compiler and Rust-free required gate.
Existing topology/role modules and CLI are Partial foundations. Validate their
quality before building dependent systems; retain test evidence and proof gaps.

## Task 2: freeze contracts by subsystem

Prepare and review specifications for new mechanics and interfaces before
dependent implementation. Next subsystem: seeded planet, settlement footprints,
production/workforce/supply and local freight with typed commands/receipts.
Then galaxy/civilizations/research/ecology/diplomacy, tactical combat, durable
history/sessions, platform/runtime and content. Each specification must resolve
its rules, numeric limits, phase order, failure semantics and independent tests.

The requested brainstorming workflow requires written-spec approval, then a
written detailed plan and execution-method selection. Overall feature approval
does not imply approval of newly invented city/combat/save mechanics.

## Task 3: implement pure authority

Implement seeded galaxy generation, civilizations, economy/logistics,
research/ecology/diplomacy, fleets, six-face settlements, creator operations,
city production and tactical combat. Validate before mutation. Make canonical
replay deterministic and share actual work allowances across simulation,
hashing, histories, restoration, publication and retirement.

## Task 4: durable sessions

New storage namespace; atomic saves and generation checks, branches,
undo/redo/replay/comparison, portable import/export and exact selected-branch
Continue. Reconcile abandoned mutations; failed operations preserve resident
state and old data.

## Task 5: macOS vertical slice

AppKit/Metal host, retained UI, input/text editing, audio, accessibility,
hierarchical navigation, inspectors, city placement and tactical controls.
Qualify new galaxy through settlement, production, diplomacy, conflict, save,
process quit and Continue before treating the macOS game as delivered.

## Task 6: remaining hosts

Windows Win32/DX12, Linux X11/XWayland/Vulkan, browser Wasm WebGPU/WebGL2 and
real IndexedDB transactions/recovery. Project-owned Zig shader/atlas tools;
small platform adapters may use system frameworks and browser JavaScript.
New production dependencies require approval; unavailable hardware proof stays
unverified.

## Task 7: content, qualification and retirement

Templates/tutorials/demos/difficulty presets/searchable help/campaigns. Replace
Rust generators and tools and retire obsolete Rust production sources only
after qualified replacements. Preserve old user data and instruction files.
Complete independent review and native/browser/manual/accessibility/offline/
performance/capacity/endurance qualification. Compilation is not game delivery.

## Execution method and dependency review

User selected subagent-driven development and parallel dispatch on 2026-10-11.
Use one implementation agent at a time for shared authority work, a fresh
task reviewer after each task, and independent parallel investigation/design
where no product state is shared. Inline executing-plans is the fallback for
controller-only integration without a suitable independent task.

| Producer → consumer | Shared contract | Current disposition |
| --- | --- | --- |
| 1 → 2/3 | Surface IDs/seam adjacency, player-role admission | Existing code; fresh review pending |
| 2 → 3 | Mechanics, capacities, schemas, phases, budgets | New subsystem written-spec review required |
| 3 → 4 | Deterministic state/commands/receipts | Not implemented; no save wire may assume it exists |
| 3/4 → 5 | Session projection, commands and durable outcomes | Not implemented; no host may mutate authority directly |
| 5 → 6 | Shared UI/runtime adapter boundary | Specification and implementation pending |
| 1..6 → 7 | Rust-free delivery and evidence inventory | Final acceptance pending |

Every task agrees with its own scope; no task permits dropping cities,
tactical battles, any final host, or old-data preservation. Later task details
are not yet sufficient for implementation dispatch.
