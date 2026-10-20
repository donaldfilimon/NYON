## Design Document Review: NYON Zig Rich City Implementation Plan

### Summary

**Needs revision: one major qualification-workload defect.** Task 17 requires an unreachable simultaneous quota tuple; correct that acceptance condition while preserving every approved quota. Otherwise, the plan adequately exposes its hard Task 0 contract and Task 1 witness prerequisites: this review grants no product implementation readiness or runtime acceptance.

### Scope and pinned evidence

Reviewed all 698 lines of [the implementation plan](../superpowers/plans/2026-10-11-nyon-zig-rich-city.md), SHA256 `b682091cfde795ba8da1ece2bf892430cad563fef64698fd3b59e0585f5f5867`. Compared the relevant mechanics, codec, phase, custody, witness and qualification clauses with [R](../superpowers/specs/2026-10-11-nyon-zig-rich-city-design.md), SHA256 `c9443864b336bf415b6dc38537118c14438343f9d4ebb0a820025adb5b1a4709`. Both pins were checked before and after review. Read the raw reconciliation/fix1/fix2 review evidence and delegated design authority; their original dispositions remain unchanged.

The plan's E/T/A/C pins are its declared planning inputs. This review does not transfer approval to later independently owned E/T/session/strategic amendments or claim their integrated acceptance. Task 0 must select and review the latest exact compatible set. No remote system, provider implementation, session durability, compiler layout or runtime performance was verified here.

Actual source inspection supports the plan's current/proposed distinction:

| Evidence | Verified implication |
| --- | --- |
| `zig/src/surface.zig:4-5,47-86` | Existing cube topology supplies four cardinal directions and bounded seam conversion; it supplies no extra diagonal access edge. |
| `zig/src/play_role.zig:3-25` | Creator/ruler/combined admission and rejection of externally submitted autonomous provenance exist; checked conversion into the new types is appropriate. |
| `zig/src/planet_economy/types.zig:21-66` | Existing v0 has two resources, three building kinds and seven command tags; the rich-city union cannot reuse its byte identity. |
| `zig/src/coordinated_galaxy/types.zig:26-45`, `state.zig:6-14,47-79` | Current coordinator is a four-world island composition, not the proposed sparse full-scale city engine. |
| `zig/src/seeded_galaxy/storage.zig:3-12,30-56` | The 512-world manifest allocation is metadata capacity, not city simulation storage. |
| `zig/src/expanded_session/types.zig:1-65` and bounded source file inventory | Session metadata/catalog/allocation work exists; it is not evidence of the completed city bridge/provider. No `rich_city`, `city_combat`, `fullscale_authority` or tactical implementation files appeared in the inspected `zig/src` inventory. |
| `zig/build.zig:28-54`, `zig/tools/check.sh:1-38` | Isolated planet tests, executable-path CLI checks and the Rust-free migration gate are real patterns. City targets and acceptance remain future work. |

### Issue 1: Task 17 requires a simultaneous gameplay state that placement and freight rules exclude

- **Severity**: major
- **Section**: Task 17, first acceptance bullet, plan line637; related loaded-workload selection in Task 18, lines658-663.
- **Description**: The instruction is to “construct reachable simultaneous distribution of4096cities/16384buildings/4096cargo/36864claims.” This is stronger than independent caps or a physical storage envelope and is impossible under the selected rules. R lines127-134 give one hall claim per city and at most two claims per building; R lines162-175 make farms the only two-cell buildings and require each farm to attach to its own hall/road. Thus `36,864 = 4,096 + 2 × 16,384` forces every one of the 16,384 buildings to be a farm, leaving zero roads. Without roads, every farm needs a distinct cardinal neighbor of its own hall. Four cardinal neighbors per hall provide at most `4 × 4,096 = 16,384` attachment slots, so reaching this farm count consumes every hall-neighbor slot. No two halls can then be adjacent: each hall-to-hall edge would remove two farm attachment slots. Freight routes require distinct endpoint halls and only completed hall/road cells throughout (R lines191-194,334-343; plan Task 10 lines448-453). With no roads and no adjacent halls, there is no legal route for even one live shipment, let alone 4,096. Existing active routes also prevent deliberate dismantling from manufacturing this state (R lines248-251,350-352; plan Tasks 8/10). Tactical training reservations cannot repair the geometry. An implementer following the current acceptance text must either fail permanently, bypass the legal-state rules in a supposedly reachable fixture, or silently change the required workload.
- **Suggestion**: Preserve all quotas and the conservative row/page/byte envelope. Distinguish (1) independently attainable cap/cap+1 gameplay cases, (2) independently derived reachable joint city/building/cargo workloads with their actual claim count, and (3) synthetic allocator/codec envelope stress that is explicitly not a reachable game state. Have the independent witness owner derive and pin the legal simultaneous tuples and action schedules before qualification; do not invent an alternative maximum or change a golden to match the implementation. Bind Task 18's 10,000 full loaded-tick measurement to those accepted legal workloads, retain the full p95≤5ms criterion and separately retain maximum storage/encoding stress. Task 15 already distinguishes syntactic maximum emitters from reachable simultaneous workloads; apply that same distinction consistently here.
- **Status**: open

### Prerequisite and integration assessment

- Task 0 is a genuine stop: lines215-223 require matching final R/E/T pins, explicit tactical kinds/events, hostility authentication, exact new profile registration, concrete storage/provider/query/copy-lifetime/retention/shutdown signatures and independent acceptance. Its unfinished declarations are acknowledged design inputs requiring an amended, rehashed plan, not authority for an implementer to invent a bridge.
- Task 1 is a second genuine stop: lines229-253 require independently generated literal vectors, compiled controls/layout ownership, the complete fixed public journey and per-hour/final hashes before product dispatch. Missing complete-story hashes are correctly not fabricated or reported as passed. Legal maximum-workload construction from Issue 1 should likewise be an independently pinned input to the qualification tasks.
- The task sequence distinguishes pure codec work, isolated phase tests over the accepted engine, completed tactical/public integration, copied queries, fresh authenticated replay, CLI journeys and durable sessions. External executable dependencies remain explicit at Task 0 lines220-221; this review does not count their future acceptance as current implementation.
- The plan preserves phase0..21, all-world tactical round barriers, the shared allowance, pre-encoded one-unit publication, cancellation/retirement ownership, qualified training producers and realm-zero units. It addresses damaged-capacity refunds, retained cargo/reservations, capture pauses, next-hour activation, current-source-owner cancellation and sole receipt emitters without adding another game feature surface.
- Interface/type canonicalization and actual allocation fit must be rechecked against the concrete Task 0/1 artifacts. The separately proposed `RC`/`CC` declarations must share the accepted type identities and budget instance; this review does not certify unwritten adapters or independently declared witness layouts as compiled production evidence.

### Strengths

- The plan ties full-story feasibility, exact canonical capital defaults and independently derived hashes to preimplementation exit gates.
- Stock, cargo, reservations, paid refunds and population crew custody are kept distinct, including minted cargo and capture/cancellation cases.
- Labor, power, clinic supply and the four-person satellite are tested as concrete constrained gameplay rather than inferred from nominal recipes.
- Old identities and user saves remain isolated; errors, lease retention, faulted cleanup and durable selection preservation receive explicit coverage.
- Performance requires complete loaded ticks and separately measured poll latency; a fast partial poll cannot satisfy the full-tick target.
- Ownership is explicit, shared files are serialized, and controller delegation avoids unnecessary human approval loops while retaining independent review.

### Verification boundary

**Current:** complete plan read, focused R/review/authority comparison, actual source inspection, the mathematical reachability argument above and documentation structure/whitespace/pin validation. The documentation validation command exited **0**; scoped `git diff --check` exited **0**. Because this review is newly untracked, direct file validation is the substantive whitespace/structure check.

**Proposed/blocked:** product dispatch remains blocked on the explicitly required Task 0 and Task 1 exit artifacts, in addition to correction of Issue 1. No contract-acceptance report, compiled layout, independent complete-story digest, engine implementation, runtime performance or durability pass is implied.

**Not run:** builds, tests, compiler invocations, executable repository gates, provider/session operations or native/browser checks. Only this review file was created; no source, plan, specification, ledger, instruction, dependency, commit or deployment was changed.
