# Full-scale initial local-economy codec

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Historical; Zig continuation canceled 2026-10-19.** Preserve this source contract and evidence. Active recovery targets Rust stable1.99.1; see the 2026-10-19 Rust designs/plans. Historical review labels do not approve new Rust behavior. Tests and CI gates remain paused.
<!-- END 2026-10-19 rewrite decision -->


Date: 2026-10-11. Status: **Proposed normative companion; independent review
required together with the full-scale authority design.** Rules identity
`nyon.fullscale-galaxy.v1`; storage profile `nyon.fullscale-storage.v1`.
This is the initial local-economy identity only. Reserved strategic capacities
are encoded as profile facts, but strategic records/commands/events are forbidden.
No existing island/coordinated/seeded identity or expected vector changes.

## Scalars, envelopes and validation

`u8/u16/u32/u64` mean unsigned little-endian exactly that width. `bytes[n]` has
no prefix; `S(text)=u16(UTF8_byte_length)||UTF8`; `Opt(T)=presence:u8` (0 absent,
1 present followed by T); bool is u8=0/1. H is SHA256. Domains below are exact
ASCII without NUL or implicit length. Concatenation is `||`. Reject trailing
bytes, truncation, noncanonical booleans/optionals, unknown tags, incorrect fixed
identity, overlong strings/arrays and checked count/length overflow. No padding,
physical page handle or implicit machine-sized integer enters these bytes.

`Cell=planet:u32||local:u64`; local is the existing cube linear coordinate.
`Policy=role:u8||Opt(civilization:u32)`; role creator=0,ruler=1,combined=2.
Provenance creator=0,ruler=1,autonomous=2. Resource food=0,material=1;
building farm=0,quarry=1,house=2; direction east=0,west=1,north=2,south=3;
cargo source transfer=0,mint=1. Policy role/selected-civilization admission is the
approved coordinated rule; genesis policy is absent, never implicit creator.

Command body is `sequence:u64||provenance:u8||Opt(actor:u32)||tag:u8||payload`.
The outer planet:u32 qualifies all local entity IDs in the body. All Cell.planet
values must equal that route planet. Body cap1024; sequence>0; full tick count
<=1024 and local routed count<=64/world. Tags and payloads, in exact order:

| Tag | Payload |
| --- | --- |
| 0 author_settlement | owner:u32, anchor:Cell, population:u8, food:u32, material:u32, economic_bypass:bool |
| 1 grant | settlement:u64, food:u32, material:u32, population:u8, economic_bypass:bool |
| 2 author_build | settlement:u64, kind:u8, anchor:Cell, facing:Opt(u8), economic_bypass:bool |
| 3 author_ship | source:u64, destination:u64, resource:u8, amount:u16, path_count:u16, path_count Cells, cargo_source:u8, economic_bypass:bool |
| 4 found | source:u64, anchor:Cell |
| 5 build | settlement:u64, kind:u8, anchor:Cell, facing:Opt(u8) |
| 6 ship | source:u64, destination:u64, resource:u8, amount:u16, path_count:u16, path_count Cells |

Path count<=65 is structural; counts0/1 then reject InvalidPathLength during
command evaluation. Unknown enum/tag, malformed optional, missing/extra bytes
or overlong body aborts the whole tick before gameplay. Validly encoded but
forbidden role/provenance, unknown local target, invalid numeric gameplay range,
foreign/invalid cell, ownership, topology, quotas or resources yields a rejection
receipt in normal execution order. Outer unknown route and duplicate/old sequence
are structural whole-tick errors. Replay accepts only exactly encoded bytes and
compares all expected outcomes, not just supplied hashes.

## Genesis object and initial state

The immutable genesis object is outside the <=2048-byte receipt framing. It is
`"NYFG"||wire_version:u16=1||S(rules_identity)||S(storage_profile)||
manifest_length:u32||manifest_wire||manifest_digest:bytes[32]||
quota_count:u16=34||34 quota values:u32||initial_tick:u64=0||
initial_revision:u64=0||initial_sequence:u64=0||previous_policy:u8=0||
initial_next_settlement:u64=1||initial_next_building:u64=1||
initial_next_shipment:u64=1||initial_settlements:u32=0||
initial_buildings:u32=0||initial_shipments:u32=0`.

Manifest wire is exactly the seeded specification's `NYSG` version1 codec,
identity `nyon.seeded-galaxy.v0`, generator1, with systems/planets/civilizations,
side, seed, derived positions/membership/home metadata and terrain seeds. Decoder
recomputes and verifies it using that frozen generator; its hash must equal the
supplied digest. It is supplied once, owned by genesis/manifest, never fragmented
into receipt records and never duplicated per world. Genesis object <=65536 bytes;
manifest_length<=16384. Exact source contract is
`2026-10-11-nyon-zig-seeded-galaxy-design.md`, reviewed SHA256
`5577e8886f80a48df656fa1eb53ff3421a74c9deaecb5d2e7307001f43f4320d`.

Quota vector position/value/name is frozen:

```text
0 64 systems                 1 512 worlds
2 16 civilizations           3 4 minimum_side
4 32 maximum_side            5 4096 settlements_global
6 64 settlements_per_world   7 16384 buildings_global
8 256 buildings_per_world    9 36864 claims_global
10 576 claims_per_world      11 4096 local_freight_global
12 64 local_freight_per_world 13 65 path_cells
14 2048 strategic_industries  15 256 industries_per_world
16 2048 strategic_routes     17 256 routes_per_origin
18 4096 strategic_freight     19 4096 strategic_freight_per_world
20 256 fleets                21 2048 hulls
22 2048 extension_metadata   23 1024 commands_per_tick
24 1024 command_body_bytes   25 64 commands_per_world
26 64 leases                 27 131071 pages_per_root
28 131072 index_rows         29 67108864 history_cache_bytes
30 33554432 candidate_bytes  31 1000000 tick_horizon
32 4096 maximum_poll_units   33 1 hours_per_tick
```

This initial identity requires these exact values, not arbitrary smaller quota
profiles. Actual manifest dimensions can vary within them. Per-world limits are
uniform, so no hidden per-world capacity array is omitted from genesis.
`G=H("NYON-FS1-GENESIS"||genesis_object)`. Initial world metadata exists for each
enrolled world with all three next IDs=1 and all three live counts=0. No other
entity rows exist. Initial global counts/tick/revision/sequence are zero and
policy absent. Immutable civilization metadata exists only in the manifest;
there is no invented mutable civilization row. IDs of systems/worlds/civilizations
retain the manifest values; no settlement or resource grant is implicit.

## Canonical state grammar

`state_bytes = G:bytes[32] || tick:u64 || revision:u64 || sequence:u64 ||
Opt(Policy) || settlements_global:u32 || buildings_global:u32 ||
local_shipments_global:u32 || row_count:u32 || rows`.
Rows sort numerically by `(realm:u32,kind:u16,local:u64)` and encode
`realm||kind||local||payload_length:u16||payload`. Count/order/unique keys,
exact payload lengths, quotas, counters and all semantic references are validated.
Only realm1..manifest.planet_count is legal here. Kinds are:

| Kind | Key local | Exact payload |
| --- | --- | --- |
| 0 world metadata | 0 | next_settlement:u64, next_building:u64, next_shipment:u64, settlements:u16, buildings:u16, shipments:u16 |
| 1 occupied claim | coordinate (including0) | settlement:u64, building:Opt(u64), is_hall:bool |
| 2 settlement | nonzero stable ID | owner:u32, hall:Cell, population:u8, food:u32, material:u32, incoming_food:u32, incoming_material:u32, housing:u8, hunger:u8, fed_streak:u8 |
| 3 building | nonzero stable ID | settlement:u64, kind:u8, anchor:Cell, facing:Opt(u8), footprint_count:u16, footprint Cells in anchor/neighbor order, remaining_ticks:u8 |
| 4 shipment | nonzero stable ID | source:u64, destination:u64, resource:u8, quantity:u16, departure_tick:u64, arrival_tick:u64, path_count:u16, path_count local:u64 coordinates |

Claim must name present settlement; optional building is present exactly for a
building claim, hall=true implies no building. Stored footprint count1/2 and
facing must match the validated pure footprint formula. Shipment path2..65 uses
the containing realm, and references/arrival/resource reservations must validate.
Missing cell rows mean unoccupied derived terrain. Other kinds, empty/tombstone
rows, duplicate world metadata and realm-zero records reject. Stored per-world
counts and global sums must agree with actual rows. Local next counters are above
all IDs ever committed; replay validates deletion/nonreuse history. Physical
parent/path page layout is excluded. `S=H("NYON-FS1-STATE"||state_bytes)`;
state excludes history head and G excludes S, so hashing is acyclic.

## Complete receipt grammar and ordering

A stream record is `length:u32||receipt_bytes`, length<=2048, no zero length.
`receipt_bytes="NYON-FS1-RECEIPT"||tag:u8||tick:u64||ordinal:u32||
phase:u8||realm:u32||provenance:u8||Opt(sequence:u64)||payload`.
The header is36 bytes without sequence and44 with it. Ordinal starts0 at each
tick and increases1 for every record globally, including all inputs and commit;
it does not reset per world or phase. Genesis is tick0/ordinal0 only.

Phase tags: setup0,input1,commands2,arrivals3,construction4,production5,
population6,commit7. Tags/payloads:

| Tag | Phase / realm / sequence | Payload |
| --- | --- | --- |
| 0 genesis | 0 / 0 / absent | G:bytes[32], manifest_digest:bytes[32], initial_S:bytes[32] |
| 1 tick_input | 1 / 0 / absent | expected_revision:u64, previous_sequence:u64, previous_policy:Opt(Policy), current_policy:Policy, envelope_count:u16 |
| 2 routed_input | 1 / outer planet / present | body_length:u16, exact command body |
| 3 accepted | 2 / outer planet / present | command_tag:u8, effect below |
| 4 rejected | 2 / outer planet / present | reason:u16, offending_index:Opt(u16), quota_kind:u8 |
| 5 arrived | 3 / planet / absent | shipment:u64, source:u64, destination:u64, resource:u8, quantity:u16, destination_stock_after:u32, destination_reservation_after:u32 |
| 6 completed | 4 / planet / absent | building:u64, settlement:u64, kind:u8 |
| 7 produced | 5 / planet / absent | building:u64, settlement:u64, workers:u8, resource:u8, quantity:u32, stock_after:u32 |
| 8 idle | 5 / planet / absent | building:u64, settlement:u64, reason:u8, workers_needed:u8, workers_available:u8, resource:u8, quantity:u32, stock_before:u32, incoming_reserved:u32 |
| 9 supplied | 6 / planet / absent | settlement:u64, needed:u8, consumed:u8, shortage:u8, food_after:u32 |
| 10 demography | 6 / planet / absent | settlement:u64, kind:u8, population_before:u8, population_after:u8, housing:u8, hunger_after:u8, fed_streak_after:u8 |
| 11 tick_commit | 7 / 0 / absent | revision:u64, last_sequence:u64, accepted:u16, rejected:u16, worlds:u16, settlements_global:u32, buildings_global:u32, shipments_global:u32, S:bytes[32] |

For routed_input/accepted/rejected tags, provenance and sequence equal the corresponding
command header; body sequence and outer record sequence must match. All other
records have provenance autonomous=2 and absent sequence. Outcomes reference
exactly one routed input by sequence and matching realm; they never duplicate the
body. Exactly one accepted/rejected outcome exists per input. Idle reason
workforce=0/output_capacity=1; demography growth=0/starvation=1/housing_blocked=2.
Footprint in effects is `count:u16||count Cells` (count1/2). Effects by command tag:

| Command tag | Exact effect |
| --- | --- |
| 0 | settlement:u64, food_minted:u32, material_minted:u32, population_minted:u8 |
| 1 | food_minted:u32, material_minted:u32, population_minted:u8 |
| 2 | building:u64, footprint, material_waived:u32, ticks_waived:u8 |
| 3 | shipment:u64, transferred:u16, minted:u16, departure_tick:u64, arrival_tick:u64, reserved:u16 |
| 4 | settlement:u64, food_moved:u32, material_spent:u32, population_moved:u8 |
| 5 | building:u64, footprint, material_spent:u32, remaining_ticks:u8 |
| 6 | shipment:u64, transferred:u16, reserved:u16, departure_tick:u64, arrival_tick:u64 |

Rejected reasons are the frozen local codes1..37 plus38/39:

```text
1 CreatorForbidden, 2 RulerForbidden, 3 CivilizationRequired,
4 WrongCivilization, 5 InvalidActor, 6 InternalCommandOnly,
7 CommandProvenanceMismatch, 8 ExplicitBypassRequired, 9 UnknownCivilization,
10 UnknownSettlement, 11 WrongOwner, 12 InvalidPlanet, 13 InvalidCell,
14 InvalidAmount, 15 InvalidPopulation, 16 InvalidFacing, 17 TerrainMismatch,
18 OccupiedCell, 19 DisconnectedFootprint, 20 SameEndpoint,
21 InvalidPathLength, 22 RepeatedCell, 23 NonAdjacentPath, 24 ForeignPathCell,
25 SettlementCapacity, 26 BuildingCapacity, 27 SettlementBuildingCapacity,
28 FreightCapacity, 29 PopulationCapacity, 30 InventoryCapacity,
31 InsufficientFood, 32 InsufficientMaterial, 33 InsufficientPopulation,
34 TickHorizon, 35 IdExhausted, 36 InvalidGrant, 37 PathEndpointMismatch,
38 GlobalCapacity, 39 PlanetCapacity.
```

Codes25/26/28 remain reserved legacy local-code positions and are not emitted by
this identity; per-world record quota emits39. Code27 remains the local settlement
building limit from existing rules. quota_kind=0 for all reasons except38/39;
for those it is1 settlement,2 building,3 claim,4 local freight. Within the existing
capacity-before-resources precedence, evaluate per-world entity quota, global
entity quota, per-world claim quota, global claim quota, then remaining original
local capacity/resource checks; emit the first failure. Claim quota checks occur
only if that command adds claims. This ordering never changes earlier role,
existence, ownership, numeric or topology rejection precedence.

Tick order is exact: tick_input; all routed_input records in **global ascending
sequence**; all outcomes in **ascending world then ascending sequence**; arrivals
by world/shipment ID; construction by world/building ID; production by world,
settlement ID,building ID; population by world/settlement ID with supplied then
optional demography; one tick_commit. No per-world genesis/input/commit receipt
is duplicated. Empty worlds still execute phases but emit no placeholder events.
For the quota collision, world1 sequence2 executes before world2 sequence1 even
though the two routed_input records appear1 then2. Accepted count+rejected count
must equal envelope_count. Commit last_sequence is max of previous and all input
sequences, including rejected inputs. Every committed world advances; zero-input
ticks retain previous_sequence. Tick/revision increase1. Previous_policy is absent
only at the first tick and must exactly match the prior committed optional policy.
A policy change becomes visible with publication, never with begin.

Initial head is `H("NYON-FS1-HISTORY")`. For every receipt update
`head=H(head||u32(length)||receipt_bytes)`, including genesis and tick_commit.
Record framing bytes are exactly the bytes used by this recurrence. Cancellation
or structural failure publishes no record and preserves the previous head/state,
even if candidate bytes were generated. Canceled first tick leaves genesis policy
absent and next retry again emits absent previous_policy at ordinal0. Receipt
integrity is deterministic corruption detection, not network authorization.

## Source-derived maximum initial tick

The full-profile local maxima are C=1024 commands, F=4096 shipments,
B=16384 buildings/projects and T=4096 settlements. Existing command cursor emits
one outcome per command (`command.zig:1145-1164,1326-1349`); phases emit at most
one arrival/shipment (`phases.zig:86-102`), one completion/building
(`157-198`), one produced OR idle/building (`201-255`), one supplied and at most
one demography/settlement (`260-313`). Construction completion and production may
both occur for one building in a tick, so both are budgeted. Shared receipt_a/b
slots are drained once (`35-57`); cursor scheduling does not add events. Future
strategic reservations emit nothing under this identity.

| Family | Max count | Max receipt bytes | With four-byte frame |
| --- | ---: | ---: | ---: |
| tick_input | 1 | 67 | 71 |
| routed_input | 1024 | 1070 (44+2+1024) | 1074 |
| accepted OR rejected | 1024 | 84 (44+1+39); rejection<=50 | 88 |
| arrived | 4096 | 71 | 75 |
| completed | 16384 | 53 | 57 |
| produced OR idle | 16384 | 68 (idle); produced62 | 72 |
| supplied | 4096 | 51 | 55 |
| demography | 4096 | 50 | 54 |
| tick_commit | 1 | 102 | 106 |

Maximum records `2+2*C+F+2*B+2*T = 47,106`.
Maximum framed bytes
`71+1024*(1074+88)+4096*75+16384*(57+72)+4096*(55+54)+106`
`=4,057,265`, below4 MiB by137,039 bytes and below32 MiB by29,497,167 bytes.
This deliberately counts mutually exclusive worst cases independently and uses
1024 body bytes although the currently admitted fields are smaller. It is a safe
upper bound, not a claimed achievable worst-case transcript. Global event ordinal
u32 is ample; record ceiling2048 is satisfied. Genesis is separate and occurs once.
The total includes all input/output records, frame prefixes and hash-chain lengths;
there is no uncounted local duplicate transcript. Pinned-compiler encoder and
maximum-fixture verification remain required and must match this frozen grammar.

## Independent literal fixtures

Computed using disposable Python standard-library `struct.pack` and
`hashlib.sha256`, with the printed grammar and explicit masked SplitMix64 formula;
no NYON implementation, encoder or Rust tool was imported. Full strings below
are literals (line breaks ignored), not expected output captured from production.
Fixture configuration seed0,systems1,planets2,civilizations2,side4, except
the final quota fragment, whose separate precondition uses64/512/16. These are codec
and hand-derived semantic witnesses; no new authority implementation has run.
Independent review must check the literal calculation before tests consume it.

### Genesis object

The immutable object includes manifest wire, all34 profile values and explicit initial counters. G=`424f312ec4c15049fbb2ad8a4d5b83c3d25daa287b47ed081e00f435e29b352b`; manifest digest=`dd37cbc05d92d36d3260c1b22ccdde9688be2ffcdf0a6c39e8dd11df246e5734`.

Length 415 bytes; SHA256(raw bytes) `f69fd1fbdc56143b27f32fca03624febc2a57d70e22759c60b0e755ddbe7dc1c`.

```text
4e594647010018006e796f6e2e66756c6c7363616c652d67616c6178792e763119006e796f6e2e66756c6c7363616c65
2d73746f726167652e7631790000004e595347010015006e796f6e2e7365656465642d67616c6178792e763001000000
000000000000010002000200040001000000790300007101000000000000010000000100000000000000000000000000
040002000000010000000100157c4a7fb979379e0400010000000100000000020000000200000000dd37cbc05d92d36d
3260c1b22ccdde9688be2ffcdf0a6c39e8dd11df246e5734220040000000000200001000000004000000200000000010
000040000000004000000001000000900000400200000010000040000000410000000008000000010000000800000001
0000001000000010000000010000000800000008000000040000000400004000000040000000ffff0100000002000000
00040000000240420f000010000001000000000000000000000000000000000000000000000000000000000100000000
00000001000000000000000100000000000000000000000000000000000000
```

### Genesis receipt

Initial state digest=`f90ea7fcfc5e454060c964da15e0652b009007e24547e70471009404bb42228c`; committed genesis history head=`9fbaf6bed116285b03e348fbf43c0250a4f8ad4a694732a6df28cf96aedbc71e`.

Length 136 bytes; SHA256(raw bytes) `74f3bff61d13c0b1dc82b857ed2c5c5db2bd059fff48882a0a629bd1a15fde8a`.

```text
840000004e594f4e2d4653312d524543454950540000000000000000000000000000000000000200424f312ec4c15049
fbb2ad8a4d5b83c3d25daa287b47ed081e00f435e29b352bdd37cbc05d92d36d3260c1b22ccdde9688be2ffcdf0a6c39
e8dd11df246e5734f90ea7fcfc5e454060c964da15e0652b009007e24547e70471009404bb42228c
```

### Empty first tick

Creator/null policy; empty input; all enrolled worlds still run. Final state digest=`cad825f0a95149cfc039135851ee8e85e0a873c6dd1c91da7e31fb32270da1ea`; history head=`df7a2ca9c0adf2c5f7ce0488c9350cf0de9060cebcc95083a63d830f936735e2`.

Length 167 bytes; SHA256(raw bytes) `7acf5b31d277556770ea892182fedc9a37878dcf830cceb6b4de66d44ffd7cf1`.

```text
390000004e594f4e2d4653312d5245434549505401010000000000000000000000010000000002000000000000000000
00000000000000000000000000660000004e594f4e2d4653312d524543454950540b0100000000000000010000000700
000000020001000000000000000000000000000000000000000200000000000000000000000000cad825f0a95149cfc0
39135851ee8e85e0a873c6dd1c91da7e31fb32270da1ea
```

### Canceled first tick

Begin the empty first tick above, stop after any candidate event/COW/index step,
and cancel. Published additional stream is the empty byte string (length0).
Genesis history head remains `9fbaf6bed116285b03e348fbf43c0250a4f8ad4a694732a6df28cf96aedbc71e`; state digest remains
`f90ea7fcfc5e454060c964da15e0652b009007e24547e70471009404bb42228c`; policy remains absent. Retry must produce
exactly the Empty first tick bytes above. No cancellation receipt is canonical.
This is a required lifecycle witness, not a claim a cancellation test has run.

### Policy change on empty second tick

From the committed empty first tick, choose ruler/civilization1. Previous_policy is present creator/null. State digest=`e3e7e5eb9a973da3c3271952349d9878aa7918c6388cfc9def84268a50b7e005`; history head=`cf1d939d65c665f72a67429e7b4891505edfa87b2b4183db191c0b2ad0a7bb46`.

Length 173 bytes; SHA256(raw bytes) `8170557ad03b537c889cbea78fb35cbfc742c1b9cde461b2501665de48610314`.

```text
3f0000004e594f4e2d4653312d5245434549505401020000000000000000000000010000000002000100000000000000
00000000000000000100000101010000000000660000004e594f4e2d4653312d524543454950540b0200000000000000
010000000700000000020002000000000000000000000000000000000000000200000000000000000000000000e3e7e5
eb9a973da3c3271952349d9878aa7918c6388cfc9def84268a50b7e005
```

### Same local settlement ID on two worlds

Separate run from genesis: sequence1 authors settlement1 on world1 for civilization1; sequence2 authors settlement1 on world2 for civilization2. Each hall is cell1, population2,food8,material20; supplied events leave food6,fed_streak1,housing4. Final state digest=`6e8a05de1fce34d3db097b580e2f42e8134d3b613cfc36dfca43dff33a6c8e1d`; history head=`fd85bcf37b0a432dccc9c7a135f483d9bd744ed7ea3616b21156215e92dc48a2`.

Length 583 bytes; SHA256(raw bytes) `ce79a77ea3ff06dc6744a1d95d20c92d91efd8624c4eee18ecba8124c59f2f92`.

```text
390000004e594f4e2d4653312d5245434549505401010000000000000000000000010000000002000000000000000000
00000000000000000000000200530000004e594f4e2d4653312d52454345495054020100000000000000010000000101
000000000101000000000000002500010000000000000000000001000000010000000100000000000000020800000014
00000001530000004e594f4e2d4653312d52454345495054020100000000000000020000000102000000000102000000
000000002500020000000000000000000002000000020000000100000000000000020800000014000000013e0000004e
594f4e2d4653312d52454345495054030100000000000000030000000201000000000101000000000000000001000000
000000000800000014000000023e0000004e594f4e2d4653312d52454345495054030100000000000000040000000202
00000000010200000000000000000100000000000000080000001400000002330000004e594f4e2d4653312d52454345
4950540901000000000000000500000006010000000200010000000000000002020006000000330000004e594f4e2d46
53312d524543454950540901000000000000000600000006020000000200010000000000000002020006000000660000
004e594f4e2d4653312d524543454950540b010000000000000007000000070000000002000100000000000000020000
00000000000200000002000200000000000000000000006e8a05de1fce34d3db097b580e2f42e8134d3b613cfc36dfca
43dff33a6c8e1d
```

### Cross-world last global building quota: ordered fragment

This is an exact four-record fragment (ordinals1..4), not a complete replay
transcript. Precondition at tick99: global buildings16383, world1 has one legal
building and local next_building2; worlds1/2 each own settlement1, hall0 and vacant
adjacent cell1; other worlds hold the remaining legal buildings; creator/null
policy and previous_sequence99999. At tick100, sequence100000 routes to world2,
sequence100001 to world1; both request author_house at cell1 with explicit bypass.
Inputs emit in sequence order, then world1 accepts building2 (waiver8,ticks2),
world2 rejects GlobalCapacity38/quota_kind2. This fragment uses a separate
64-system/512-world/16-civilization manifest; it does not use the two-world
genesis fixture above. The fragment proves wire ordering
and quota outcome bytes. A full64/512/16 replay fixture with the complete setup
history remains required implementation qualification; its missing execution is
not disguised as a complete campaign golden.

Length 296 bytes; SHA256(raw bytes) `1f5fc797204fcadd25d3038ab405cde15bf44a73ccae3cad609345f8ea28cbb2`.

```text
500000004e594f4e2d4653312d524543454950540264000000000000000100000001020000000001a086010000000000
2200a0860100000000000000020100000000000000020200000001000000000000000001500000004e594f4e2d465331
2d524543454950540264000000000000000200000001010000000001a1860100000000002200a1860100000000000000
020100000000000000020100000001000000000000000001480000004e594f4e2d4653312d5245434549505403640000
00000000000300000002010000000001a186010000000000020200000000000000010001000000010000000000000008
00000002300000004e594f4e2d4653312d524543454950540464000000000000000400000002020000000001a0860100
0000000026000002
```
