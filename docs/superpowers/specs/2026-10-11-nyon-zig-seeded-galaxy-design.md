# Seeded strategic galaxy and civilization initialization

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Historical; Zig continuation canceled 2026-10-19.** Preserve this source contract and evidence. Active recovery targets Rust stable1.99.1; see the 2026-10-19 Rust designs/plans. Historical review labels do not approve new Rust behavior. Tests and CI gates remain paused.
<!-- END 2026-10-19 rewrite decision -->


Status: **Controller-approved decisions under delegated authority; independent
re-review pending**, 2026-10-11. Donald delegated game decisions and approvals at
08:19 EDT and requested continuation at08:20 EDT; this does not claim he reviewed
this exact revision. Implementation requires successful independent review and a
detailed implementation plan. This artifact changes no coordinated-galaxy v0
contract and claims no implementation, runtime qualification or delivered game.

## Authority and scope

The expanded roadmap requires seeded galaxy generation, civilizations, creator,
ruler and combined play, one authoritative clock, deterministic replay and a
Rust-free delivered system. Existing profiles are never read, converted, changed
or deleted (expanded-zig-rewrite.md:9-23,32-43,45-58).

Coordinated-galaxy v0 is a qualification-sized four-world economy coordinator.
Its approved rules explicitly defer strategic positions, travel and civilization
decisions, require a reviewed identity/phase extension, prohibit genesis grants,
and freeze the terrain formula (coordinated-galaxy-design.md:58-117). This proposal
adds a separate immutable strategic genesis manifest. It does not introduce
fleets, autonomous decisions, diplomatic relations, research, colonization,
city recipes, tactical combat or persistent saves. Those remain required future
subsystems and need their own approved rules.

## Capacity decision

Proposed full strategic manifest bounds: 1..64 systems, 1..512 planets, 1..16
civilizations, side 4..32, at least one planet per system and at least one distinct
founding planet per civilization. Require systems <= planets and civilizations
<= planets. Default: 12 systems, 24 planets, four civilizations, side8. These
values preserve the existing Living direction's default galaxy scale without
claiming the old Rust rules apply to the rewrite. IDs are contiguous one-based
enum(u32) values, assigned in stable manifest order and never reassigned.

This is a strategic metadata capacity, not a promise to allocate 512 existing
planet authorities. At 8,642,560 requested bytes per island, 512 islands alone
require 4,424,990,720 bytes; coordinated v0 supports only four and a 47,153,152-byte
maximum reservation. Expanding its arrays or creating 128 independent coordinators
would neither satisfy its contract nor establish global atomicity. Large playable
galaxy authority therefore requires a subsequent reviewed storage/coordinator
capacity design, including shared page pools or bounded resident economies,
before implementation dispatch for full-scale simulation. No invisible ticking
of unloaded planets is allowed. Four-world compatibility qualification does not
satisfy full-game scale acceptance.

## Generator rules

Rules identity: `nyon.seeded-galaxy.v0`; generator version:u16 is exactly1.
Config: seed:u64, systems:u16, planets:u16, civilizations:u16, side:u16 and
version:u16. Generated manifest is immutable, with no mutable PRNG state and no
clock. Tick0 has no settlements, assets, resources or gameplay grants.

Use keyed SplitMix64, independently for each entity and field. Let M be the
SplitMix64 output transform: add wrapping 0x9E3779B97F4A7C15; xor-shift30 and
wrapping multiply 0xBF58476D1CE4E5B9; xor-shift27 and wrapping multiply
0x94D049BB133111EB; xor-shift31. Define draw(domain,id,field) =
M(seed XOR (u64(domain)<<56) XOR (u64(id)<<16) XOR u64(field)). Domains1,2,3
mean system, planet, civilization; field IDs are frozen below. Only these mixing
operations wrap; all bounds, indexes, counts and distance calculations are
checked. No standard-library random algorithm or host entropy participates.

System i occupies grid slot i-1 in an eight-column grid. Coordinates are signed
i32 integer strategic units: x=((i-1)%8)*4096+draw(1,i,0)%1024;
y=((i-1)/8)*4096+draw(1,i,1)%1024; z=0. Distinct slots guarantee distinct
positions and bounded spacing without rejection sampling. Systems have names
derived from IDs (`System 1`, etc.); localization and display names are excluded
from authority bytes.

Planet p belongs to system 1+((p-1)%systems); the ascending planet subsequence
within that system defines orbit_index:u16 starting0. Orbit_index is routing and
presentation metadata, not a physical orbital simulation. Planet terrain_seed
retains `seed XOR ((p-1)*%0x9E3779B97F4A7C15)` to permit exact v0 economy bridging.
Terrain generation itself remains the approved placeholder; strategic generation
does not claim varied climate/ecology or change cube adjacency. No invented
planet-field ecology values are emitted.

Civilization c has founding_planet=c, kind=`uninitialized`, and stable ID c.
Home designation is metadata, not ownership or a colony. Selecting a ruler
requires a present civilization but grants nothing. Civilization names are
derived from IDs; no random personality, knowledge or technology is inferred.
Future founding supplies must use explicit receipted creator commands or a
separately approved scenario-start contract. The empty manifest is inspectable
but is not a sufficient playable campaign start.

## Navigation boundary

No lane graph or fleet orders are admitted in this subsystem. The strategic
metric for systems is floor(sqrt(dx²+dy²)) using checked i128/u128 differences
and the integer square-root algorithm; same-system metric is0. This returns
distance, never duration. Travel speed, within-system orbital travel, route
topology, zero-length minimum duration and fleet scheduling require a later
approved fleet contract. Do not reuse Classic speed23 or its real-time tick rate.
The coordinated clock remains one hour per tick and horizon1,000,000.

## Roles and initialization

Use the existing SessionPolicy admission semantics: creator provenance has null
actor; ruler actor equals selected civilization; combined admits both;
autonomous provenance is internal-only. Manifest construction is trusted session
setup, not a ruler command or secretly authored grant. A setup policy is stored
as separate session input and is not included in the immutable manifest digest.
No setup policy implicitly becomes creator. Policy validation is role admission,
then selected civilization presence. Ruler and combined require non-null present
selection; creator accepts null or present selection. No command changes role or
creates/deletes a civilization after genesis in this proposal.

## Incremental creation and failures

Creation owns one manifest allocation and one fixed scratch allocation, each
<=65,536 bytes, aggregate requested ceiling131,072 bytes including controls,
alignment and job bookkeeping. Implementations must demonstrate actual struct
layout and accounting before approval of the plan. No allocation during poll.
One setup lane; no queue. Poll budget0..4096, default256. Budget0 observes without
mutation. One generated record, validated record or copied record costs one unit; each
record is at most256 bytes and uses bounded primitive arithmetic. Encoding costs
one unit per output record, with header at most256 bytes counted as one record.
Hashing charges one unit per SHA256 compression block: at most64 input bytes,
including the final padding block(s). Preparing and compressing each block occur
in that unit; no whole-message hash is allowed. Bounded digest finalization costs
one additional unit. No phase performs an uncharged whole-manifest scan. Caller
queries after readiness synchronously copy at most64 records (each <=256 bytes),
with no hashing or allocation; these bounded query calls are separate from poll
work allowance. Interior pointers do not escape. SHA256 uses fixed scratch.

Phases: validate bounded config, generate systems, generate planets, generate
civilizations, validate cross-references, encode/hash, ready. Records are in
ascending ID order. Every phase yields at exhaustion; caller polling cadence
cannot influence bytes. Cancellation retires owned scratch/output, produces no
partial published manifest, and preserves the current session. Construction
becomes ready only after all validation and hashing succeeds. Taking a ready
manifest transfers unique ownership exactly once. No partial installation into
the resident session occurs in this module.

Public lifecycle is session-owned `Setup`, never independent Jobs:
`Setup.init(allocator, owner_namespace:u64)` creates an idle controller with one
lane and inline controls, without allocation; namespace0 is invalid. The session issues monotonically
increasing checked namespaces and never reuses a namespace within its session
lifetime, including controller recreation. Namespace exhaustion is IdExhausted. The pure module validates namespace equality, not process-wide uniqueness;
callers owning multiple controllers must account for them and provide distinct
namespaces. Reinitializing a controller needs a fresh namespace. Calls use a live Setup value;
no API authorizes dereferencing destroyed controller storage. Reusing the same
storage with a fresh namespace rejects all old tokens.

`beginCreate(Config by value)` or `beginDecode(input bytes)` on an idle Setup
returns a token `{owner_namespace:u64,id:u64,generation:u64}`. ID and generation
start1 and increment checked on each admitted begin, never reset when the lane
reopens, never wrap/reuse during that controller lifetime. Reserve both counters
before allocation; exhaustion returns IdExhausted. Allocation failures may consume
reserved counters but publish no job and leave the lane idle. A busy begin returns
Busy before reserving counters. Config is copied by value.

`poll(token,budget)` yields pending, ready, canceled or failed; `cancel(token)`
requests cleanup; `take(token)` succeeds only at ready, transfers the owning
Manifest, releases scratch and reopens the lane; `release(token)` disposes a
ready/failed/canceled outcome and its remaining allocations, then reopens the
lane. Failure/cancellation cleanup completes through poll before outcome release;
ready retains lane ownership until take/release. Cancel of ready transitions to
cleanup without publication. Repeated cancel while cleanup is pending is
idempotent. Token equality includes namespace, ID and generation; any stale,
foreign, taken or released token returns StaleJob, even before another begin.
Take of a matching active non-ready token returns NotReady. `Setup.deinit()` frees
only currently owned scratch/output (at most two frees, no record walk), marks
controller closed and is idempotent. Closed controller operations return Shutdown.
A taken `Manifest.deinit()` is separate and idempotent; allocator provenance moves
with ownership. Queries copy values and create no leases or interior pointers.

Decode begin performs only input length admission (0..16,384 bytes), counter
reservation and allocation before snapshot copying the complete input into owned
scratch before return. This synchronous bounded copy is an explicit exception to
polled work charging: at most16,384 bytes, no decoding, hashing or recomputation.
Record snapshot bytes and copy latency separately in qualification. No borrowed
slice survives begin or caller mutation. Header/schema validation and generated
value recomputation are charged during poll. Creation preallocation validation
is distinct and described below; decode cannot claim that validation occurred
before its owned input allocation.

Each Setup's active candidate uses at most two65,536-byte allocations, aggregate
131,072 bytes including controls/alignment; no second decoded manifest allocation.
A detached Manifest owns at most65,536 bytes including controls/alignment. Session
policy retains at most one resident manifest plus one active replacement candidate;
peak replacement requested bytes are196,608. Once take creates the replacement,
at most two detached outputs total131,072 bytes are retained briefly for atomic
session pointer installation; dispose the superseded resident before admitting
another candidate. Copied reads need no lease drainage. Failed/canceled candidate
leaves the resident unchanged. The module supports independently accounted caller
ownership, but this session policy is normative for the game and does not claim
multiple controllers or detached manifests are free. Setup shutdown never frees
a detached Manifest. Installation is a caller transaction after ready/take, not
hidden publication by setup.

Errors: InvalidVersion; CapacityUnsupported for out-of-range dimensions;
InvalidConfig for counts relationships; InvalidPolicy; OutOfMemory;
InvalidBudget; Busy; StaleJob; NotReady; Shutdown; ArithmeticOverflow;
InvariantViolation; IdExhausted; InvalidWire; UnsupportedWireVersion. Creation
validates version, dimensions, relationships in that order before allocation.
Decode admits only bounded byte length before allocating its snapshot; equivalent
configuration validation occurs incrementally during poll. Invalid poll budget changes nothing. Allocation failure frees
all owned allocations. Job IDs start1 and increment checked, never reuse or wrap;
exhaustion is IdExhausted. Caller retains its existing state after every error.

## Canonical wire

All integers unsigned little-endian unless explicitly signed; coordinates use
two's-complement i32 little-endian. No padding, pointers, locale, enum-layout
assumptions or optional trailing data. Header: magic bytes `NYSG`, wire_version
u16=1, identity_len:u16 followed by identity UTF8 bytes, generator_version:u16=1,
seed:u64, systems:u16, planets:u16, civilizations:u16, side:u16. Then system_count
records `(id:u32,x:i32,y:i32,z:i32)`; planet_count records
`(id:u32,system:u32,orbit_index:u16,terrain_seed:u64,side:u16)`; civilization_count
records `(id:u32,founding_planet:u32,kind:u8=0)`. IDs/order/counts must match config.
Identity is ASCII `nyon.seeded-galaxy.v0` exactly. Digest is SHA256 over the entire
canonical wire, excluding the digest itself. The maximum wire is under16384 bytes.
This wire is an in-memory provenance/export contract, not persistent session
save authorization. Decoder rejects truncation, trailing bytes, unsupported
versions, nonzero z/kind, count/order/reference errors and any generated value
different from recomputation. A supplied digest is never proof of validity. Decoder uses the same owned-input
lifecycle, bounds, cancellation and work allowance as creation: each parsed
record <=256 bytes costs one unit, validation/recomputation costs one unit per
record, and digest work uses the same charged SHA256 blocks. It recomputes fields
from the copied config incrementally and compares each parsed record without a
second manifest allocation. Begin can reject only the byte ceiling immediately;
truncated/header/schema failures discovered during poll terminate as failed with
InvalidWire, UnsupportedWireVersion, InvalidVersion or InvalidConfig as
appropriate. Extra bytes, malformed identity, ordering/reference/value mismatch
return InvalidWire. No malformed decoded result becomes ready. InvalidBudget or
StaleJob is a request error and leaves a valid job unchanged; arithmetic/invariant
failures terminate that job and preserve resident session state.

## Explicit coordinated-v0 bridge

An adapter may produce a v0 Config only when systems, planets and civilizations
all <=4 and every planet side/capacity is accepted by v0 validateConfig. It copies
seed/counts/system membership/side and explicitly supplied v0 local capacities
and global_history_capacity (1..4096), preserving every supplied value exactly; it
never changes v0 terrain, identity, genesis bytes or phase order. Larger inputs
return CapacityUnsupported before any authority allocation. It cannot drop
planets/civilizations, remap IDs or fabricate an accepted four-world sample.

The manifest's strategic digest stays separate from v0 history head. A session
may retain both with explicit association, but existing v0 replay proves only
economy authority and cannot certify strategic bytes. A future integrated rules
identity must bind the manifest digest in its own approved genesis/history;
appending fields to v0 genesis is forbidden. Founding metadata likewise does not
authorize settlement creation in v0. A later receipted setup can author founders,
then explicitly hand off to ruler play without disguising creator effects.

## Independent witnesses and acceptance

1. Handwritten mix witness: M(0)=0xE220A8397B1DCDAF. Separate Zig oracle or a small
   checked-in non-Rust oracle must verify keyed draws, seeds0/max, coordinates,
   exact wire and SHA256 against independently computed fixtures before source
   implementation; do not update expected bytes from implementation output.
2. Seed0/config systems2,planets4,civilizations2: memberships1,2,1,2; orbit indexes
   0,0,1,1; founding planets1,2. Terrain seeds for planets1/2 are0 and
   0x9E3779B97F4A7C15. Exact independent coordinates and wire appear below.
3. Systems64/planets512/civilizations16: every system has eight planets; last
   system lies in grid row7,column7; all positions distinct; IDs complete. This
   fixture must pass metadata generation and fail v0 bridge without side effects.
4. Compare exact bytes/digests across budgets0,1,7,256,4096, cancellation/retry and
   Debug/ReleaseSafe; randomized schedules must produce identical ready output.
5. Exhaustive allocation/cancellation cut points, retained active-state witness,
   duplicate take returning StaleJob, stale/foreign/exhausted controller tokens, malformed wire and zero poll allocations.
6. Creator/ruler/combined admission matrix, absent selected civilization and no
   startup grants; bridge exact genesis equivalence against unmodified v0 output.
7. Record max metadata requested bytes and charged units, then p95/max poll time
   on named hardware. Proposed target p95<=2ms,max<=8ms with budget256; timing is
   evidence independent from deterministic work accounting.

## Independent reference fixture

Computed 2026-10-11 with a disposable Python standard-library reference using
explicit masked integer arithmetic, `struct.pack` little-endian records and
`hashlib.sha256`; it imports no NYON implementation, encoder or Rust tool. Python
is fixture preparation evidence, not a required final delivery dependency.
Config is seed0, systems2, planets4, civilizations2, side8, generator version1.
System1 draws fields0/1: `4AE81809EEB05379`, `FF2ED4AFDFE0AD71`, yielding
coordinates `(889,369,0)`. System2 draws: `8452BC15E225812F`,
`F17FF086E1314AFC`, yielding `(4399,764,0)`. The position calculation applies
modulo1024 before adding the grid offset. Planet terrain seeds are
`0000000000000000`, `9E3779B97F4A7C15`, `3C6EF372FE94F82A`,
`DAA66D2C7DDF743F`; multiplication is masked to64 bits.

Canonical wire is177 bytes: header47 + systems32 + planets80 + civilizations18.
Identity byte length21. The complete hex below is one byte string (line breaks
are presentation only):

```text
4e595347010015006e796f6e2e7365656465642d67616c6178792e7630010000000000000000000200040002000800
01000000790300007101000000000000020000002f110000fc02000000000000
010000000100000000000000000000000000080002000000020000000000157c4a7fb979379e0800
030000000100000001002af894fe72f36e3c0800040000000200000001003f74df7d2c6da6da0800
010000000100000000020000000200000000
```

SHA256: `d2453927e7ba740f64ade02f4a63175fcf45d343b32817df54bbba86e9ec72b1`.
Implementation tests must consume these independently prepared literals; future
algorithm changes require a new generator/rules version rather than replacement
expected bytes. Reference recomputation and fixture copying still need review.

## Alternatives and approval decisions

Recommended: keyed grid generation plus separate immutable strategic manifest,
with an honest small v0 bridge and a later integrated scale design. It gives
bounded reproducible topology metadata without rewriting actively owned v0.

Alternative A: curated anchor catalog with seeded bounded jitter. Better authored
campaign geography, but needs a catalog identity/content approval and independent
asset tooling; permits fewer novel layouts without a larger catalog.

Alternative B: random positions with deterministic rejection/minimum separation
and connected lane graph. More organic geography but introduces retry bounds,
fallback placement, graph tie-breakers and new navigation rules. Defer unless
Donald prefers its gameplay tradeoff enough to approve that larger contract.

Controller decisions under Donald's delegated authority select the full manifest
bounds/default, keyed-grid layout, home
designation without automatic colonies, distance-only navigation boundary,
versioned strategic wire and separate-digest bridge, memory/work ceilings, and
the requirement for a subsequent integrated large-galaxy coordinator design.
Independent review must check these before a detailed implementation plan. No
approval of autonomous civilization strategy or new startup grants is implied.

## Evidence consulted

- `docs/superpowers/plans/2026-10-11-expanded-zig-rewrite.md:9-23,32-58,83-102`.
- `docs/qualification/2026-10-11-expanded-rewrite-acceptance.md:8-28,31-35`
  (dated audit, not a fresh current implementation verdict).
- `docs/superpowers/specs/2026-10-11-nyon-zig-coordinated-galaxy-design.md:50-138,223-227`.
- `zig/src/coordinated_galaxy/types.zig:26-45,254-282`; `state.zig:6-37,47-79`.
- `zig/src/generator.zig:1-77` is frozen Classic; `navigation.zig:4-25` supplies
  an integer distance seam; `play_role.zig:3-25` supplies admission semantics.
- `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md:41-62` is historical
  scope reference, not authority to import old mechanics into the new identity.
