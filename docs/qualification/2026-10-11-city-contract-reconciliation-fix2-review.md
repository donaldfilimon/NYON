## Design Document Review: Rich-city narrow codec fix2

### Summary

**Approve the two city-side clarifications in the exact R revision below.** Both major findings from the [fix1 review](2026-10-11-city-contract-reconciliation-fix1-review.md)—canonical capital defaults and accepted-command custody mappings—are resolved; no directly connected regression was established. **Integrated composition approval and implementation authorization remain open**, including the mandatory consumer repin described below.

Scope: only those two findings and their direct byte/custody consequences. This is not a renewed broad mechanics, tactical, authority, persistence or performance review. Existing reports and all specifications/source/ledgers/instructions were preserved. This task created only this review file and ran no builds or executable product gates.

### Exact reviewed revisions

Pinned at **2026-10-11 13:39:32 UTC** and rechecked at **13:40:37 UTC**; all three were unchanged during this review:

| Document | SHA256 |
| --- | --- |
| [R: rich city](../superpowers/specs/2026-10-11-nyon-zig-rich-city-design.md) | `c9443864b336bf415b6dc38537118c14438343f9d4ebb0a820025adb5b1a4709` |
| [E: city integration annex](../superpowers/specs/2026-10-11-nyon-zig-expanded-city-economy-design.md) | `2647a9e43022e37ce0343362684617c295aaa210d48dd0be7ae1d819c6ae829a` |
| [T: tactical](../superpowers/specs/2026-10-11-nyon-zig-tactical-ground-combat-design.md) | `41916c11f31c50b62e012fa162530c4a9629501e49e2f7e3e24f60c72195e08a` |

The R fix2 additions were read against the previously reviewed R fix1 grammar, with narrow E/T reads for their exact consumer pins and adopted contracts. The previous source inspection establishes the unchanged old-v0/new-city boundary; no new implementation or source-gate evidence is claimed here.

### Resolved finding 1: Canonical capital and creation defaults

R lines553-611 now fixes enabled and priority bytes for all14 initial buildings, including passive roads/residence and solar. Every building is enabled; Waterworks9 and Farm10 have priority0; all others priority1. Work, paid-cost, integrity and activation defaults are explicit. Alternate enabled/priority values must fail recomputed-genesis comparison even when behavior is unchanged.

The same amendment specifies capital, empty, author_city, found_city, resettle_city, paid-build and author-building defaults without borrowing implicit host initialization. Resettlement preserves owner, hall integrity, occupation, policy toggles and unrelated goods/reservations while resetting the stated population/health/streak fields. No hidden repair or ownership reset was introduced.

**Disposition: resolved for the pinned R city contract.** Full genesis/placement and compiled decoder qualification remain separate existing obligations.

### Resolved finding 2: Accepted-command stock and custody mapping

R lines987-1052 now maps tags0..17 explicitly to created/related IDs, command-time stock debits/credits, population movement and waivers. In particular:

- Ship and transferred author-ship debit source stock with zero destination stock credit; shipment owns cargo and destination owns only a reservation.
- Minted author-ship has zero stock debit/credit; its authenticated tag11 input identifies the explicit cargo mint. Arrival changes stock/cargo custody without another mint.
- Founding/resettlement separate supplied goods transferred to the destination from consumed construction/expedition materials through the exact debit-minus-credit vectors.
- Recruitment, cancellation, repair, waived authoring and zero-effect policy commands have explicit ID/vector meanings; retired IDs never occupy created_local.

The narrow fixtures distinguish command boundary from household phases. They do not miscount incoming reservations as owned goods.

**Disposition: resolved for the pinned R city contract.** Tactical-family accepted mappings remain owned by its separately reviewed composition; this result does not approve unspecified tactical fields.

### Independent literal verification

A disposable Python standard-library calculation rebuilt bytes directly from the displayed scalar grammar, capital table, stated cells and effect mappings. It imported no game code or production serializer. Assertions and SHA256 comparisons exited **0**.

| Independent reconstruction | Verified result |
| --- | --- |
| All14 canonical building records, realm1/city1 |849 bytes; SHA256 `d4587faaf1900d6eeccb988b2d17d22946b00290b0aa948c5256e8e7ef40ee11` matches R1061-1064. |
| Solar13 canonical record |60 bytes; exact printed hex at R1068 matches reconstructed key/length and payload. |
| Enabled/priority sensitivity |Independently flipped each of the28 field bytes; every altered vector's SHA256 differs from the fixed849-byte vector. This checks byte sensitivity, not an implemented decoder's rejection behavior. |
| Transfer accepted frame |150 bytes, length prefix146, header19, payload127; exact hex and SHA256 `f3f1f3ce748c235e63fc0d40261de274d78e52a14166fcdf00a7e395afd5e4e3` match. |
| Minted-cargo accepted frame |150 bytes, length prefix146; exact hex and SHA256 `2fcd17eccf0dc22541a667629ae43a0b9ab31860199f2de346e58f30f25286b7` match. The corrected final literal is150 bytes, not153. |
| Founding accepted frame |150 bytes, length prefix146; exact hex and SHA256 `9bb4c993121736656ebece2ef3c292a0e821f93067905fa6a3350c44f62a39c9` match. |
| Transfer custody |Source96 +destination20 +cargo4 =120 at departure; source96 +destination24 =120 after arrival. |
| Mint custody |Source100 +destination20 +cargo4 =124 at admission; source100 +destination24 =124 after arrival; exactly one four-unit mint. |
| Founding custody |Source becomes `[68,68,84,84,10,10,6,10]`, new city receives `[32,32,0,0,0,0,0,0]`; summed-stock sink is exactly16timber/16stone/4tools, and12 residents become8+4. |

The explicit fields remain within the previously reviewed150-byte accepted-frame size and256-byte reservation. No enlargement of the declared12746752-byte combined tick envelope is required by these fixes. This is static byte arithmetic, not a maximum event-emitter or runtime performance test.

### Issue 1: Integrated consumers still pin the intermediate city bytes

- **Severity**: major
- **Section**: E “Composition authority and immutable selected revisions — fix3,” lines287-301; T “Revision Summary — joint fix3,” lines705-718.
- **Description**: Both consumers explicitly pin rich-city snapshot `32c9e391e9f8936b8124a984b8c83f588aa51017ea8e987a0e551bff362c6668`, while this review approves final R `c9443864b336bf415b6dc38537118c14438343f9d4ebb0a820025adb5b1a4709`. E correctly states that subsequent public drift is a separate integration prerequisite. Its adoption of the city defaults, custody table, phase0..21 and crew:u8 does not make the intermediate and corrected final bytes identical. This is an exact-revision dependency, not a reopened general mechanics objection.
- **Suggestion**: E/T owners must adopt a byte-exact snapshot of the final reviewed R revision, update all applicable pins and validate the corrected150-byte minted-cargo literal within the same composed grammar. The controller should pin the resulting E/T revisions and complete their integrated review. Do not silently replace the frozen snapshot, claim an old review covers new bytes, or reuse the initial full-scale economy identity for city fields.
- **Status**: open

### Strengths

- Every capital field now has one canonical value even where the value is behaviorally passive.
- The accepted-effect table separates city stock, cargo, reservations, material sinks and explicit mints without increasing the record size.
- Literal witnesses cover both transfer and mint, plus the mixed consumption/transfer founding command.
- E/T explicitly expose their frozen revision boundary; the remaining repin is concrete and reviewable.

### Approval boundary and verification record

**Current:** the two requested R clarifications pass this scoped independent design review; exact literal comparisons, custody arithmetic and28 byte-mutation checks passed in a standalone standard-library script, exit0. Document hashes remained stable through the stated final check. Direct artifact structure/whitespace validation is recorded with this review's creation.

**Still open:** consumer repin and integrated composition review; previously identified complete tactical/authority/session codec registration, physical layout/ownership proof, full gameplay journey, loaded p95≤5ms,100000-tick retention and native/browser qualification. Those were deliberately not reopened or claimed complete here.

**Not run:** builds, repository executable gates, decoder/gameplay/runtime tests or remote/native/browser durability probes. Static fixture reproduction does not establish that the implementation accepts/rejects those bytes. No source, other documents, dependencies, profiles, commits, pushes or deployments were changed by this review.
