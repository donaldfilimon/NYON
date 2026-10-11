# Expanded Zig rewrite execution

User-approved direction: complete Zig 0.18 rewrite, no final Rust runtime,
libraries, generators, shader tools or required verification. New save identities;
existing user data is untouched. Creator, ruler and combined roles; strategic
surfaces, cities and tactical combat; desktop and browser delivery.

## Current implementation

- Exact compiler pin: `0.18.0-dev.146+35accc06e`.
- Required `zig/tools/check.sh` no longer invokes the Rust differential oracle.
  Historical comparison scripts are not required rewrite gates.
- `surface.zig` supplies integer cube topology, reversible cross-face adjacency
  and unique cell IDs. Positive/negative X/Y/Z face order is fixed. Each face's
  right/up basis defines orientation; cell ID is face*side²+y*side+x. Side is
  1..65535, invalid coordinates fail before calculation, and IDs use u64.
- `play_role.zig` supplies explicit role/provenance admission. Ruler and combined
  require a civilization; creator commands cannot impersonate an actor, ruler
  commands must match the selected civilization, autonomous commands cannot
  enter through player admission. The new planet economy authority uses this
  policy for its bounded one-planet command batches.
- `zig/src/planet_economy/` implements the separate in-memory
  `nyon.planet-economy.v0` authority: typed seed/side/capacity configuration,
  budgeted creation and ticks, structural command admission, settlement and
  building phases, freight, population, paged receipts and replay. The four
  fixed CLI stories now drive its public runner and print actual published
  receipt bytes and observed settlement rows. The freestanding wasm probe is
  a compilation boundary only.

## Remaining implementation order

1. Extend the scoped planet authority into galaxy/civilization rules, research,
   ecology, diplomacy, fleets and fixed-step ground combat through one clock.
   The current one-planet economic rules do not decide those contracts.
2. Design durable new-version saves, history archives/branches and exact
   Continue beyond this in-memory receipt and replay boundary. Existing user
   data has not been read, migrated or mutated by the new slice.
4. Deliver real AppKit/Metal macOS journeys, input/IME, retained UI, accessibility
   and audio; then Win32/DX12, Linux Vulkan and browser WebGPU/WebGL2/IndexedDB.
5. Replace atlas/shader tooling with Zig, complete content and qualification,
   and retire Rust only after replacement proof. No new dependency acquisition,
   commits or publication is authorized by this implementation record.

## Qualification boundaries

Historical foundation verification on 2026-10-11: `zig/tools/check.sh` exited 0 with Rust commands
blocked on PATH. Debug reported 479 tests passed; ReleaseSafe tests, ReleaseSafe
executable build, Classic filesystem CLI checks and surface CLI vectors all
completed successfully. Surface CLI checked two independently derived seam
vectors and four rejected inputs. `git diff --check` passed. Router changes
were compared to formatter output from HEAD and contain only formatter edits.
The gate log is `/tmp/nyon-zig-20261011-final-gate.log` (temporary evidence).
This is scoped Zig evidence, not the old Rust CI or complete-game acceptance.

Fresh foundation review on 2026-10-11 found no scoped topology/role correctness
defect. A focused `zig build test -Doptimize=ReleaseSafe --summary all` exited 0
and explicitly reported 479/479 tests passed; an accompanying `failed command`
diagnostic remains unexplained and does not replace the successful summary.
The review checked 96 maximum-side reciprocal seam pairs. Repository CI still
requires Rust; final CI replacement remains an Important acceptance gap.
Independent seam orientation fixtures beyond positive Z remain a Minor coverage
gap. See `2026-10-11-zig-foundation-review.md` for evidence and limitations.

The planet economy slice adds local settlement, production, freight, population,
receipts and replay semantics beyond the foundation result above. Task 7's
direct Debug and ReleaseSafe aggregate reported 103/103 tests, including
exhaustive cancellation at 14,360 charged creation boundaries for seed 2 /
side 4 (96 cells), 3,617 nonempty and 3,461 author tick cancellation boundaries,
3,265 cleanup units, 1,185 dirty mutable pages, and zero polling allocations.
Separate tests compare topology against an independent face-basis oracle for
every cell/direction at every side 4..32, and construct side 32 with all maximum
capacities and exactly 8,642,560 requested bytes. These are distinct coverage
claims; creation cancellation was not enumerated for every admitted side.
Task 7's earlier empty-tick timing preceded the final invariant-validation path;
its 378-sample p95 5,666 ns / maximum 94,208 ns is historical evidence only.

Task 8's final `zig/tools/check.sh` exited 0 on 2026-10-11 with Rust commands
blocked on PATH. Direct Debug and ReleaseSafe test executables each printed
582/582 passed. The native subprocess checker passed the four complete CLI
stories, default invocation and eight invalid inputs, comparing all 27 ordered
published tick rows against fixture expectations. The isolated planet suite
directly printed 105/105 passed. The wasm32-freestanding ReleaseSafe object
compiled, and Classic store CLI (57 process checks) and surface seam CLI checks
passed. `git diff --check` exited 0. Full command/log evidence is in
`.superpowers/sdd/2026-10-11-nyon-zig-planet-economy/task-8-report.md`.

The final review fix adds a precharged semantic validation cursor between history
finalization and publication. COW page differences select touched records;
reference lookups, housing/reservation totals and route checks advance under the
same budget. The CLI now includes actual observed freight transit/delivery/absence.
The unchanged maximum reservation is 8,642,560 bytes, below 16 MiB. Fresh timing
of this poll path on Apple M5 / 24 GiB / macOS 27.2 (26B5101f), with concurrent
local test work, measured 462 budget-256 polls across 14 side-32 empty ticks after
two warmup ticks: p95 666 ns, maximum 1,625 ns, peak 1,217 pages. This qualifies
only that ReleaseFast empty-tick fixture, not maximum-loaded gameplay latency,
allocator overhead, browser execution or host presentation. The historical Task 7
benchmark log is preserved. The single final fix `./zig/tools/check.sh` run exited 0: both Debug and
ReleaseSafe directly passed 585/585 tests, including 5,377 author and 6,529
nonempty tick cancellation cuts, unchanged 14,360 side-4 creation cuts and
3,265 cleanup units. Release build, all CLI stories/default/errors, wasm object,
57 Classic process checks and surface checks passed. Across all four stories,
153 printed receipt lines, 27 tick chains and final heads match the captured
pre-fix executable byte-for-byte; only the added Freight column changes output.
The single scoped re-review approved all three fixes with no residual finding.
Full gate/parity/benchmark evidence and limits are recorded in
`.superpowers/sdd/2026-10-11-nyon-zig-planet-economy/final-fix1-report.md`.

Final delivery remains Partial: richer galaxy/game rules, durable sessions and
saves, desktop/browser host execution, accessibility, visual acceptance and
performance at maximum capacity remain unqualified. Repository CI still runs
Rust; the Zig-only gate and wasm object compilation do not retire that CI or
prove a Rust-free repository. No live browser/IndexedDB, native graphical host
or saved-user-data behavior is established by these checks.
