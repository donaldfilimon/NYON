## Design Document Review: Unified city contract reconciliation

### Summary

**Needs revision; integrated implementation is blocked.** Select rich-city's power, health, production and deterministic ruler capital as the city base, and reconcile the separately owned expanded-city proposal into a combat/recruitment annex. Neither proposal, the tactical owner's new selection statement, nor this recommendation constitutes an approved composed contract or runtime evidence.

**Post-review drift notice:** the owner revised E into an annex, S to32MiB transactions, and A/C to fix2 after the pinned review. The numbered findings below are an audit of the pinned revisions, not a claim that all remain present in the latest files. See the final drift addendum before using them to dispatch amendments. Unified implementation remains unapproved by this review.

Review date: 2026-10-11. Responsibility: this review file only. No source, specification, instruction, ledger, build configuration or profile was edited. Existing dirty work and concurrent ownership were preserved. The finishing plan requires reviewed subsystem contracts and one shared authority implementer; the delegated-design record permits the controller to make decisions without another ceremonial human approval pause. Independent review still applies.

### Evidence and revision boundary

Read in full: the two city proposals, the finishing plan, full-scale authority design and codec, tactical design and expanded durable-session design. Source inspection verified current role admission, topology/terrain and footprint helpers, existing economy phases/types, four-world coordinator storage/types, and seeded manifest storage. The live tree contains no files matching city/tactical/fullscale/expanded-session modules under `zig/src`; their document APIs remain Proposed. File absence is a bounded path-search observation, not a claim about all other concurrent work.

Reference abbreviations below:

- **R**: [rich city](../superpowers/specs/2026-10-11-nyon-zig-rich-city-design.md).
- **E**: [expanded city](../superpowers/specs/2026-10-11-nyon-zig-expanded-city-economy-design.md).
- **T**: [tactical ground combat](../superpowers/specs/2026-10-11-nyon-zig-tactical-ground-combat-design.md).
- **A**: [full-scale authority](../superpowers/specs/2026-10-11-nyon-zig-fullscale-authority-design.md).
- **C**: [initial full-scale codec](../superpowers/specs/2026-10-11-nyon-zig-fullscale-authority-codec.md).
- **S**: [durable sessions](../superpowers/specs/2026-10-11-nyon-zig-expanded-durable-sessions-design.md).

Hashes were pinned before substantive reading and checked again at **2026-10-11 12:57:41 UTC**. Findings apply to these exact reviewed contents, not later edits. T changed while this review was reading it; its final “Selected city base and required owner amendments” section was read and incorporated. Other six primary documents were unchanged at that check.

| Document | Start SHA256 | Reviewed SHA256 |
| --- | --- | --- |
| R | `03b322a8b1084d72e4e4821befedd37fbc2df99a528df6ea0808223f8fd87e8b` | same |
| E | `3aca0ebd0e8f3b767fcb195305f9be7695ce31fc6c168dec39abc7951aaa586d` | same |
| T | `abd50589e483f9bfdd9eae709e22b8207ba2d86ad30c7cd7304fde13c9a64b79` | `afea7435ed7fdbfc49a739fc6958fa98be64ed3bf5881b9faa5a50f911dd7f10` |
| A | `f32ae2ea88a582bcb72e72c7b7b0ba1adfb1ab43af0b06cda74c4fcacc79b125` | same |
| C | `453fb362910d214f9e2e634b890c983b89853d875c7d304b34e172f5f64e8284` | same |
| S | `6e113ae956f6c9cd189c136e0633f6eee08cce357d3357463615e6568b0ea751` | same |
| [Finishing plan](../superpowers/plans/2026-10-11-finish-nyon-in-zig.md) | `f80f5a966e3db147c8f6d3d34ca7bf4933405ff78a37872c1aac9129e363d7a1` | same |

Current code evidence:

| Source | Verified reading and implication |
| --- | --- |
| `zig/src/play_role.zig:5-29` | Creator/ruler/combined provenance and external-autonomous rejection exist. A pure ruler cannot accept a disguised creator bootstrap. Scenario genesis must have its own trusted construction path. |
| `zig/src/planet_economy/types.zig:21-66` | Existing identity is v0, with food/material, three building kinds and seven command tags. Neither new eight-resource vector may reuse those bytes. |
| `zig/src/planet_economy/rules.zig:8-56` | Pure `AlgorithmContext`, side4..32, seed low-two-bit terrain and one/two-cell topology footprints are real reuse seams. These do not supply city networking, care or power. |
| `zig/src/planet_economy/phases.zig:157-313` | Current construction precedes production; workers equal population; farm/quarry yield6/2, growth after8 supplied ticks and loss after3 shortage ticks. These are old rules, not evidence for either proposed new labor model. |
| `zig/src/planet_economy/state.zig:6-54` | Existing requested reservation is8,642,560 bytes with compile-time source assertions. No fresh layout build was run here. |
| `zig/src/coordinated_galaxy/types.zig:26-45`, `state.zig:6-37,47-79` | Existing v0 is explicitly four worlds with separate island ownership and12,582,912 coordinator bytes. A metadata manifest with512 worlds does not make this authority full-scale. |
| `zig/src/seeded_galaxy/storage.zig:3-12,30-56` | Manifest storage admits64/512/16 and bounded wire/scratch; it is distinct from populated city simulation. |
| `zig/tools/check.sh:1-36` | The scoped gate pins the compiler, blocks Rust subprocesses, and tests implemented Zig surfaces. It is not a city/combat runtime gate today. |

### Issue 1: Two incompatible mechanic registries still purport to define one city

- **Severity**: major
- **Section**: R “Units, limits and logical records,” “Buildings, recipes and construction,” “Hourly allocation and basic needs” (lines113-314); E corresponding sections (lines130-314); T “Jointly proposed paid recruitment” and “Selected city base” (lines406-456,715-734).
- **Description**: R encodes `food0, water1, timber2, stone3, ore4, alloy5, tools6, medicine7`, ephemeral power, population512 and health-adjusted half-population labor. E encodes `water=0, grain=1, food=2, timber=3, stone=4, ore=5, metal=6, tools=7`, population256 and one worker per population with homelessness/previous-shortage deductions. R's13 kinds and E's10 kinds also assign the same numeric tags to different assets. R has food/water/clinic/power growth at24 hours; E has food/water and periodic tools growth at16. T now chooses alloy/512, but calls its recruitment table a cross-owner agreement while E still says metal/256. Neither a name substitution nor concatenating the tables reconciles these economics.
- **Suggestion**: Make R the sole base registry. Keep its exact eight stored commodities, ephemeral power, health debt, half-population workforce,512 population ceiling,13-kind table, clinic and24-hour growth rules. Adopt T's alloy training costs only through the city annex. Remove E's competing base registry from normative status through its owner; preserve it as historical rejected alternatives if useful. No grain/metal aliases, extra mine/kitchen chain, or ninth commodity. Independently rederive complete ruler/care/recruitment fixtures under this single labor budget.
- **Status**: open

### Issue 2: Combat invalidates strict functional-capacity invariants

- **Severity**: critical
- **Section**: R “Units, limits,” “Buildings,” and “Conservation” (lines126-148,224-247,551-559); E “Resources, buildings and construction” (lines241-275); T recruitment cancellation (lines439-449).
- **Description**: R requires `stock+incoming<=capacity` and disallows dismantling capacity below stored/reserved amounts. It has no damage behavior. E permits damage to disable warehouses and explicitly retains goods above functional capacity. T now requires refunds to fit “current functional capacity” with maximum4096. A valid hostile hit must not make the entire candidate fail validation, destroy goods silently, or revoke cargo already admitted. For example,1000 goods/reservations in a city with1024 capacity survive losing its only warehouse, although functional capacity becomes512. The current documents do not agree which invariant validates that state.
- **Suggestion**: Add one explicit R-base damage exception: absolute per-commodity `stock+incoming<=4096` always; normal incoming deposits require current functional capacity; involuntary capacity loss preserves existing goods/reservations and exposes `over_capacity`. Reserved arrival converts incoming to stock without changing their sum. Adopt the stricter T rule for *new* discretionary refunds/cancellations: refuse when the resulting sum exceeds functional capacity, preserving the job/cargo. Do not import E's65535 hard bound or forced-refund privilege. Repair may restore capacity without generating goods. Distinguish damage from deliberate dismantling: keep R's deliberate housing/capacity protections. A ruined hall loses housing/service but retains512 base ledger capacity so ownership and recovery remain representable. Test damage→arrival→refund refusal→repair→successful cancellation, across occupation and replay.
- **Status**: open

### Issue 3: Pinned uncancelable freight conflicts with capture and road damage

- **Severity**: major
- **Section**: R “Cube placement” and “Local trade” (lines153-188,315-329); E “City topology” and “Local logistics” (lines195-220,353-379); T occupation/city effect contract (lines328-404).
- **Description**: R says “A shipment cannot be canceled or retargeted after publication” and “future route availability cannot invalidate admitted delivery.” E advances a shipment one edge per tick, can stall indefinitely after removal/capture and permits cancellation returning all cargo. Occupation changes endpoint and road ownership; damage may remove functional roads under an unspecified R annex. Thus R's every-route-remains-valid invariant cannot simply survive the tactical insertion. E's quantity256 and R's64 also differ. Both otherwise correctly reserve destination capacity and avoid departure-tick arrival.
- **Suggestion**: Retain R's quantity1..64, path2..65 and paid road construction. Adopt E's explicit launch owner, position, no departure movement, one-edge movement, stalled reason and cancel command. Capture suspends movement when endpoint ownership differs; validate the next edge's allowed owner/road state. Preserve cargo and the destination reservation until successful arrival or cancellation. Authorize cancellation to current source owner, return cargo only if R-base functional capacity permits, atomically release destination reservation, and keep the ID burned. Keep R's deliberate dismantle protection for active routes; involuntary capture still has explicit stall semantics. Specify roads as noncombat targets in this first annex, with cover=false, to avoid introducing bridge/road destruction rules. The host displays an ETA only while unstalled. Add complete source-capture/destination-capture and repaired-capacity recovery witnesses.
- **Status**: open

### Issue 4: No single workforce and phase registry can currently be executed

- **Severity**: major
- **Section**: R “Hourly allocation” and “Commands, receipts and phase order” (lines249-314,495-508); E global phases and workforce (lines114-128,249-298,404-417); T fixed phase order and recruitment (lines164-190,430-438).
- **Description**: R runs waterworks→farms→clinics→construction→industry→spaceports and activates completions next hour. E assigns construction first, then training, then all production by priority and allows completion to produce immediately. T repeats construction→training→production, which would starve R's essentials-first tiers if adopted literally. R freezes0..9 phase tags without tactical rounds. Occupation freezes all E labor for one tick, but R has no corresponding power/care/growth semantics. Pasting T before R's existing numeric phases is an incompatible history change.
- **Suggestion**: Freeze one new composed registry: structural/root preparation; public commands; copied city/hostility projections; four tactical rounds; validate/apply damage then occupation; local freight; city infrastructure/workforce snapshot; R operating tiers with **training after construction and before industry**; household needs/health; validation/history; prepared barrier; one publication; retirement. Keep next-hour construction activation. Within each global phase visit every world before the next; within city operations preserve R tier and priority order. Define occupation as zero worker assignment, no growth-streak advance that hour; households still consume and health responds to actual supply. Connected intact solar can supply its zero-worker service, unless damage disables it. Newly trained reserves become eligibleT+1. Commands that change labor, population, integrity or roads must affect the stated snapshot boundary exactly once. Freeze numeric tags and rejection precedence in the composed codec, not in parallel modules.
- **Status**: open

### Issue 5: Empty genesis cannot deliver the required pure-ruler journey

- **Severity**: major
- **Section**: R “Trusted capital genesis” (lines378-444), E “Founding, creator operations” and genesis (lines315-351,553-571), C genesis (lines51-102).
- **Description**: E says “Fresh genesis has no city, goods or population” and requires creator bootstrap. R binds a trusted capital scenario with population24, a productive capital for every civilization and visible endowment receipts. C encodes no entities, initial policy absent and only three next-ID counters. All three are internally different genesis contracts. Calling the creator bootstrap under ruler policy would contradict current `SessionPolicy.admit`; silently installing a capital into C would change its frozen bytes.
- **Suggestion**: Retain R's `nyon.capital-start.v1` and explicit empty scenario, with capital start the pure-ruler default. Bind manifest, selected scenario, exact initial policy, all authored assets and waived costs in the new composed genesis. Keep seeded manifest and C's initial-economy identity unchanged. All civilizations receive capitals independent of selected player. Freeze the successful side4..32 placement algorithm and independent checked-in oracle, including terrain predicates and tie order, before source dispatch. R's disposable116-case Python assertion is author-reported design evidence; it was not rerun here and is not accepted Zig/runtime qualification. Prove the entire public-command capital→industry→care→growth→recruitment→supported satellite story, not just two supplied hours.
- **Status**: open

### Issue 6: Road and claim accounting must select one physical model

- **Severity**: major
- **Section**: R limits/placement/construction (lines123-148,153-247); E quotas/layout/roads (lines139-183,202-220); A quotas and sparse storage (lines64-157).
- **Description**: R puts roads inside16384 building/project quota,256/world and256/city, retaining36864 claims globally/576 per world. E adds32768 road records,4096/world and64 buildings/city; its128272-page combined estimate leaves2799 pages but does not specify whether separate road occupancy also consumes A's occupied-claim rows. If it does, another32768 claim pages cannot fit; if it does not, the single canonical occupancy lookup/invariant must change. E road completion at admission and R road projects also produce different access timing and construction costs. Farms differ: R requires fertile anchor only; E requires both cells fertile. Other E two-cell structures differ from R's one-cell structures.
- **Suggestion**: Use R's road-as-building record, stone2 paid project and256/city subject to world/global caps. Use R's footprint and anchor-terrain rules; retain roads/structures as exclusive claims and future-road adjacency for placement only. Do not add E's separate road pool or64/city cap. The proposed combined envelope then is **83216 rows and95504 pages/root**: A74512/86800 +2048 training +6656 tactical; headroom35567 pages. This is arithmetic only and includes A's reserved future strategic families; it is not proof their eventual schemas fit. Every training reservation lives in its city training record, not a duplicate tactical reservation page. Validate one256-byte record each, route children, updated counters, old/new roots, detached transitions and tactical occupancy as a separate overlay.
- **Status**: open

### Issue 7: Recruitment agreement omits a common schema and end-to-end crew accounting

- **Severity**: major
- **Section**: E recruitment (lines381-437); T recruitment (lines406-456); R conservation and building records (lines143-149,538-550).
- **Description**: E and T have matching class crew/durations but currently disagree on metal/alloy, population bound and refund capacity. R's population equation omits training and unit custody. T defines a global realm-zero UnitId while city TrainingId is planet-local; unqualified producer matching could duplicate or misroute grants. R's building refund formula is proportional unfinished work, whereas training uses half original goods; merging them by generic project cancellation would change one rule accidentally.
- **Suggestion**: Keep the narrow T/E custody protocol:2048 jobs,32/world,4/city; reserve unit slot and global ID before spending; crew1/2/2; food8 and tools2 for infantry, alloy8/tools4 armor, alloy6/tools6 artillery;2/4/3 staffed training hours. Keep two workers/job, admission retains one resident, full crew transfer and half-goods training refund distinct from R building cancellation. Use population512 and strict refund capacity from Issues1-2. Completion converts reservation to one reserve unit at full HP, crew preserved, eligibleT+1; capture transfers unfinished jobs but no existing reserves. Freeze grant producer `{planet,TrainingId}`, reserved UnitId and replay uniqueness. Extend conservation to city residents + training crew + live/reserve crew, with explicit growth/death/casualty sinks and creator mints. Define horizon-delayed completion without wrapping eligibility. No public grant API.
- **Status**: open

### Issue 8: Effect and canonical byte contracts are incomplete and mutually inconsistent

- **Severity**: major
- **Section**: R commands/history (lines446-536); E effects and codec (lines439-484,545-665); T city boundary and save grammar (lines353-404,556-582); C scalars/state/receipts (lines10-261).
- **Description**: E occupation bytes omit ObjectiveId while T requires it to complete the right objective. E cover excludes farm only; T now excludes roads/farms/solar. E embeds planet in command header withu8 tag; R uses outer planet, qualified refs andu16 tag. R capital tie order names north0/east1/south2/west3 while E/C directions encode east0/west1/north2/south3. C allows only kinds0..4, two commodities, u8 population, seven commands, no scenario endowment and no combat. R/E/T defer full persistent record/receipt grammars. E and T both budget city-effect outcome records, but neither fixes the unique emitter. A hash of an underspecified record is not a replay contract.
- **Suggestion**: Assign one owner the composed codec appendix before implementation. Use distinct typed command-family tags, one outer route, explicit direction wire tags, one phase/receipt ordinal registry and one effect outcome emitter. Retain capital candidate tie order as an explicit algorithm order even if direction wire tags use existing east/west/north/south; do not reinterpret fixtures. Unify effect types with target tick/source revision, qualified structure, ObjectiveId on occupation and deterministic ordinal. Exclude roads as targets; intact non-farm/non-solar completed structures provide cover. Freeze every city/training/unit/objective/counter/claim field, all source/sink receipts, state traversal, genesis chunks and rejection precedence. Keep C normative only for its initial identity. Do not append city records to C or call deferred tables implementation-ready. Independently encode maximum records, all tags and full endowment/training/capture fixtures.
- **Status**: open

### Issue 9: Durable sessions reject the proposed transcript and retention envelope

- **Severity**: critical
- **Section**: S portable graph/reservations/work limits (lines258-329,443-489,580-616); A global history/provider/M1 (lines326-488); R receipt bound (lines525-536); T history accounting (lines536-554).
- **Description**: S fixes complete transactions at1MiB, two1MiB unsaved buffers and objects fitting4MiB physical batches. A reserves32MiB/tick and its initial codec alone bounds4057265 bytes; R bounds6258688 and T5292096, before a fully reconciled annex. R+T's conservative11550784 bytes over100000 ticks is approximately1075.75GiB, exceeding S's144GiB container/512GiB namespace. This multiplication is an envelope observation, not a claim each per-tick maximum is simultaneously reachable or the required mixed workload produces that size. T's city-effect duplication and shared command cap mean the final bound must be derived afresh. Simply raising the one node length breaks batch and128MiB transient reservations. S also does not yet implement A's exact Prefix/Ticket/authenticated-segment retention protocol. The controller separately reports A's provider/frontier rereview still open; this review does not close it.
- **Suggestion**: Require the session and authority owners to publish one reviewed transport adapter and capacity amendment. Keep bounded1MiB transport segments while allowing a complete logical transaction up to the reviewed32MiB authority envelope; specify transaction spanning segments, digest/commit linkage, partial-tail history, exact-prefix pins, stale completions and GC closure. Recalculate queue/index/batch memory and portable/namespace maxima together, including source+target and import/recovery overlap. Pin real100000-tick workload bytes and prove they fit the chosen fixed limits; maximum-load stress remains separate. Backpressure protects custody but cannot substitute for required endurance. Do not authorize city serializer or session implementation against the incompatible current envelopes.
- **Status**: open

### Issue 10: Work metering does not establish the fixed5ms authority target

- **Severity**: major
- **Section**: A sorted index and alternatives (lines110-122,572-597); T work accounting (lines523-535); R work cursors (lines561-581); finishing plan acceptance (lines30-37).
- **Description**: A allows linear index moves at up to131072 rows for each insertion and copies/retains all occupied rows every tick. T explicitly allows4096 units×4 rounds×6144 vertices =100663296 BFS vertex visits before edges. Both city proposals add connectivity scans, worker sorting, receipt encoding and history hashes. Charging this work correctly only bounds polls; it does not make the complete maximum-capacity authority tick meet p95≤5ms. No named-host measurement or reviewed asymptotic improvement establishes that target here. Record/scratch arithmetic likewise does not establish allocator or Wasm capacity.
- **Suggestion**: Preserve the5ms full-tick target and existing caps. Gate implementation-plan approval on a named-hardware feasibility experiment with final loaded fixtures and an algorithm review. Use city component reachability once per dirty network/world rather than repeated unconstrained world scans; meter cache rebuilds and maintain deterministic results. Investigate shared-goal or exact precomputed topology distances for combat and a bounded chunked/stable-key index for bulk mutations; independently prove equivalent movement/tie order and account their memory before selecting them. Do not silently reduce units/worlds, split publication or replace full-tick measurements with256-unit poll latency. Report request copy, simulation, validation, encoding/hash and retirement separately, with10,000 samples and worst legal changes.
- **Status**: open

### Issue 11: Founder and capital fixtures must use the same selected mechanics

- **Severity**: minor
- **Section**: R local founding and gameplay journey (lines330-349,609-682); E founding (lines334-342).
- **Description**: E requires a road path and transfers4 people/16food/16water; R uses a documented frontier expedition with32food/32water and no route requirement. Selecting E's road requirement while retaining R's story, labor and seed placement would introduce an unreviewed startup constraint. R correctly notes its4-person satellite supplies only2workers and cannot staff its4worker farm; prose about “establishing” a satellite is not a sustainable supply proof.
- **Suggestion**: Preserve R's explicit paid founding/resettlement abstraction and stocks as the bounded city-v1 decision. Freeze the supporting road/freight schedule and food imports in the complete acceptance story, including production/crew diverted to recruitment and care. Treat autonomy, interplanetary transport and military hostility production as separate still-required strategic contracts; do not add a substitute feature list to this city review.
- **Status**: open

### Recommended owner handoff and remaining proofs

The following are recommended decisions for the controller, not changes made to other owners' files:

| Owner responsibility | Concrete artifact needed before dependent implementation |
| --- | --- |
| Controller | Name one lead unified-city specification author and confirm annex ownership with the other session; pin the chosen R base and retired E alternatives. Do not run two city implementation agents. |
| Rich-city lead | Incorporate the reviewed integrity/capacity/freight/occupation/training amendments; retain power/health/capital, next-hour activation, road quota and one commodity registry. |
| Expanded-city owner | Produce only the compatible combat/custody annex, remove conflicting base identities/numbers from normative status, and reference the lead's exact revision. |
| Tactical owner | Match city type/cover/road rules, grant/effect fields, strict refunds and operating phase insertion; retain paid ruler troop journey and explicit hostility-producer dependency. |
| Authority/codec owner | Publish one composed identity and complete codec/phase registry; verify83216-row/95504-page proposed envelope,256-byte records,8MiB city/shared plus8MiB tactical scratch partitions, roots/transitions and owned allocation accounting. Resolve existing independent provider/frontier findings. |
| Session owner | Align32MiB logical transaction support with segmented transport, exact-prefix retention, graph limits and bounded buffering; re-review durability/GC/cancellation before consumption. |
| Independent reviewer | Review the unified pinned revision and exact witnesses; return blocking findings without silently selecting new gameplay during source implementation. |

Required proofs are distinct:

1. **Semantics and bytes:** independent capital placement and arithmetic, exact complete journey, immutable endowments, needs/power/health conservation, worker tiers, proportional building refunds versus half training refunds, combat damage/occupation and stalled-cargo recovery, all tags and maximum receipt multiplicities.
2. **Lifecycle and scale:**512-world all-phase participation; legal maximum populations/entities; source and target roots; zero-budget immobility; no poll allocations; cancel/abort during every ownership/copy/effect/grant transition; old leases; counter exhaustion; exact shared quota contention.
3. **Persistence:** complete transcript accepted by the selected container, exact-prefix provider authentication and retention, damaged storage with retained cargo, saveN/persistN+1, abandoned mutations, corrupt/missing history, selected-branch Continue, no legacy namespace access.
4. **Measured delivery:** named-host full-tick p95≤5ms and bounded polls, actual native/browser memory and durability,100000-tick complete retained history, Rust-blocked scoped gate, later real ruler combat/save/relaunch journey. CLI or layout success alone cannot close these rows.

### Strengths

- Both city drafts now acknowledge concurrent ownership and prohibit two competing authorities.
- R supplies concrete player choices, ephemeral power and measurable health effects, an explicit pure-ruler endowment, independent arithmetic anchors and honest satellite staffing limits.
- E supplies the strongest conservation treatment for damage, freight stalls and training custody; these can be retained without importing its competing recipes.
- T retains simultaneous combat, copied effects, single publication and crew casualty accounting, and its latest revision explicitly identifies required rich-city amendments.
- A clearly separates canonical records from physical storage, counts source plus replay target, and does not treat poll metering as full-tick timing evidence.
- S specifies exact Continue, namespace CAS, resident versus operation pins and truthful abandonment outcomes; the byte-envelope conflict is explicit and repairable before implementation.

### Verification record and limits

**Current:** full primary-document reading, code inspection and SHA256 drift checks completed. Python standard-library arithmetic confirmed83216 rows,95504 pages and35567-page headroom for the recommended no-extra-road-pool envelope; R+T bound sum11550784 bytes and100000-tick multiplication above. Arithmetic/hash command exited0. These are static calculations, not runtime qualification.

**Partial:** T changed during review; the final pinned revision was incorporated. Concurrent later edits require rereview against new hashes. The controller's report about open A provider/frontier rereview is attributed coordination evidence, not a finding independently retested here.

**Unverified:** new record/control compiler layouts; startup placement116-case result; actual authority/session integration; memory/RSS and5ms timing; external native/IndexedDB durability behavior; complete long-run retention and all manual gameplay. No remote services or external-system claims were verified.

**Not run:** executable Zig or historical Rust gates. This task writes review notes only; existing code gates cannot demonstrate nonexistent proposed city/combat contracts, and no source was changed. Future implementation must run the documented scoped Zig gate and relevant new qualification, recording actual exit status. No commit, push, dependency, deployment or profile operation occurred.


### Post-review drift addendum

Final verification observed four further owner edits after the complete reading boundary:

| Document | Later observed SHA256 | Review disposition |
| --- | --- | --- |
| E | `00c99e03855c8f4e87e08857d60988132b970752cf0d61f07483ab6d2eb01f7b` | Now titled “city combat and paid recruitment integration annex.” Targeted reading confirms rich-city base, alloy/512, roads in building quota,4096 hard storage and strict training refunds. Several requested decisions have been adopted; this revision was not independently read/reviewed in full here. |
| S | `5dd3743f7eca2f667e7fd89568567a3e0e59c9ac36c8783f37b7b4d82edaca43` | Targeted reading confirms32MiB logical ticks,1MiB segments and revised provider/buffering/retention arithmetic. **Issue9's literal1MiB incompatibility is superseded as a current-text claim.** The new complete session contract needs its own independent review; no PASS is implied here. |
| A | `a8bc0d41767722b3215e2e6c334d06d5cf5cacc3107f28f83b3b4219ba6ce2f5` | Controller reports provider/frontier fix2 and independent approval to planning. This review does not repeat that rereview or continue to assert the old frontier finding against fix2. |
| C | `c116655859eae999ee1529180521254a575570ba471099fc0ca2e00c8778b6b9` | Same fix2 boundary; remains an initial-economy codec, not reviewed city/combat schema evidence. |

R and T retained their reviewed hashes at this final check. E's newer annex deliberately retains guaranteed admitted freight and therefore differs from Issue3's recommendation to stall/cancel on capture. That is a controller/owner decision requiring a single reviewed invariant, not permission for an implementer to combine both semantics. The review's principle is conserved custody and an explicit capture outcome; either chosen deterministic model needs complete tests and matching city text.

The11 issue statuses are preserved as **open against the pinned review snapshot**. Adoption in an owner draft is useful progress but cannot close integrated review until R, annex, T, authority codec and S point to one mutually compatible pinned composition. Do not apply obsolete E replacement instructions to the rewritten annex or restore obsolete session limits. The final artifact structure/whitespace check and scoped `git diff --check` command exited0; the review is new/untracked, so the direct structure/whitespace check is the substantive artifact check. No executable gate ran.
