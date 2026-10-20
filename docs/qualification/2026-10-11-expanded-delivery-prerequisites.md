# Expanded rewrite delivery prerequisites

Date: 2026-10-11. Status: **Partial; full delivery remains unverified**.
Read-only source investigation followed by this isolated evidence report. No new
runtime, test, hardware, profile, dependency, asset, CI or retirement evidence is
claimed. The active coordinated-galaxy implementation and its ledger were not
inspected or changed. Existing audits remain unchanged.

## Scope and authority

The [expanded roadmap](../superpowers/plans/2026-10-11-expanded-zig-rewrite.md)
records the approved full Zig game: creator/ruler/combined roles, city development,
tactical ground combat, one deterministic clock, macOS first and final Windows,
Linux, WebGPU/WebGL2 delivery (lines 9–21). No Rust may remain in the delivered
game, libraries, generators, shader tools, build or required verification.
Existing user profiles must not be read, converted, overwritten or migrated.

Overall execution approval does not approve invented subsystem mechanics.
Roadmap lines 32–43 require reviewed written specifications, written approval,
detailed plans and execution selection. New durable-save and gameplay rules
remain subject to that workflow. Preserve all existing source and instructions;
no commit, push, deployment, new production dependency or retirement is implied.

## Current source routing and discrepancies

| Status | Source evidence | Delivery implication |
| --- | --- | --- |
| Current | [compiler pin](../../zig/.zigversion), line 1; [build](../../zig/build.zig), lines 4–7 | Exact pin is `0.18.0-dev.146+35accc06e`. |
| Current / Partial | build lines 15–29 installs/runs the CLI; [entrypoint](../../zig/src/main.zig), lines 12–35 exposes authority demos and explicit Classic store probes | No graphical host build target is established. |
| Current / Partial | [Classic store](../../zig/src/classic_store.zig), lines 10–18, 122–129, 453–465, 524–578 | Scenario/preferences records have legacy fallback and temp-file sync/rename. They are reference primitives, not expanded durable sessions. Copying fallback/profile discovery violates the clean-profile constraint. |
| Current / Partial | [Wasm probe](../../zig/src/planet_economy_wasm.zig), line 1; build lines 49–54 | Freestanding object compilation is not browser runtime or IndexedDB acceptance. |
| Current / Partial | [Zig gate](../../zig/tools/check.sh), lines 17–39 | Rust PATH blockers and scoped tests/CLI/Wasm checks do not qualify the whole game. |
| Current / Partial | [CI](../../.github/workflows/ci.yml), lines 41–51, 178–183 | Required repository pipeline still runs Cargo. |
| Approved design / Proposed interfaces | [platform specification](../superpowers/specs/2026-10-09-nyon-zig-platform-bindings-design.md), lines 7–13, 29 | Written specification approval permits detailed planning; implementation plan/review and acquisition/asset/retirement gates remain separate. |
| Discrepancy | platform specification lines 33–40, 191 | Prerequisites route older Classic/Workshop/Living/V3 contracts and line 191 names older Zig `dev.120`. Reconcile against the expanded clean-break design and actual pin before dispatch; do not silently apply historical profile fallback. |
| Approved qualification contract | [coordinated-galaxy specification](../superpowers/specs/2026-10-11-nyon-zig-coordinated-galaxy-design.md), lines 68–80 | Four systems/planets/civilizations are qualification-sized; the spec explicitly says four worlds do not satisfy large-galaxy delivery. This is contract evidence, not a claim about active implementation. |
| Historical acceptance target | platform specification lines 567, 582; [Living inventory](2026-10-09-zig-living-inventory.md), line 32 | Earlier acceptance includes 64 systems/512 worlds and wider entity caps. These numbers do not prove expanded capacity or authorize new mechanics. Final expanded scale needs explicit reconciliation, reviewed limits and fresh loaded-memory/work evidence. |

The [expanded acceptance audit](2026-10-11-expanded-rewrite-acceptance.md),
lines 18–28, retains durable-session, host, accessibility and qualification gaps.
Its dated coordinated-galaxy status must not substitute for the active owner's
current evidence.

## Ordered safe sequencing

1. **Ready for independent preparation:** reconcile the approved platform design
   with the expanded authority/session boundary, actual compiler pin and clean
   profile rule. Produce a detailed macOS/tooling plan and independent review.
   Read-only SDK and dependency inventories can proceed without touching active
   authority work; installed headers are not ABI/runtime proof.
2. **Ready for specification; implementation pending reviewed contracts:** design
   the new storage namespace, archive identity/wire limits, atomic save and CAS,
   selected branch, history/replay/comparison, import/export and abandoned-write
   reconciliation. Use synthetic fixtures exclusively. Roadmap lines 53–58 and
   95–96 require reviewed deterministic authority and durable-outcome interfaces;
   no save wire should assume unimplemented mechanics. Failure must preserve the
   resident session and old data.
3. **Ready for independent tooling planning:** owned Zig shader parser/checker/
   lowering, renderer ABI fixtures and atlas generator contracts. Platform spec
   lines 88, 191 and 511–512 make these prerequisites of Metal graphics and
   distinguish immutable qualification candidates from final release assets.
   Preserve committed assets; changed pixels/metrics need reviewed versioning.
4. **Host implementation pending plan/ABI/tool gates:** AppKit/Metal bootstrap,
   bounded event/completion ownership, input/text/IME, retained UI/semantics,
   device loss, audio and filesystem adapters. Platform spec line 512 lists the
   macOS prerequisites. Isolated host conformance work can precede complete
   gameplay after dispatch approval; callbacks must enqueue validated events,
   not mutate authority. Integrated gameplay consumes reviewed session/authority
   contracts. Required macOS journey is galaxy → settlement → production →
   diplomacy → conflict → save → process quit → exact Continue (roadmap 60–65).
5. **Proposed, separate targets:** Windows Win32/DX12, Linux X11/XWayland/Vulkan,
   and browser Wasm with owned glue, both WebGPU/WebGL2, real IndexedDB, recovery
   and offline behavior. Prepare each dependency/SDK and failure-test plan in
   parallel; no macOS result substitutes for another platform.
6. **Final delivery pending replacement qualification:** extend Rust-blocked
   required gates to actual hosts, shaders/generators/assets, independent replay,
   malformed input, cancellation/stale completion and maximum-loaded capacity.
   Replace CI and retire Rust only when equivalent retained capability and
   dependency closure are proved. The [retirement inventory](2026-10-11-zig-rust-retirement-inventory.md),
   lines 8–19, routes exact replacement paths; deletion is not qualification.

## Hardware and manual evidence blockers

**Unverified:** actual AppKit/Metal ABI/device execution; Windows SDK/COM/DX12
and Linux distribution ABI/driver behavior; browser backend/device-loss and
IndexedDB transaction/offline recovery; IME, screen readers, touch, Retina;
maximum-loaded timing, memory, endurance and process overhead. These require
the corresponding running host, named hardware/OS/browser and real interaction,
not just code or cross-compilation. Platform spec lines 27–29, 512, 514, 517–518,
and 582 define the distinctions. The currently available local machine is not
evidence for unavailable Windows/Linux hardware. Record absent hardware proof
as Unverified while continuing independent code/specification work.

## Checks for this report

The report target was checked absent before creation. Local Markdown targets
were checked for existence and `git diff --check --` was run for this file.
No build or test was run; these checks establish report hygiene only.
