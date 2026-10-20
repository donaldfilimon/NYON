# Finish NYON in Zig

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Canceled on 2026-10-19:** Donald canceled further Zig rewrite work. Unfinished Zig tasks and continuation instructions in this document are superseded; retain its historical evidence and existing source. See [the current implementation decision](../../../REWRITE-DECISION-2026-10-19.md).
<!-- END 2026-10-19 rewrite decision -->

## Summary

Complete the approved full-game rewrite using Zig `0.18.0-dev.146+35accc06e`. Deliver macOS first, then qualify Windows, Linux, WebGPU, and WebGL2. Completion requires playable journeys, durable saves, deterministic gameplay, and platform acceptance—not just passing library tests.

Preserve the current uncommitted work. Use the existing expanded roadmap and coordinated-galaxy specification as authority. Review and approve each remaining mechanics design before implementing it.

## Implementation sequence

1. **Finish coordinated galaxy.** Inspect and review the in-progress planet/state changes, then complete the existing ten-task plan: shared command admission, leased snapshots, canonical histories, atomic global ticks, bounded rollback, cleanup fault handling, replay, robustness, CLI integration, and qualification. Never simulate global atomicity by publishing planets sequentially.

2. **Complete strategic and city gameplay.** Prepare separate designs for seeded galaxy generation, civilizations, richer city production/workforce/needs, infrastructure and interplanetary logistics, research, ecology, diplomacy, and fleets. For each subsystem, obtain written approval, produce a detailed implementation plan, implement, review, and qualify before dependent work begins. Preserve creator, ruler, and combined roles.

3. **Implement tactical ground combat.** Review a design covering units, terrain, movement across cube seams, targeting, simultaneous resolution, objectives, retreat, occupation, and autonomous behavior. Player and autonomous actions must use the same deterministic authority rules.

4. **Implement durable sessions.** Review the new archive and session contracts, then implement atomic saves, generation checks, branches, undo/redo, replay, comparison, import/export, and exact selected-branch Continue. Use new versioned identities and storage namespaces; never read, migrate, or overwrite existing user profiles.

5. **Deliver the macOS vertical slice.** Implement the shared runtime and AppKit/Metal host, retained UI, galaxy/system/planet navigation, inspectors, city placement, tactical controls, text input, audio, and accessibility. Complete a real journey from new galaxy through production, diplomacy, combat, save, process exit, and Continue.

6. **Complete remaining hosts and owned tooling.** Implement Windows Win32/DX12, Linux X11/XWayland/Vulkan, and browser Wasm with WebGPU/WebGL2 and real IndexedDB. Complete project-owned Zig shader, atlas, build, validation, and packaging tools. Keep foreign-language adapters limited to approved platform responsibilities.

7. **Complete content and delivery qualification.** Add templates, tutorials, demos, difficulty presets, searchable help, and campaigns. Qualify the complete game, then retire obsolete Rust production/tooling paths only after replacement evidence and explicit retirement authorization.

## Interfaces and invariants

- Pure authority owns gameplay; platform, UI, storage, audio, and advisory code consume copied projections and submit typed commands.
- One fixed authoritative clock and canonical command ordering govern the galaxy. Simulation, hashing, history, publication, restoration, and cleanup share actual bounded work allowances.
- Use the coordinated-galaxy plan’s existing command, receipt, progress, snapshot-lease, and failure contracts. Freeze later gameplay and save interfaces through their approved designs.
- Prepare and validate changes before publication. Allocation failure, cancellation, stale completion, storage conflict, and malformed input preserve the active state.
- Run one implementation agent at a time for shared authority work, followed by independent review. Parallelize investigation or isolated work only with explicit ownership.

## Tests and acceptance

- Run the scoped Zig gate after each implementation task; extend it as new systems become real. Reproduce and fix attributable failures before advancing.
- Verify independent wire/hash fixtures, replay parity across budgets and build modes, global atomicity, snapshot leases, allocation failures, cancellation boundaries, rollback, and cleanup faults.
- Exercise durable save conflicts, interrupted writes, abandoned mutations, corrupt archives, failed imports, and quit/reopen Continue. Confirm old profiles remain unchanged.
- Qualify actual native and browser journeys, device loss, offline operation, keyboard/touch/IME input, screen readers, high-DPI layouts, and graphics fallback.
- Measure the approved capacity and performance fixtures on named hardware, including authority p95 ≤5 ms, native/WebGPU frame p95 ≤16.7 ms, WebGL2 Low ≤33.3 ms, and input p95 ≤100 ms.
- Audit the final build, generation, verification, runtime, and package graph with Rust unavailable. Mark missing hardware evidence unverified; never substitute compilation for runtime acceptance.

## Assumptions and approval boundaries

- The full approved game scope remains required; macOS-first sequencing does not remove other delivery targets.
- Each remaining city, combat, and save design receives Donald’s written review before implementation.
- Preserve existing changes and instruction files. No commits, pushes, publication, deployment changes, or new production dependencies without authorization.
- Update qualification records with verified results and explicit gaps. Declare completion only when all required gameplay, tooling, host, persistence, and acceptance rows are satisfied.
