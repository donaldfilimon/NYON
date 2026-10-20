## Design Document Review: Rich-city reconciliation fix1

### Summary

**Needs revision: city mechanics are substantially reconciled, but two narrow city-codec ambiguities remain before city-side signoff. Integrated implementation remains blocked by cross-owner composition disagreement and outstanding layout/performance/durable qualification.** The independently checked scalar hex, resource fixtures and declared event arithmetic agree with R fix1; this review grants no implementation approval.

Scope: independent follow-up to [the original reconciliation review](2026-10-11-city-contract-reconciliation-review.md), preserving that raw artifact. This task owns only this new review file. Read R fix1 in full; read connected clauses in E/T/A/C/S to identify residual integration conflicts without repeating their complete independent reviews. No source, other specification, ledger, instruction or profile changes; no executable gates.

### Pinned evidence and drift

R is [rich-city design](../superpowers/specs/2026-10-11-nyon-zig-rich-city-design.md); E is [city integration annex](../superpowers/specs/2026-10-11-nyon-zig-expanded-city-economy-design.md); T is [tactical design](../superpowers/specs/2026-10-11-nyon-zig-tactical-ground-combat-design.md); A/C are [full-scale authority](../superpowers/specs/2026-10-11-nyon-zig-fullscale-authority-design.md) and [initial codec](../superpowers/specs/2026-10-11-nyon-zig-fullscale-authority-codec.md); S is [durable sessions](../superpowers/specs/2026-10-11-nyon-zig-expanded-durable-sessions-design.md).

At review start **2026-10-11 13:17:31 UTC**, local read snapshots were taken with these SHA256 values. Line references below apply to these pinned revisions.

| Document | Start/reviewed SHA256 |
| --- | --- |
| R | `25cd230a3f4bdaf83f14e119269263896df269f27104ce09a7d5daa07717238c` |
| E | `a2a05f17a620cde2853a437633d1e77e892cb54a4a23c1995bdc74584dc50afa` |
| T | `afea7435ed7fdbfc49a739fc6958fa98be64ed3bf5881b9faa5a50f911dd7f10` |
| A | `a8bc0d41767722b3215e2e6c334d06d5cf5cacc3107f28f83b3b4219ba6ce2f5` |
| C | `c116655859eae999ee1529180521254a575570ba471099fc0ca2e00c8778b6b9` |
| S | `ce8f58f9a4427a741b1fa72586975cee534ce18977669370af35edc78970b4f5` |

At **13:30:13 UTC**, R/A/C/S were unchanged. E changed to `27cd50ef335008aac70ef48b9b91abe242b7c765dabe6c878bb4027a2eaa46a3` and T to `a5146b601890d4999256242faae5701a668c610073970bd62ea3cb9ef44b4738`. Those later revisions are not reviewed here. Cross-owner findings below therefore describe the pinned E/T conflicts and required verification, not assertions that the later text necessarily retains every conflict. Recheck matching revisions before dispatch.

Current source evidence remains the prior review's verified `zig/src/play_role.zig:5-29`, `planet_economy/{types,rules,phases,state}.zig` and `coordinated_galaxy/{types,state}.zig`: old provenance rules, two-resource v0 and four-world authority are separate from the Proposed city composition. This follow-up makes no new source-behavior or runtime claim. A/C fix2 approval to initial-economy planning is controller-reported review status and does not register rich-city fields under the old identity.

### Disposition of the original findings

These are **city-side design dispositions**, not integrated closure or edits to the original review statuses.

| Original issue | R fix1 result and source |
| --- | --- |
| 1 goods/labor/population | Addressed on the city side: one eight-commodity registry, ephemeral power,512 population, health-adjusted half-population workers; R114-156,260-328. |
| 2 damage/storage | Addressed: absolute4096 bound, retained stock/reservations after involuntary capacity loss, positive-output checks, strict new/refund deposits and retained hall base storage; R134-141,458-469,485-494. |
| 3 freight | Addressed as a selected city direction:64-unit cargo, launch-owner/position/stall state, preserved destination reservation, current-source-owner cancellation with strict return capacity; noncombat roads; R330-361. E must adopt the same rule. |
| 4 phases/workforce | Addressed city-side: explicit0..21 phase registry and four rounds, essentials before construction/training, training before industry, next-hour activation, occupation suppresses workers/growth while intact solar survives; R275-291,485-494,684-711. T's exact event placement still requires matching. |
| 5 genesis/ruler | Playable trusted scenario is retained with explicit new composed identity, initial policy and endowment provenance; R534-601,713-811. Exact initial-record defaults still need Issue1 below. |
| 6 roads/quotas | Addressed city-side: roads consume building quota, no extra road pool, claims bounded by hall plus footprints, per-world component traversal; R126-156,158-197. New record/control layouts remain unproved. |
| 7 training/crew | Addressed: paid alloy-based recruitment, qualified TrainingId/global UnitId custody, reservation before spend, eligibleT+1 reserve, capture/cancel semantics and crew conservation; R496-532,964-987. |
| 8 wire/effects | Much improved: city family0/tags0..17, explicit record kinds/fields, rejection codes, effect ObjectiveId, unique city outcome emitter and numeric phases; R603-962. Issue2 below remains; tactical payloads are explicitly deferred. |
| 9 session envelope | Original literal1MiB conflict is superseded. Pinned S admits32MiB logical transactions with1MiB transport segments. Registration and independent durable review remain separate; R1190-1191 does not claim otherwise. |
| 10 performance | Still open and correctly retained: R1200-1208 explicitly preserves loaded full-tick p95≤5ms, caps,10,000 samples and independent poll targets. Component traversal improvement alone is not timing proof. |
| 11 founding/journey | Design direction addressed: paid frontier abstraction and supported satellite remain explicit; R363-382,1093-1100. The exact public-command full journey is correctly still a preimplementation proof, not claimed complete. |

### Issue 1: Canonical capital fields need explicit defaults for every building

- **Severity**: major
- **Section**: R “Trusted capital genesis,” lines550-593; “New genesis and canonical state,” lines749-790.
- **Description**: The template says “All productive buildings enabled; priorities0 for life support,1 for forestry/quarry,” while the canonical building payload contains `enabled:bool,priority:u8` for **every** building. Solar priority and road/residence enabled/priority values are not fixed by that statement. The later author-building default of enabled=true/priority1 is a command rule; scenario genesis explicitly is not that command. Different implementations can construct mechanically identical capitals but different genesis bytes/G/history unless these otherwise irrelevant fields have one required value. The paragraph on authored/genesis records fixes paid cost, work and activation but does not close enabled/priority.
- **Suggestion**: Add one explicit canonical template/default table for all14 buildings, including passive roads/residence and solar. For example, uniformly enabled=true/priority1 except Waterworks/Farm priority0 would be a coherent choice, but the author must freeze the selected values. Require the decoder/recomputed-genesis comparison to reject alternatives. Apply the same explicit-default check to empty/author/founded/resettled city fields rather than relying on host struct initialization. This is a narrow codec clarification, not a request for a new mechanics design.
- **Status**: open

### Issue 2: Accepted-command resource fields do not unambiguously map every custody transition

- **Severity**: major
- **Section**: R “Framed events and sole emitters,” lines890-896,929-937; local freight lines330-361; recruitment lines496-532.
- **Description**: The accepted payload has debit/credit vectors, population_moved, waiver values and two IDs, but their mapping is described generically as “In a two-city transfer they are source debit/destination credit.” `ship_local` is a two-city transfer whose destination gets only a reservation at admission; its stock credit happens on later arrival. Literal destination credit in the command receipt would falsely record received stock or double-count it when freight delivers. Founding/resettlement combine consumption and transfer; creator grants mint; cancellation returns custody. Inputs allow these to be reconstructed, but the promised exact event bytes still need a single explicit mapping rather than implementer interpretation. `created_local`/`counterpart_id` also need per-tag unused/created/retired semantics, not only the training exception.
- **Suggestion**: Add a concise per-command-tag accepted-effect table for tags0..17. In particular, ship/author-ship transfer records source debit and zero destination stock credit at departure; minted cargo records mint origin without source debit, with stock arriving only in freight_hour. Specify founding source debits versus destination credits, grant/author-city mint fields, recruit/cancel crew and paid goods, repair waivers and zero-effect policy commands. Clarify that original command/tag plus fields distinguish explicit mint, material sink and custody move; preserve current bounded payload. Independent examples should cover transfer versus minted cargo and founding consumption versus transferred supplies.
- **Status**: open

### Issue 3: Two documents claim incompatible composition authority

- **Severity**: critical
- **Section**: Pinned E “Composition authority and immutable selected revisions,” lines264-284, “Single phase registry,” lines286-297, and freight lines250-260; R “Selected base,” lines47-71, freight lines330-361, phase registry684-711, owner amendments1183-1192.
- **Description**: Pinned E says it is “the normative composition manifest” and “overrides conflicting standalone base, tactical, full-scale and initial-codec statements.” It pins rich-city hash `9939e488...`, not reviewed R fix1 `25cd230a...`. E retains guaranteed captured-destination delivery, damageable roads and phase0..14 with combat_round5. R selects stalled/cancelable freight, noncombat roads and phase0..21 with tactical subphases3..12. Both use `nyon.city-combat-galaxy.v1`; a rule name therefore does not identify one byte/behavior contract yet. R candidly identifies most of these as owner amendments, but no implementer can resolve document precedence safely.
- **Suggestion**: Follow the controller's explicit decision received during this review: R fix1's city direction is selected Proposed input, not integrated approval. The controller must name a single composed-contract owner; E owner must repin the accepted R revision, remove the stale overriding claim or restrict it to one agreed manifest, and adopt exactly the selected freight/road/occupation/phase semantics. T must repin the same composition and remove references to the old city-only0..9 registry. Review the resulting exact set together. Preserve both owners' work; do not silently restore an older R because E's prose says “normative.” Later E/T drift means this exact issue requires fresh comparison before closure.
- **Status**: open

### Independent arithmetic and scalar verification

A disposable Python standard-library calculation imported no game code, serializer or recipe implementation. All assertions exited0. These are design checks, not compiled layout, decoder or gameplay tests.

| Check | Independently verified result |
| --- | --- |
| Infantry recruit command | Body30 bytes, outer frame36 bytes; both literal hex strings at R860-865 match `struct.pack` from the displayed grammar exactly. |
| Maximum city/building/training/world payloads |98 /53 /60 /60 bytes; canonical key+length adds16. These logical lengths do not establish Zig physical layout. |
| Command input and accepted outcome |1055 /150 framed bytes, within1056 /256 reservations. |
| City/hour and building/hour |136 /124 framed bytes, within256 /192 reservations. |
| Freight, training, deployment grant |63 /53 /74 framed bytes, within96 /128 /128. |
| Occupation effect input and city outcome |69 /67 framed bytes, each within96. |
| Scenario city/building/stock |133 /122 /40 framed bytes, within256 /192 /64. |
| Base city byte envelope |6258688. |
| City plus training/effects |7962624 =6258688 +2048×128×2 +6144×96×2. |
| Raw tactical allocation |4784128 =16384×256 +8192×64 +512×128.128-byte theater reserve fixes the old32-byte impossibility for16u32 scores, but its complete tactical payload still needs review. |
| Combined tick allocation |12746752 bytes, below33554432 by20807680. Shared command/effect outcomes are counted once. |
| Genesis object upper-envelope sum |150528 bytes under262144 cap; includes512 zero-score theater records. |
| Initial receipt stream |55488 bytes under65536 cap. |
| Claimed rows/pages |74512+2048+6656=83216;86800+2048+6656=95504;131071−95504=35567 page headroom. Arithmetic is correct; family ownership/layout proof remains outstanding. |
| Source+target reservation |197197824×2=394395648;402653184−394395648=8257536 bytes. Assumes city scratch fits the existing8MiB and tactical needs exactly8MiB extra. |

R's capital recruitment tables independently agree:

- Tick1 after one infantry recruit: population23,11 available workers; waterworks2+farm4+training2+forestry2 uses10; quarry idles. Stocks `[193,265,102,96,24,24,14,8]`; unused power17.
- Tick2 completion: stocks `[202,274,108,96,24,24,14,8]`, same labor/power; reserve eligibility3 preserves one crew outside city population.
- Tick3 with deployment and no training: stocks `[211,283,114,102,26,24,14,8]`; quarry now runs, unused power15.
- Alternative cancel branch at tick2: `[205,273,108,102,26,24,15,8]`, population24 and no job/unit, matching the stated half-goods/full-crew refund.

The event sum is a safe-envelope calculation conditional on its stated emit multiplicities. It does not validate unspecified tactical UnitRound/ObjectiveRound/Theater payloads or prove achievable maximum output. Full hex/hash fixtures, all city command-effect mappings, complete public-command journey and emit-site proof remain required. The one new global metadata row must also be assigned explicitly within the composed storage family's envelope; inherited A counts are not automatic ownership for new kind0 despite ample aggregate headroom.

### Exact remaining owner amendments

| Owner | Required bounded amendment/proof |
| --- | --- |
| R author | Resolve Issues1-2; retain existing selected mechanics and record/event sizes; repin the accepted corrected city revision. No broader redesign requested. |
| E owner/controller | One manifest authority, one pinned R revision; cancelable/stalled freight64, noncombat roads, occupation suppresses workers while passive solar remains; phase0..21/tier placement and family0 command17. Adopt R's effect input/outcome ownership and remove obsolete conflicting totals. |
| T owner | Match nonroad projection, ObjectiveId/qualified producer/crew:u8, training after construction before industry, next-tick reserves, one effect outcome emitter, phase3..12 barriers/four rounds; supply family1 tags and exact kinds7/8 plus kind9 sixteen-score payload and event64/65/66 within256/64/128. Do not count command/effect records twice. |
| A/C owner | Keep initial `nyon.fullscale-galaxy.v1` codec unchanged. Separately register city-combat profile/52 quotas, explicit added global/city/training/tactical record families,262144 genesis,65536 initial stream and12746752 tick reservation. Prove all page/control/scratch/root ownership, not merely subtraction from headroom. |
| S owner | Register the new G/RC1 history grammar using existing transport-tree domains and exact logical transaction boundaries; accept the declared bounds only after independent retained-prefix/partial-tail/CAS/GC/source-target review. Pinned32MiB support resolves the obsolete numerical1MiB objection, not runtime persistence. |
| Plan/qualification owner | Freeze independent exact capital defaults, full event/command vectors and complete city→care→troop→supported-satellite journey. Preserve p95≤5ms at declared capacity,100000 retained ticks, separate poll timing and actual host durability evidence. |

### Strengths

- R fixes the actual mechanics conflicts rather than adding a second resource or road family.
- Damage conserves stored and reserved goods while normal deposits stay bounded; freight capture now has explicit custody and recovery.
- Recruitment fixtures account for lost worker capacity, half refunds and transferred crew; they do not assume every startup producer remains staffed.
- The appendix distinguishes logical city hashes, provider transport hashes and session node identities, and assigns one emitter to city effects.
- The writer correctly leaves tactical payloads, complete journey, physical layout and measured performance as unresolved gates rather than declaring implementation readiness.

### Verification and conclusion boundary

**Current:** full read of pinned R fix1, targeted connected-contract review, independent byte-length/hex/resource/arithmetic assertions, and start/end SHA256 checks. Independent calculation command exited0. No production code was imported by those calculations.

**Needs revision:** two narrow city-codec clarifications and one integrated composition-authority conflict. Raw original findings remain untouched. City mechanics are adequate as the selected direction; exact-byte city signoff waits for Issues1-2, and integrated signoff additionally waits for consistent owner revisions and their required proofs.

**Unverified:** complete capital116-case placement, full checked-in event/hash vectors, tactical codecs, Zig record/control layouts, allocator/cancellation tests, named-host5ms measurements, durable/runtime/native/browser behavior and the complete gameplay journey. No executable gate, commit, dependency, deployment or profile operation was run. Later E/T revisions noted above require their own scoped comparison; this review does not transfer approval to moving documents.
