# Independent seeded galaxy design review

Date: 2026-10-11. Verdict: **Changes required before written-spec approval**.
Two Important contract findings; no Critical finding. This is a review of
Proposed rules, not an implementation verdict or approval of gameplay choices.

## Reviewed artifacts and scope

- `docs/superpowers/specs/2026-10-11-nyon-zig-seeded-galaxy-design.md`, SHA256
  `8a5a81d62485877949dacfbb5c4cc41d2e298f4fe68e5a05312734e6fdf8fd00`.
- `docs/qualification/2026-10-11-expanded-delivery-prerequisites.md`, SHA256
  `f6b2844c601b46c162a3252436346d52c05f9d3ba0e9deebca0a4bd88efac9ce`.

Read the home and repository instructions, approved expanded roadmap,
coordinated-galaxy specification, relevant earlier Living/platform references,
and current Zig config, reservation, role, generator and navigation seams.
This reviewer changed only this previously absent report. No product source,
author document, active coordinator ledger, build, profile, dependency or commit
was changed. No build, test suite or hardware qualification was run.

## Findings

### F1 — Important: identify the persistent owner of the lane and token namespace

Locations: seeded specification **112, 133–151, 158–159**.

The contract promises one setup lane, Busy, never-reused IDs/generations,
StaleJob rejection and safe polling after deinitialization. Its public entry
points instead take only an allocator and config/bytes and return an owning
Job. No persistent owner is defined for the lane, next-ID counter, generation
counter or token lookup after a Job is destroyed. Two independently constructed
Jobs, or destruction followed by reconstruction, can both start at `{1,1}` unless
the implementer invents a namespace. An allocator is not that namespace. The
contract also does not say when taking a manifest reopens the lane or how a
retained taken manifest counts against the advertised aggregate byte ceiling.

**Required fix:** specify a session-owned Setup controller (or an equally
explicit alternative) that owns the lane and checked counters for its lifetime.
Define which receiver validates a token, how tokens from another controller or
a destroyed/recreated controller are rejected, and which terminal operation
releases the lane. Define controller destruction with a live Job and the
independent lifetime of taken manifests. State whether 131,072 bytes is the
additional per-job reservation or a controller-wide retained-output ceiling;
include the resident old manifest plus the replacement job when reporting
replacement peak memory. These are Proposed choices until approved.

Add witnesses for two simultaneous begins, old token after take/new begin,
cross-controller token use, controller teardown/recreation, detached-manifest
destruction and replacement while the old manifest remains resident. Do not
solve this with process-global mutable counters or a silent extra allocation.

### F2 — Important: reconcile decoder admission, validation order and copy accounting

Locations: seeded specification **113–119, 138–139, 156–159, 176–187**.

Lines 156–157 require version, dimension and relationship validation before
allocation. Decoder admission at lines 138–139 allocates/copies admitted bytes,
and lines 181–184 defer header/schema validation to poll, allowing only the byte
ceiling to reject immediately. Invalid-version wire therefore has incompatible
allocation/error timing under the two rules. Meanwhile each copied record costs
a work unit, but beginDecode performs an entire admitted-input copy before any
poll budget exists. The copy is bounded at 16,384 bytes; that makes it finite,
but does not resolve which work contract it follows.

**Required fix:** make the preallocation version/dimensions/relationships rule
explicitly apply to beginCreate, and state the separate decoder validation and
error precedence. Either define the synchronous admission copy as an explicit
bounded primitive outside poll, with its own work/latency accounting, or move
copying into a budgeted ownership protocol that still prevents caller mutation
from changing admitted input. Do not leave an implicit borrowed slice. Define
whether Busy/token exhaustion or invalid wire-size wins when both apply, and
preserve the live lane for request errors.

Add exact failure-order and allocation witnesses for oversized, truncated,
wrong-version and invalid-count inputs; include caller mutation immediately
after begin, allocation failures and the maximum admission-copy measurement.
The fix must preserve incremental recomputation/hashing and zero poll allocation.

## Findings not raised

The prerequisites report's current compiler/build/store/Wasm/CI observations
are supported by the cited source. Its platform approval claim matches that
specification's approval metadata; its qualification row now correctly names
the approved four-world contract. The stale platform compiler and historical
profile/authority dependencies are explicitly reported as discrepancies.
No unsupported full-game, active-coordinator or hardware completion claim was
found in the reviewed report. Required final hosts remain required even where
their implementation details are Proposed.

The new generator does not silently adopt Classic fleet speed or clock rules.
`zig/src/navigation.zig:4–25` supports integer square root/distance;
`zig/src/play_role.zig:3–25` supports the stated role admission semantics.
The implementation plan must preserve the pure dependency boundary when using
that math seam: `navigation.zig:2` imports Classic `model.zig`, so importing the
whole Classic navigation API is not evidence of a clean new strategic boundary.

`zig/src/coordinated_galaxy/types.zig:26–45,254–282` supports the explicit small
bridge, including global history capacity and terrain seed derivation.
`zig/src/coordinated_galaxy/state.zig:6–37` supports the requested reservation
figures as source layout claims. No execution or final coordinator acceptance
is inferred from these reads.

## Independent arithmetic and wire checks

**Current review evidence:** a disposable Python standard-library calculation
implemented the written masking/mixing and wire formulas independently of NYON
modules and the author's reference program. It confirmed:

- `M(0) = 0xE220A8397B1DCDAF`.
- Seed 0, systems 2, planets 4, civilizations 2, side 8 has 177 canonical bytes
  and SHA256 `d2453927e7ba740f64ade02f4a63175fcf45d343b32817df54bbba86e9ec72b1`.
- The maximum configured wire has **11,455 bytes**, below 16,384.
- `512 * 8,642,560 = 4,424,990,720` requested island bytes.

The historical Living specification at lines 45–52 supports the 12-system,
24-world, four-civilization default reference and 64/512/16 upper scale.
It supplies no approval to import its colonies, lanes, clock or other mechanics.

These calculations are design evidence only. The required checked-in independent
oracle, seed-max/boundary fixtures, implementation consumption of literal wire,
malformed-wire tests and cross-budget/build-mode equivalence remain mandatory.
The review does not waive them because one reference witness agrees.

## Scope and remaining qualification

After F1/F2 are resolved, this is a coherent next **immutable metadata** slice.
The keyed grid, distinct founding designations, no grants, distance-only metric,
separate digest and explicit small bridge are concrete Proposed choices for
Donald to approve. They do not form a playable campaign start. The default
12-system/24-planet manifest cannot enter the four-world coordinator.

The proposal explicitly retains the expanded roadmap's cities, fleets,
research/ecology/diplomacy, tactical ground combat, durable sessions, content and
all native/browser hosts. It does not shrink their mandatory scope. Subsequent
integrated authority must bind strategic identity in reviewed genesis/history,
provide one shared clock/transaction/work budget, and demonstrate the accepted
large playable scale. Generating 512 metadata records or rejecting the bridge
at that scale is not full-scale integration acceptance.

**Unknown/unverified:** implementation correctness; actual struct/control
layout, allocator overhead and replacement resident memory; maximum loaded
economy capacity; poll and admission-copy latency on named hardware; native/
Wasm deterministic execution; user-visible navigation, startup, save/Continue,
accessibility and final host behavior. No claim in this report closes those
rows or authorizes dependent product implementation.

## Report hygiene

The report target was checked absent before creation. Review hashes were read
from the named frozen artifacts. Only documentation hygiene is checked for this
new report; the required implementation gates remain for their owning tasks.
