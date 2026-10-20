# Tactical distance candidate investigation

Date: 2026-10-11. Status: **Partial, independent geometry experiment only**.
No game implementation or full-tick performance qualification is claimed.

The proposed movement rule ignores units and terrain when constructing its
reverse-BFS distances. It then chooses the neighbor minimizing distance and
original local CellId; occupancy and movement credit are checked afterward.
The cube graph therefore admits a candidate immutable distance table shared
across worlds and source/replay authorities with the same topology rules.

Signed axis permutations give48 candidate graph automorphisms. Map a goal to
positive-Z with `x<=y<ceil(side/2)`, retaining the selected transform for lookup.
There are `ceil(side/2)*(ceil(side/2)+1)/2` canonical goal fields per side.
Transform source cells for distance lookup but retain original CellIds for
movement tie-breaking. Reflections affect this undirected distance lookup only;
the directional firing-ray contract is unchanged.

A temporary independent Python model using the written cube bases/neighbor
formula compared compressed distances and movement choices with ordinary BFS.
The command exited0:572,388 distance comparisons and572,388 movement choices
matched;190,464 automorphism edge checks matched. Sides4,5,8 used all source/goal
pairs. Side32 used all6,144 sources for64 regularly spaced goals, not every goal.
Observed maximum distances were11,14,23,95 respectively. The Python run took
9.94 seconds; that is experiment execution time, not Zig or gameplay timing.
This comparison has not established parity with an independently executed Zig
table, all side32 goal orbits, every allowed side, or the complete combat tick.

For all sides4..32, u8 distance entries total6,061,056 bytes. A two-byte goal
mapping `(transform_index,canonical_goal_index)` totals137,112 bytes. Their sum
is6,198,168 bytes before descriptors, allocator/alignment and initialization
scratch. Adding this sum to the proposed source/replay reservation394,395,648
gives400,593,816 bytes, below384MiB by2,059,368 bytes. This is arithmetic only.
Any actual immutable-cache sharing needs explicit ownership and full measured
peak allocation; retaining per-authority copies would exceed this estimate.

A project-owned Zig feasibility probe must compare every side4..32 neighbor
under all48 transforms, every source/goal pair at side32, original-ID tie choices,
and measured requested memory. It must measure cached movement queries on named
hardware, retaining checksum outputs. The complete city/combat/history/hash/
cleanup tick still requires its separate5ms p95 qualification. Neither passing
geometry nor a fast movement microbenchmark closes that requirement.
