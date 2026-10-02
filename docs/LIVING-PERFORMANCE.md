# Living performance qualification

Status: Partial. Timing requirements are authoritative in the accepted Living
qualification contract. Passing tests and counted private cursor visits do not
establish native/browser frame time, input latency or full-capacity performance.

The new `examples/living_authority_benchmark.rs` measures the current resident
Step action: `SandboxSession::step_once`, including the actual authority boundary,
receipt/decision retention and Chronicle bookkeeping. It neither renders a frame
nor emulates a different executor. It reports nearest-rank p50/p95/max, actual
successful sample count, emitted event count, initial/final bookmarks, catalog,
seed and object counts. A fault reports the last valid bookmark and exits nonzero.
Warmup runs are excluded from sample statistics. The first ten measured steps
print an estimate before the remainder; later changing simulation load can make
that estimate inaccurate.

```sh
cargo test --example living_authority_benchmark
cargo clippy -p nyon --example living_authority_benchmark -- -D warnings
cargo run --release --example living_authority_benchmark -- --fixture vale --steps 100 --warmup 10 --host 'Apple M5 / macOS 27.2 (26B5091g)'
```

Fixtures are `vale`, `expansion`, reproducible `seeded`, and `topology`. The last
is a validated 64-system/512-world/16-dormant-civilization genesis. Its ordinary
resident step is tested; it has no industry, freight or combat. It is explicitly
**not** the declared mixed full-capacity fixture and cannot qualify the 5 ms
full-capacity target. The default is 3,600 samples with ten warmup steps; CLI
limits are 1–100,000 samples and 0–10,000 warmup steps. Run a short pilot before
requesting large counts. No 36,000/100,000 run is implied by supporting those
arguments.

Existing `living_step_profile.rs` measures 10,000 authority-only boundaries on
two small fixtures. Existing `living_endurance.rs` runs paired zero/one/many
fixtures (36,000 default), compares receipts/decisions and periodically validates
state. Their source does not prove they were executed at the latest checkpoint,
and neither includes the declared full-capacity fixture.

Remaining qualification:

- Validated mixed active full-capacity fixture: 64 systems, 512 worlds, up to
  16 civilizations, 2,048 facilities/routes, 4,096 shipments, 256 fleets and
  2,048 total hulls within per-colony/catalog constraints.
- Named-device p95 authority boundary ≤5 ms at declared caps. Resident Step
  measurements include presentation bookkeeping and are labelled accordingly.
- Native/WebGPU frame p95 ≤16.7 ms, WebGL2 Low ≤33.3 ms at 1920×1080, and input
  response p95 ≤100 ms.
- Actual 36,000 mixed and 100,000 endurance runs with invariant/reproducibility
  checks and bounded retained history/event memory.
- Requested versus effective transport speed, offline complete journey, and
  separate browser/device/platform evidence.

## Executed measurements

Two short release pilots completed with actual exit 0 on Apple M5, macOS 27.2
build 26B5091g, arm64 (host identity read directly). These are sparse pilot
measurements, not full qualification.

Focused checks completed with actual exit 0:

- `cargo test --example living_authority_benchmark`: 2 passed, zero failed or
  ignored (`/tmp/nyon-living-benchmark-tests3.log`). The topology fixture validates
  and runs the real resident Step, then checks receipt/state digest agreement;
  nearest-rank percentile tests cover one and small sample sets.
- `cargo clippy -p nyon --example living_authority_benchmark -- -D warnings`:
  actual exit 0 (`/tmp/nyon-living-benchmark-clippy2.log`).
- `rustfmt --edition 2024 --check examples/living_authority_benchmark.rs`:
  actual exit 0.

The first compile rejected a missing genesis kind; the fixture now uses the
normative `LIVING_GENESIS_KIND_V2`. The measured release pilots below supersede the initial pending compilation
status; focused checks themselves do not supply timing results.


### Release pilots, 2026-10-01

Executable SHA-256:
`300e03829f0a9130e1b291714b5278deb8c2c14b6a461c916c224e2f75033c14`.
Example source SHA-256:
`8273ad32bc4dd987bc120b6d6804aef97ae9790cb1080b78b1a6d42cef7d4dd5`.
Catalog hash:
`2b391f0585ee573c90c4f2346b108a7d3b23d489e06cc7e732d1e43b93ea7c42`.
The executable is the exact measured artifact; the shared checkout was active,
so these measurements must not be assigned to another later build.

| Fixture | Warmup / measured steps | Final tick | p50 | p95 | Maximum | Measured elapsed | Events |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Vale Confluence | 10 / 100 | 110 | 0.129083 ms | 0.481083 ms | 11.436375 ms | 36.150 ms | 163 |
| Topology only | 90 / 10 | 100 | 0.750250 ms | 6.747958 ms | 6.747958 ms | 16.210916 ms | 0 |

Vale command is the release command above; actual exit0, log
`/tmp/nyon-living-benchmark-vale-pilot.log`. It measured ticks11–110 and retained
3 civilizations,4 systems,8 worlds,23 facilities; fleets changed6→12, routes0→1,
shipments0→1. Final state digest:
`e8f59605e98a8318f68fcf88f83f844a17ab8a5ef359833eace452d7b3e0c6aa`.

Topology command:

```sh
target/release/examples/living_authority_benchmark --fixture topology --steps 10 --warmup 90 --host 'Apple M5 / macOS 27.2 (26B5091g)'
```

Actual exit0, log`/tmp/nyon-living-benchmark-topology-pilot.log`. Ticks91–100
include a periodic boundary, with64 systems,512 worlds,16 dormant civilizations
and no facilities/routes/shipments/fleets/hulls. Final digest:
`c8fded84a0288aa0f3a4c949ac9ace56e4eed558bfffe60d4f16fcf6fa3e542c`.
Its ten-sample nearest-rank p95 is the maximum; the6.748ms result is above5ms
even on this limited topology, but it does not establish a declared-capacity
authority-only failure because the path includes Session bookkeeping and the
fixture lacks the full active workload. Vale's11.436ms maximum also shows that
its low p95 does not bound every boundary.

Straight-line pilot projections are about36.15s per100000 measured Vale steps
and162.11s per100000 topology steps, excluding startup/warmup. These estimates
can change as societies evolve or periodic work distribution changes. No
36000/100000 run or full-capacity run was started. Frame time, input latency,
wasm/device parity, offline journey and full LG20 acceptance remain Partial.


## Declared-capacity harness extension (focused automated verification Current; timing Partial)

`--fixture capacity` initializes synchronized freight due boundaries at tick 1; `--fixture capacity-staggered` uses world-index freight offsets 1 through 10. Hub/recipe clocks retain mandatory genesis periods (10/20/100); they are not arbitrarily staggered. Both initialize 64 systems, 512 owned worlds/colonies, 16 active civilizations, 2048 facilities, 2048 Internal routes, 4096 outbound shipments, 256 fleets and 2048 escort hulls, plus 512 ore deposits. Each colony has solar array/extractor/foundry/shipyard, within six slots and one shipyard; routes have distinct same-system endpoints, batch 10, cadence 10 and reserve 20. Fleet hulls have valid HP and no invented fuel credits. Shipments carry 10 ore units with actual positive arrival deadlines. State and embedded genesis pass normal validation before execution; the focused regression requires both variants to step and match receipt/state digest.

Use `--authority-only` to time `LivingGalaxyAuthorityV2::step` directly. Without it, the harness times the current resident Session path including Chronicle bookkeeping. These paths are labeled separately in output; neither measures rendering or input. No claim of persistent maximum occupancy follows from initial counts: cargo resolves, recipes consume inventory, and autonomous choices can change the population. Pilot 10 steps with `--warmup 0` so the initial synchronized burst remains in samples. Run neither 36000 nor 100000 capacity steps until a measured pilot runtime estimate is recorded. Full mixed/endurance reproducibility, memory and platform qualification remain Partial.

Focused example tests finished with actual exit 0: 3 passed, zero failed/ignored (`/tmp/nyon-capacity-benchmark-tests2.log`). Both capacity variants validated and produced identical first receipts on direct authority and resident Session paths. Scoped strict example Clippy actual exit 0 (`/tmp/nyon-capacity-benchmark-clippy2.log`); standalone example formatting check exit 0. The initial capacity attempt failed genesis Clock validation because it incorrectly staggered mandatory hub/recipe clocks; that fixture error was corrected before this green checkpoint. Release build actual exit 0 (session 93902, 18m40s, `/tmp/nyon-capacity-benchmark-release.log`). Automated correctness evidence is not timing acceptance.

### Capacity pilot: contended host, diagnostic only

2026-10-01, Apple M5 / macOS 27.2 (26B5091g), release artifact SHA256 `f90bffa0d027c9b22eca955c2fc02124170735d0a3554514d9c7b71b75dfa537`, example source SHA256 `05e447442e6107416906c9b0a56f09ff988df6b6f730b073f5e9c6d5235f1afc`. Several rustc processes remained active (observed about 43–58% CPU each immediately before pilots); these results are contended diagnostics. All used `--steps 10 --warmup 0` and completed with actual exit 0, no fault, 7168 events, final tick 10. Ten-sample nearest-rank p95 equals maximum.

| Fixture/path | p50 ms | p95/max ms | Total ms | Log |
|---|---:|---:|---:|---|
| synchronized / authority | 41.653 | 161.345 | 530.033 | `/tmp/nyon-capacity-authority-pilot.log` |
| staggered / authority | 38.570 | 49.989 | 398.159 | `/tmp/nyon-capacity-staggered-authority-pilot.log` |
| synchronized / resident | 90.984 | 370.473 | 1052.086 | `/tmp/nyon-capacity-resident-pilot.log` |
| staggered / resident | 154.078 | 334.995 | 1697.630 | `/tmp/nyon-capacity-staggered-resident-pilot.log` |

The paired paths reached identical final authority digests: synchronized `b10df09357e133da09846674e655c5847fd5e93a47d3d49b73013261b2b1fd06`; staggered `6ee78ced33fa32e8dbb44e81ef2456150d73e34c9d786e7a032f3609896bd045`. All non-shipment declared counts remained at initial caps; shipments decreased from 4096 to 2048 after actual deliveries/dispatches. Observed authority p95 exceeds the 5ms target; contention prevents a clean-host attribution of the exact cost, and these samples do not establish accepted performance. Resident measurements include verified Chronicle projection, which is separately expensive.

Straight-line 100000-step projections from these short pilots are approximately 1.47h/1.11h authority and 2.92h/4.72h resident for synchronized/staggered respectively. These estimates exclude future workload changes and are not runtime guarantees. No 36000/100000 capacity run was started. Quiet-host repeat and representative later decision/recipe boundaries remain required.

### Quiet-start capacity repeat

Same artifact, host and four 10-step/zero-warmup paths, each actual exit 0. No rustc or workshop capacity-test process was observed immediately before the four sequential pilots; rustc PID38492 was observed immediately afterward at about85% CPU. Thus this is a quiet-start repeat, with no guarantee of full-host quiescence throughout. Results retain identical events, counts and final digests to the contended pilots.

| Fixture/path | p50 ms | p95/max ms | Total ms | Log |
|---|---:|---:|---:|---|
| synchronized / authority | 21.983 | 77.730 | 291.032 | `/tmp/nyon-capacity-authority-quiet-pilot.log` |
| staggered / authority | 27.513 | 42.759 | 302.468 | `/tmp/nyon-capacity-staggered-authority-quiet-pilot.log` |
| synchronized / resident | 25.424 | 108.625 | 362.002 | `/tmp/nyon-capacity-resident-quiet-pilot.log` |
| staggered / resident | 30.368 | 85.795 | 411.826 | `/tmp/nyon-capacity-staggered-resident-quiet-pilot.log` |

This repeat still exceeds the 5ms authority p95 target. Its 100000-step straight-line estimates are approximately49/50min authority and60/69min resident, excluding later changes. No endurance run was started. Full-capacity performance remains Partial; successful bounded execution and paired digest parity do not establish performance acceptance.

### Bounded call-stack diagnostics (proposal evidence)

Used existing macOS `/usr/bin/sample`, one-second duration with one-millisecond interval, against the same release artifact. No dependency or production source changed. Sampling commands completed with actual exit 0. Each sampled benchmark requested 100 steps but was intentionally terminated after a bounded wait (authority/resident initial samples: 12 seconds after sampling; delayed intent sample: 4 seconds after sampling), process exit -15. These are not completed 100-step runs and supply no endurance timing. Concurrent compilation remained possible.

Initial authority sample `/tmp/nyon-capacity-authority.sample.txt` captured 761 main-thread stacks. Collapsed leaf counts include canonical structure scanner103, colony/industry validation87, observation poll77, SHA25663, JSON string serialization63, combat60 and freight48. Inclusive source-state digest frame122, candidate canonical-state frame115 and event-facts sealing frame104 demonstrate substantial validation/serialization/sealing work, without proving exclusive wall-time percentages. Resident sample `/tmp/nyon-capacity-resident.sample.txt` captured a `record_chronicle` subtree41; event-facts revalidation occupies15 of those stacks, alongside aggregation searches, clones and text formatting. These are window-specific counts, not estimates of all later game phases.

Delayed authority sample `/tmp/nyon-capacity-intent.sample.txt` began three seconds after launch. All756 captured stacks were under `generate_and_reserve_intents_with_disclosure`. Collapsed leaf counts: `reserve_delta<LivingFacilityV2>`321 and `reserve_delta<LivingFreightRouteV2>`290, together611/756 (approximately81% of this sampled window), with allocator/free/clone work following. Source `rules.rs::reserve_delta` repeatedly performs linear `find` and `any` searches across sorted unique snapshot/proposal/reserved collections for every row. This supplies a concrete priority ahead of assuming forecast dominance; the window did not isolate forecast as the bottleneck.

Recommended next slice (Proposed): replace the repeated membership lookups with keyed binary search or a stable merge walk, retaining exact changed-row application order, unchanged-row comparisons, first conflict behavior, deletion order, errors and caller atomicity. Regression axes should include unchanged maximum collections, inserts/updates/deletes, stale reserved edits, simultaneous resource/identity reservations, rejected proposals and frozen receipt/decision parity. Then remeasure decision-boundary samples on the same validated fixture and record the new artifact hash.

Other measured candidates remain separately Proposed: preindex per-world industrial/job counts during validation while preserving error order; avoid duplicate full-state serialization when an authority-owned exact-boundary digest is already available with complete invalidation coverage; reduce repeated whole-sidecar encoding during exact capacity admission; and avoid redundant producer-facts cloning/reverification in the trusted Session path without weakening the public untrusted Chronicle verifier. None is implemented by this profiling review. Snapshot footprints reported by sample (authority29.4MiB, resident19MiB/peak34.1MiB) do not establish bounded endurance memory.
