# Task 7 V3 corpus prerequisite inventory

Observed 2026-10-09. **Partial: rules approval is recorded; a current independently qualified FULL corpus and coherent initialization-amended package freeze are not established.** Task 7 retains these prerequisites before separate V3 implementation, and separately retains strategic architecture/spec/plan review and actual host experience delivery. This report does not complete Task 7. [docs/superpowers/plans/2026-10-09-nyon-zig-migration.md:212-224; docs/qualification/2026-10-09-zig-acceptance-ledger.md:55-57]

Scope: source-only inspection of Task 7 and its referenced plans, eight-spec package, initialization design, corpus helpers/coverage, and review/freeze evidence. No generator, verifier, build, runtime, network, profile, dependency, Git mutation or golden update ran. Only this report and `2026-10-09-zig-v3-corpus-inventory.json` were written. The JSON records raw SHA256/byte identities for all 209 non-cache files in the nine referenced V3 fixture directories, source/freeze comparisons and selected review evidence. These are file identities, not admission or runtime proofs. [2026-10-09-zig-v3-corpus-inventory.json, § corpus_files]

## Approval and source authority

- **Recorded approval:** the newest completion ledger records Donald's 2026-10-08 19:32 EDT instruction as approval of current proposed V3 rules, explicitly preserving repair-transition, initialization/corpus coverage, integrated independent review and immutable-freeze prerequisites. Do not repeat older “human rules approval absent” headers as the current approval verdict. No implicit V2 conversion is approved. [docs/qualification/2026-10-08-living-completion-ledger.md:127-132]
- **Written review, narrower than full package acceptance:** initialization Task 3 received Spec Compliance/Task Quality approval for the complete written contract and readiness for Donald's review. Its preserved planning-handoff ruling permits package/corpus planning and explicitly distinguishes that from corpus/rules completion. These historical receipts must be composed with the newer approval ledger, not substituted for it. [.superpowers/sdd/2026-10-03-complete-nyon-v2-then-v3/task-3-review.md:1-16; .superpowers/sdd/2026-10-03-complete-nyon-v2-then-v3/task-3-written-spec-planning-handoff-ruling.json:2-26]
- **Current package identity:** all eight live sources match their immutable IA2 completion-lane freeze2 copies and expected hashes; all eight differ from the earlier `v3-current-release-source-freeze` that underlies the reviewed CAT/GEN/HIST/FORK/RUN/CHILD-RUN families. Both frozen copy sets still match their own manifests. This is measured identity evidence, not a broad rereview. [2026-10-09-zig-v3-corpus-inventory.json, § source_freeze_comparisons]
- **Current written-review limit:** IA2 FIX1 has scoped approval; untouched package was not broadly rereviewed. Initialization I01–I10 still prescribe a coherent eight-source amendment, new freeze and new full corpus. The inspected current foundation README explicitly says richer initial contracts remain unadopted at schema4088–4143. Thus recorded approval does not imply those package/corpus operations occurred. [.superpowers/sdd/2026-10-01-sandbox-completion-and-expansion/v3-ia2-completion-lane-fix1-review.md:3-15; docs/superpowers/specs/2026-10-03-living-v3-initialization-design.md:469-484,514-520; docs/superpowers/specs/fixtures/living-v3-complete-proposed/README.md:54-59]

All following semantics are written source contracts with recorded rules approval above; none is asserted as Current product behavior. Detailed initialization tables remain in their separately reviewed design, with coherent amendment/vector evidence missing. [docs/superpowers/specs/2026-10-03-living-v3-initialization-design.md:14,469-484,514-520]

## Exact contract constraints to preserve

**Domain and identity:** V3 owns separate authority, identifiers, commands, codecs, archives, Sessions and storage; V2 opens through its matching engine, with no conversion/default filling. Envelopes are PACK/GENESIS/ARCHIVE/NOTEBOOK/TRANSFER kinds suffixed `_V3`, format/rules3. Coordinator is `NYON_SANDBOX_SELECTION_V3`, authority `living_v3`; present corrupt/unsupported newer selection preserves the resident and reports an error. Storage is separately namespaced with generation CAS and exact catalog binding. [docs/superpowers/specs/2026-10-01-living-expansion-architecture.md:22-44,151-174; docs/superpowers/specs/2026-10-01-living-expansion-wire.md:28-32; docs/superpowers/specs/2026-10-01-living-v3-complete-schema.md:4011-4017]

Canonical required fields include explicit null/false/empty, declaration order, sorted typed keys, u64/Tick decimal strings, no Unicode normalization. IDs are16 bytes; Digest/Seed/RevisionID32. Domains are literal ASCII `NYON_LIVING_V3_` plus PACK, STATE, GENESIS, ROOT_BRANCH, REVISION, RECEIPT, EVENT, ARCHIVE, NOTEBOOK, GENESIS_ENTITY, CREATOR_ENTITY, AUTO_ENTITY, FORK_BRANCH, OBSERVATION, TRANSFER, COORDINATOR or CLAIM_RANK, each ending in exactly one NUL. Producer-specific THREAT_OBSERVATION, TREATY_TERMS and CANDIDATE_AUDIT domains have their own clauses; stellar selection deliberately uses a different non-NUL formula. [docs/superpowers/specs/2026-10-01-living-v3-complete-schema.md:10-34,3138-3143,3293-3297,3742-3789,4860-4863; docs/superpowers/specs/2026-10-01-living-expansion-wire.md:180-216]

Exact framing locators, using H=SHA256, D=NUL domain, L=u32le length+bytes, OptRevision=0 or1+32 bytes:

| Identity | Binding |
| --- | --- |
| Genesis entity | `H(D(GENESIS_ENTITY) || catalog32 || seed32 || L(generator_id) || generator_version:u32le || 3:u8 || kind:u16le || ordinal:u64le)`, ID first16; positive unique per-kind ordinal, sorted roster, Generator provenance only, never run during decode |
| Creator entity | root16, branch16, flag2, accepted sequence u64le, operation index u32le, kind/local u16le; excludes revision digest/command bytes |
| Auto entity | root/branch, namespace flag, actor16, tick u64le, phase/kind u16le, successful allocation ordinal u32le, subject kind/id, permitted snapshot digest32; failed allocations consume no ordinal |
| Revision | root/branch, OptRevision(parent), tick, ordinal, mode paused0/running1, L(capture), L(command); full32 ID |
| Fork | root/parent branch, OptRevision(parent revision), fork tick, original parent state digest32, global fork ordinal; child adjusts only branch_sequence, no invented fork Receipt/Anchor |
| STATE / ROOT_BRANCH | STATE hashes complete `{format_version:3,rules_version:3,catalog_hash,state}` wrapper; root hashes catalog32, seed32, Genesis digest32, length-framed Generator and version; no Genesis/root cycle |
| Receipt / Event | Receipt hashes ordered tick/applied revisions/EventPayloads/state digest without EventIDs; EventID derives afterwards from receipt digest/tick/phase/ordinal; no current-receipt self witness |

These rows are source formulas, not newly derived codec vectors. [docs/superpowers/specs/2026-10-01-living-v3-complete-schema.md:3713-3725,3751-3827,3878-3884; docs/superpowers/specs/2026-10-01-living-expansion-wire.md:198-214]

**Direct Genesis:** only tick0, branch_sequence0, next_sequence0, counters{}; explicit authored starting conservation account, no ordinary Receipt/Event/Anchor/Decision/CandidateAudit, hidden phase execution, payments, negotiation or outcomes. Allocation correspondence is exact, with nested Hull11; Shipment13 has no direct source. Endowed facilities/jobs carry paid_alloy0 through completion/cancel/scrap; research grants satisfy transitive closure and cost/membership equality, with no lab debit/completion Event. Damaged assets start repair deadline10. [docs/superpowers/specs/2026-10-03-living-v3-initialization-design.md:49-55,134-138,160-201]

**Repair clarification controls the ambiguous sentence:** ineligible due asset preserves deadline and emits/spends nothing; eligible due attempt, including funding/hold denial, advances from previous deadline+10; no catch-up loop. Missing transition fixture must show ineligible tick10, later eligibility with unchanged already-due clock, first actual attempt/funding result/next deadline. [docs/superpowers/specs/2026-10-08-living-v3-repair-eligibility-clarification.md:3-11]

**Claims/occupation:** direct claim is authored eligible subset at arrival0/close50, exact original Genesis membership/cohort source; ordinary50 performs rank, Ark consumption and award once. Occupation starts0 and requires exact enemy/War/defender/third-party geometry; ordinary1 may increment1 and capture requires100 subsequent qualifying ticks. No initializer combat, winner, award or ownership transfer. [docs/superpowers/specs/2026-10-03-living-v3-initialization-design.md:308-316]

**Observation and Cut0:** independently recompute complete permitted direct observations, including empty authorized scope assertions. Build one private nonserialized Actor32 cut from validated base: offers/demands/DefenseRequests/shares empty, shipments empty, Defense companion request null/statusready/reasons[], other source fields exact. Hash OBSERVATION+canonical32 without branch/root/state/receipt/other actor digest. Source winners precede newest event identity selection, then expiry, then caps; final wave outputs cannot feed the cut. Compare all submitted fields/rosters; never fill omissions. [docs/superpowers/specs/2026-10-03-living-v3-initialization-design.md:320-356,362-380,404-419,433-435]

**Authenticated post-root history:** validate/seal Genesis first; paused creator revisions at0 and advancing ordinary history must use actual producers. Creator evidence never becomes actor-paid provenance. Travelling, true return credit, nonnull passenger/Supply origins, RecipeFacts/Needs, historical relation reasons, terminal objectives and Peace assert prior history; a State wrapper is not an import endpoint. Original Genesis exemptions require exact original typed source and uninterrupted lineage; same-key/equal-byte replacement does not restore them. Document accepted/fork high waters differ from historical State counters and never recycle accepted abandoned marks. [docs/superpowers/specs/2026-10-03-living-v3-initialization-design.md:51-55,220-239,437; docs/superpowers/specs/2026-10-01-living-v3-complete-schema.md:3811-3827]

## Corpus and review availability

**Current foundation is present but incomplete.** It contains explicit B0–B7 Catalog/Genesis/State-wrapper inputs, framing/visibility/structural/rejection controls and helper/source pins. All143 corpus files currently match its stopped FIX1 package manifest. Current Catalog bytes4269 hash to PACK `52a64c125d4be640b6887c014fa620b390764a870ecbb8e52c8d7015c69521bd`; all18 issuance frames are explicitly framing-only. This report did not rederive PACK or run verification. [docs/superpowers/specs/fixtures/living-v3-complete-proposed/README.md:15-41; 2026-10-09-zig-v3-corpus-inventory.json, § corpus_files; .superpowers/sdd/2026-10-01-sandbox-completion-and-expansion/v3-completion-lane-foundation-fix1-package-manifest.json, § files]

Provenance is inspectable: stdlib helpers copy and adapt historical Catalog/Genesis helper code, rather than consume Rust output or old expected digests. Builder/parser share canonical encoding, so agreement is self-consistency. The verifier is designed to check source pins/protected files and compare complete regenerated bytes; helper changes are pinned. It has no named release-version field established here beyond immutable manifest/helper hashes and V3 format/rules3. [docs/superpowers/specs/fixtures/living-v3-complete-proposed/provenance.md:3-29,38-44; README.md:15-19; provenance.py:14-29]

Historical CAT-C finite Catalog, GEN-C selected B0–B7, empty-root paused/ordinary HIST-C, retained-parent FORK, RUN and CHILD-RUN have scoped independent formula/literal reviews. They supply useful preserved evidence, not current source admission: each family's reviewed release predates current eight-source bytes; selected empty histories do not cover inhabited gameplay/Actor32/forecast/storage. Reviews explicitly distinguish independent reconstruction from author verify and scope limits. [.superpowers/sdd/2026-10-01-sandbox-completion-and-expansion/v3-current-catalog-review.md:3-9,79-81; v3-current-genesis-review.md:3-9,58; v3-current-history-rereview.md:3-6; v3-current-fork-history-review.md:3-9; v3-current-running-history-review.md:3-14,48; v3-current-child-running-history-review.md:3-14,48; 2026-10-09-zig-v3-corpus-inventory.json, § source_freeze_comparisons]

No whole current independently reviewed FULL byte corpus is established by the inspected materials. The foundation explicitly lists pending genuine boundaries1–49/T50, Actor32, 200-boundary forecasts/admission, economy/ecology/research/conservation, diplomacy/War/paid Supply/Defense/Peace, creator/history/forks/all18 issued kinds and all portable union/annex cases. Node/line counts are not semantic coverage. [docs/superpowers/specs/fixtures/living-v3-complete-proposed/README.md:43-59; full-package-coverage.json, §§ required_later_packages,remaining_foundation_gaps]

| Required variant surface | Exact missing gate |
| --- | --- |
| F2 nested needs_reserved | Integrate local/account/source checks at every nested BlockedReason/DecisionReason/RecipeFacts; full nested positives/negatives, five genuine guard producer/cardinality/outcome rules, pre-recipe holds, denied atomicity, Farm waiver, privacy |
| F5 general validator | Generic semantics separated from bootstrap conditions; every union/key/range/cap/reference/frame/allocator correspondence; source-complete semantic decomposition and exact decided multi-fault diagnostics; independent literals |
| R1 original references | Distinct original typed issuance/observation/formation provenance for removed paid Route/Commitment, terminal objective/Secure endpoints and environmental children; strict current membership for live/new positions. The live-only resolver defect remains |
| Actor32/scopes/200 forecast | Authentic boundaries1–49/T50 phase7 source, all32 fields, hidden/expired/negative/order/duplication/refresh/eviction cases, immutable candidate inputs, conditional baseline/candidate forecasts/ranking/reservations/admission; T1 projection is not scheduled planning |
| Allocator/fork/creator | All18 genuine issuance histories and collision/consumption rules; pending/running/abandoned marks, lineage/cascade/conservation, same-key replacement controls; all creator and registry variants |
| Archive/Notebook/Anchor | Populated connected history, all experiment/metric/evidence unions, exact original anchors and selected ancestry, actual replay; historic empty examples are bounded |
| Retention/transport/coordinator | Cache eviction preserving anchors/replay, bounded retirement, continuations/cancel/stale, streamed byte caps, exact catalog/generation CAS/native/wasm consumer agreement; no trusted checkpoint or fallback |
| Initialization/rejections | Every direct family plus first real transitions, Cut0/THREAT, endowed job/passenger/account sources, repair clarification, all registry/diagnostic/boundary/negative controls from initialization §13, with reachable-cap limits explicit |

F2/F5/R1 remain ChangesRequested/Partial in scoped foundation review; corrected semantic draft is pending-proof input, not repair or qualification. The last queued FIX2 brief is NOT RELEASED and requires original-author correction plus rereview. [v3-completion-lane-foundation-fix1-rereview.md:3,20-41; v3-foundation-semantic-ledger-draft-fix1-rereview.md:3,28-32; v3-completion-lane-foundation-fix2-queued-brief.md:3-15, all under .superpowers/sdd/2026-10-01-sandbox-completion-and-expansion/]

Remaining rows trace to full-package required-later packages and initialization all-family matrix; scope/forecast timing specifically to Actor preflight. Retention/transport constants and preservation rules are schema4145–4195. [docs/superpowers/specs/fixtures/living-v3-complete-proposed/full-package-coverage.json, § required_later_packages; docs/superpowers/specs/2026-10-03-living-v3-initialization-design.md:524-565; .superpowers/sdd/2026-10-01-sandbox-completion-and-expansion/v3-current-actor-admission-preflight.md:7-9,38,89-95; docs/superpowers/specs/2026-10-01-living-v3-complete-schema.md:4145-4195]

## Exact identities observed

Complete raw-file hashes are in the companion JSON; selected current entries below. These are SHA256 of file bytes, distinct from framed PACK/STATE/ROOT digests. [2026-10-09-zig-v3-corpus-inventory.json, §§ selected_evidence,corpus_files]

| File / artifact | Bytes | SHA256 |
| --- | ---: | --- |
| IA2 freeze2 manifest |2904|`55ca0ede26a284142f6d1208fa57ebe83ecd6bcce59a99751751332d42d6d1eb`|
| Foundation FIX1 package manifest |48022|`12775b726d46d21c3e5dd4717565e9c0522b64d37355cde83f50fe8a3de34418`|
| Initialization design |93676|`d305e685b58f37a3fadfb31325de48513a9e4b0fcdcb8a7faabe833e67c9b147`|
| Repair clarification |1894|`c8d99b8ada1daeb38c712d71215884bddd64dedc5fdca3ad54843332e498e56e`|
| foundation catalog/vectors.json |6048994|`c84ef335f942bb160a01388a274671b4f626e7559c723022628c1cd584de08cc`|
| foundation genesis-cases.json |12496337|`31f6738be1421310b5ed1a2943155553bf68a2423f03260e5af8491649ea76c0`|
| foundation initial-state-and-root-identities.json |37938710|`fa915513e986e5d1883615222170f4437940cf73414868593992e1d07f1ecf18`|
| foundation helper_hashes.json |1276|`a28cd51fbb4f9f2e0cff032612d4145addd8ffaa4a1ca7d9df8c005b935a202d`|
| pending-proof semantic draft |16287127|`e3aae41e201d062d0c1777084d9e004f5a1a49dbc3b5193d10a06478da369321`|
| companion inventory JSON |60897|`1358220137676244af085861994e32ab459d4886fb1b18ac30b627eb5d3cdaad`|

## Prerequisite sequence for a separately reviewed Zig V3 island

1. Reconcile the approved rules record, unchanged eight-source freeze, reviewed initialization I01–I10 and repair clarification into one coherent explicit effective package; preserve previous freezes. Resolve/review that package and freeze exact source/derivation/review identities. [initialization design:469-484,514-520; completion ledger:127-132]
2. Correct F2/F5/R1 and qualify the complete foundation against that package. Build a semantic variant matrix with per-case strength/source/provenance, all registry variants and independently formed rejection inputs. [foundation FIX1 rereview:20-41; FIX2 queued brief:9-15]
3. Independently derive complete direct and genuine post-root bytes, not fabricated paid State; cover Actor32/scopes/200 forecasts, allocator/creator/fork, mechanics/conservation and populated portable/evidence/retention/transport cases. Independently review derivation and literals; close omissions; freeze immutable FULL corpus. [Complete V2 then V3 plan:64-68; initialization design:524-565]
4. Finalize and independently review the Zig-specific island implementation plan while preserving version/domain/no-conversion and bounded owner/commit/retirement contracts. Only then implement and review codec/authority against independent corpus; a Python verifier is neither Zig codec proof nor shared native/wasm proof. [Complete V2 then V3 plan:74-77; migration plan:214-224; architecture:22-54; initialization design:565]
5. Integrate real Sessions, actions/projections, isolated durable storage/coordinator, portable/player flows and separate native/browser/storage/accessibility/endurance/performance qualification. Strategic architecture approval and written-spec/plan review remain a separate dependency, followed by actual strategic host implementation. [Complete V2 then V3 plan:75-77; migration plan:217-224; completion ledger:282-290]

```text
recorded rules approval + reviewed initialization/repair
  -> coherent effective eight-source amendment/review/freeze
  -> F2/F5/R1 foundation + exhaustive semantic variant matrix
  -> independent FULL direct/history/Actor32/forecast/portable corpus
  -> independent derivation/literal review + immutable corpus/package freeze
  -> Zig V3 implementation-plan review -> island implementation/codec proof
  -> real host/storage/player integration + separate platform qualification
strategic architecture approval -> written-spec/plan review -> strategic host delivery
Both chains and the retained acceptance rows are required for Task 7 completion.
```

Gaps: no broad end-to-end read/rereview of all eight long specifications, independent reconstruction of all literals, generator execution, codec/product inspection or live qualification occurred in this source-only inventory. Approval traceability is from the supplied ledger; exact current source/corpus identities were measured. No strategic cube-world design, V3 keys, initialization defaults or paid receipts were invented.
