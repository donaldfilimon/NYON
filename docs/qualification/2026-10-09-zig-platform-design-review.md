## Design Document Review: NYON Task 6 platform bindings and complete game delivery

### Summary

**CHANGES_REQUIRED.** Architecture/specification compliance needs a correction to the accepted scheduler contract; the recommended Zig policy/direct native API/bounded browser glue architecture otherwise retains the complete product and host scope. Implementation readiness also needs the shader gate and dependency-preflight ordering corrected before dispatch: **0 Critical, 3 Important, 2 Minor** issues below.

Reviewed the complete 360-line draft, preparation brief, binding inventory, migration requirements and acceptance ledger. This is a production-read-only design review, not approval of implementation, dependencies, tooling acquisition, strategic architecture, or runtime acceptance. Design locations below refer to the reviewed draft; source locations were inspected in the current shared checkout and may subsequently move.

### Issue 1: Live ticket capacity does not enforce the accepted per-update publication limit

- **Severity**: major
- **Review class**: Important — blocking
- **Section**: Threads, scheduling, and bounded retirement; work-scheduler row, line 98; slice 6B, line 278.
- **Description**: “At most 20 live request/publication tickets shared with Living” replaces two distinct accepted invariants with a concurrency limit. Releasing and recycling a ticket allows more than twenty publications in one update while never exceeding twenty live tickets. Conversely, consuming all twenty slots with unrelated host I/O can change Living request admission. Saying each core poll consumes the shared allowance also leaves its reset boundary and the obligation to charge every child’s actual remaining allowance implicit. This is a source-contract mismatch, not a missing runtime measurement.
- **Evidence**: `docs/superpowers/plans/2026-10-02-living-resident-runtime.md:15` explicitly requires one shared resident update allowance, at most twenty published boundaries per update, and pending requests capped at twenty. Its lines 33–47 require retirement to survive no resident, drain-before-admission ownership, private cache/cue preparation, and exact pace/debt/Step behavior. Current `src/sandbox_session.rs:3619–3650` separately caps retained due requests and supplies the update poll allowance, but still has the transitional synchronous suffix at lines 3537–3568. That source is evidence of the migration seam, not proof that the complete budget contract already works.
- **Suggestion**: Name three separate counters: the shared update allowance (default 1024, exercised at 1/64/1024/mixed), Living pending-boundary count capped at twenty, and publications committed in this update capped at twenty. Keep any proposed global physical-ticket bound independent and label it Proposed. All history/restore/compare/boundary/cache/retirement children consume the same remaining allowance, including failed work; no independent fresh budget in a callback or store completion. Give the runtime a retirement owner that survives session removal. Preserve integer pace 1/4/20, dropped excess prospective debt, exactly-one Step, and accepted-command retention. Add a test with immediately completing/recycled tickets that would otherwise publish twenty-one boundaries, plus mixed history/retirement work and hidden/no-session cleanup.
- **Status**: open

### Issue 2: The shader prerequisite requires an unsupported compute target

- **Severity**: major
- **Review class**: Important — blocking
- **Section**: Graphics, shaders, and advisory, lines 150–160; slice 6C, line 279.
- **Description**: The pipeline sends “the seven existing WGSL shader families” through all listed target languages, and 6C requires “all seven WGSL families validated, target translations compiled.” However, the selected WebGL2 path is GLSL ES 3.00 with “no compute advisory”; forced native GL likewise does not require compute. Without a shader-entrypoint/backend eligibility table, this prerequisite either demands an impossible WebGL2 advisory compute build or invites an undocumented skip. This also blocks the downstream 6D–F prerequisites as written.
- **Evidence**: `assets/shaders/advisory.wgsl:13–18` declares three storage buffers and an `@compute @workgroup_size(8)` entrypoint. The draft itself explicitly excludes WebGL2 compute at lines 152 and 160. `src/engine/primitives.rs:11–29,327–333` independently confirms the 36-byte primitive vertex contract; it is separate from this compute-target issue.
- **Suggestion**: Validate all seven source families, then compile only the explicitly required entrypoints for each backend. Record advisory compute as required for supported selected compute backends, explicitly absent for WebGL2 and optional for native GL with CPU fallback. Manifest entries must distinguish intentional capability exclusion from failed translation or an unexecuted test. Define 6C common reflection/source validation plus per-target compilation gates; a platform host may start after its own applicable gate, while Task 6 remains Partial until every required target gate passes. No rendering family may disappear from the WebGL2 Low path to satisfy this correction.
- **Status**: open

### Issue 3: Dependency authorization requires evidence from a slice that already requires authorization

- **Severity**: major
- **Review class**: Important — blocking
- **Section**: Offline delivery, dependencies, and packaging, lines 219–230; concrete choices, line 249; slice 6A, line 277.
- **Description**: Line 230 says to capture exact Linux package versions, closure and notices “in slice 6A on the target before dependency authorization,” while 6A requires “Approved design and dependency decisions.” Those are circular entry conditions. In addition, the overview permits a tooling-language decision as late as “before Task 8” (line 17), but 6C depends on that decision to select the shader generator. The individual proposals are explicit and appropriately unauthorized; the problem is the sequence that makes them actionable.
- **Evidence**: These conflicting requirements occur within the draft. The global operator agreement requires an explicit user instruction before a new production dependency is added; a design reviewer cannot supply that authorization. The migration plan, `docs/superpowers/plans/2026-10-09-nyon-zig-migration.md:196–210`, requires a reviewable binding design before dependency addition, not approved dependencies before read-only proposal preparation.
- **Suggestion**: Split 6A into a read-only target/package/protocol inventory and concrete proposal phase, followed by authorized acquisition/linkage and ABI probes. Inventory existing target headers/packages without installing anything; identify missing prerequisites in the proposal. Record explicit dependency authorization as the boundary before adding/linking the new production library. Make the offline generator-language decision and any new tool provisioning prerequisites of 6C, with Task 8 auditing the implemented decision rather than first deciding it. Separate target-specific gates so unapproved Linux additions cannot accidentally become prerequisites for a macOS-only conformance probe. This correction requests no permission now and grants none.
- **Status**: open

### Issue 4: Preserve concrete numeric conversion behavior alongside byte layouts

- **Severity**: minor
- **Review class**: Minor — recommendation before renderer-interface freeze
- **Section**: Project ABI, lines 85–89; Text/logical geometry, line 139; shader reflection contract, line 156.
- **Description**: Checked lengths, signedness and layout reflection are covered, but the migration contract does not name finite-value rejection, float-to-integer range/rounding rules, or overflow behavior for host extents/scissors/uploads. The phrase “round scissor edges outward and clamp” does not specify what happens for NaN, infinity, negative or unrepresentable extents. A byte-identical vertex layout alone cannot catch changed numeric behavior.
- **Evidence**: `src/engine/render.rs:233–247` checks count addition and sanitizes logical viewport values; lines 354–378 check count narrowing, byte multiplication, device size limits and capacity growth. `src/engine/ui_renderer.rs:240–245` suppresses drawing for nonfinite/nonpositive viewports. `src/engine/primitives.rs:53–99,144–164` rejects nonfinite geometry and `:182–189` defines clamp/round/pack behavior for the upper 24 ring-width bits. The primitive ABI is size/stride 36 with offsets 0/8/24/32 (`:327–333`).
- **Suggestion**: Add a compact numeric-boundary contract to the reflection/ABI slice: reject or sanitize invalid host values before integer conversion, preserve each source operation’s finite/range behavior, specify outward scissor rounding followed by target clamping, use checked count×stride/addition/narrowing, and preserve the ring-width quantization. Include zero/negative/NaN/infinite/oversize inputs and boundary packing in source-parity tests. Keep canonical integer authority values outside float presentation conversions. This needs no new dependency.
- **Status**: open

### Issue 5: Name prerequisite interfaces instead of whole-task completion in dispatch rows

- **Severity**: minor
- **Review class**: Minor — dispatch clarity
- **Section**: Prerequisites, lines 27–36; slices 6G/H/L, lines 283–288; complete acceptance mapping, lines 332–336.
- **Description**: The prose correctly calls for interface review, but 6H’s shorthand “Tasks3/4/5” can be read as requiring those tasks to be fully complete. Task 3’s graphical completion and Task 5’s real runtime handoff depend on Task 6. Similarly, Task 7’s final player experience depends on host integration. I do not find a necessary architecture cycle in the intended design; the whole-task shorthand risks introducing one during dispatch. The reviewed draft retains the final requirements and should make their Partial status equally explicit.
- **Evidence**: Migration plan lines 95–106 and 127–130 distinguish pure 3A–3I from graphical 3J, retaining Task 3 Partial until 3J. Lines 146–158 separate core Forge from durable player Continue. `docs/qualification/2026-10-09-zig-living-inventory.md:119–120` gives 5K a pure handoff interface and keeps Task 5 Partial until actual Task 6 adoption. Migration plan lines 212–224 preserve the separate V3 freeze and strategic architecture/specification/plan approval sequence.
- **Suggestion**: State the dispatch graph explicitly: reviewed pure 3A–3I + 4A–4H + 5A–5K/headless 5L interfaces → 6B–K host/runtime/storage integration → graphical 3J and Task 5 adoption qualification. Task 7 corpus/freeze and separately approved strategic architecture/spec/plan → 6L integrated V3/strategic experience. Pure/interface milestones may pass while Tasks 3/5/6/7 and final dependent delivery remain Partial; 6M/Task 8 waits for all integrated products and required hosts. Avoid requiring final Task 6 closure before beginning the Task 7 prerequisites that 6L consumes.
- **Status**: open

### Strengths

- Preserves Metal, DX12, Vulkan, forced native GL, WebGPU and WebGL2, all required browsers, Low/High/failure recovery, all 27 LG rows, complete Forge/Living journeys, creator tools, V3 and strategic scope. Source acceptance matrix agrees with the retained hosts and performance/endurance targets (`docs/superpowers/specs/2026-09-04-nyon-living-galaxy-qualification.md:149–174`).
- Separates application policy in Zig from typed platform glue, excludes Rust-built runtime facades, and identifies target SDK/ABI/thread/lifetime proof as future work. The proposed ABI handles and copied browser spans address actual owner and Wasm-memory-lifetime risks without claiming compiled bindings.
- Correctly preserves Workshop abandonment as forgetting the outcome rather than cancelling work (`src/workshop/store.rs:441–480`), while retaining physical permits until real completion. Won durable commits cannot be reported as Cancelled.
- Correctly selects transaction completion, exact snapshot CAS and separate authority/coordinator ordering. Current primary source confirms the Living generation/head transaction (`src/living_store/web.rs:15–45`) and coordinator transaction (`src/library_selection/web.rs:23–51`); the design does not mistake either database for a cross-database transaction.
- Addresses actual gaps in Unicode/IME, native accessibility and Windows/Linux clipboard, rather than treating the current Rust limitations as acceptance scope. Text bytes remain unchanged, ownership remains explicit, and real assistive-technology qualification remains required.
- Keeps offline package contents, dependency closure, target versions, artifact identities, shader tooling and runtime retirement visible. The direct-API recommendation is credible as a staged proposal with substantial named costs; this review does not claim its performance or total implementation effort has been measured.

### Named gates retained after revision

Architecture acceptance is separate from explicit authorization to add Linux libraries or acquire new tools. The final offline-tooling language decision, concrete SDK availability/ABI probes, shader target validation, and reviewed pure-core interfaces remain prerequisites of their respective implementation slices. Strategic architecture approval and written-spec/plan review remain independent; this host design cannot replace them.

Runtime, manual and remote-platform evidence are later phase gates, not reasons by themselves to reject a design: real IndexedDB/restart/CAS/interruption; native durable replacement/locking; actual GPU and shader execution; VoiceOver/Narrator/Orca; Unicode/IME/touch; final package offline cold journeys; performance and endurance; V3 and strategic integration. Unknown Windows/Linux SDK versions, distribution closure, foreign APIs and all platform behavior remain unverified. Linked external platform documentation in the draft was not independently fetched or executed during this review, and no exact foreign signature is endorsed here.

### Evidence and execution boundary

**Source-inspected:** the complete design/brief/inventory/requirements/ledger, relevant migration and accepted resident-runtime plan sections, accepted host/performance qualification sections, and the specific primary source files cited in the issues and strengths. Read-only Git status observed the existing shared dirty main checkout; no existing file was modified.

**Executed:** bounded file reads, focused `rg`, line-number inspection, read-only Git status, creation of this review, and its document-only validation. Some initial focused path lookups returned missing-path errors before the inventory located the actual files; these were retrieval misses, not passing checks. This review ran no build, source gate, unit test, shader compiler, browser, platform binding, fixture, profile, installation, download, commit, push or PR operation. Historical gate results quoted in supplied requirement documents remain supplied evidence, not re-executed results.

**Verdict:** address Important issues 1–3 and retain the named gates. Issues 4–5 tighten implementation contracts and dispatch clarity. Re-review the corrected design before using it as the Task 6 implementation prerequisite.
