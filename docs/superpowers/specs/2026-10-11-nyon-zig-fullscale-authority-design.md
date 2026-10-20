# NYON Zig: full-scale shared authority storage

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Historical; Zig continuation canceled 2026-10-19.** Preserve this source contract and evidence. Active recovery targets Rust stable1.99.1; see the 2026-10-19 Rust designs/plans. Historical review labels do not approve new Rust behavior. Tests and CI gates remain paused.
<!-- END 2026-10-19 rewrite decision -->


Date: 2026-10-11. Author: Abbey.
Status: **Proposed — decisions selected under delegated authority; independent
technical review and controller approval of the reviewed revision are required.**
Compiler: exactly `0.18.0-dev.146+35accc06e`.
Proposed storage profile: `nyon.fullscale-storage.v1`.
Proposed integrated rules identity: `nyon.fullscale-galaxy.v1`.

## Decision and boundary

Use one owner, one clock, one globally published descriptor and one shared,
preallocated copy-on-write page pool for all enrolled worlds. Store only occupied
entity records and sparse cell claims. Derive immutable terrain from manifest
seed and qualified coordinates. Share history, command staging, replay scratch
and allocation accounting globally; do not instantiate 512 island reservations.
Every world participates in every phase, even when empty or unselected.

This document fixes memory, ownership, capacities, ordering and failure behavior.
It authorizes no new city recipe, research rule, diplomacy outcome, fleet battle
or tactical substep. Those systems remain required by the expanded roadmap and
need reviewed semantic extensions. Reserved strategic rows below are a storage
admission contract, not invented gameplay schemas or implemented records.
The first implementation slice uses the existing local-economy semantics under
the new identity and binds the seeded manifest; later mechanics require a reviewed
rules identity/version extension. Do not ship arbitrary opaque strategic records.

The delegated authority record permits controller design decisions and qualified
Rust replacement, while preserving independent review and old data. No source,
build, old specification, instruction file or user profile changes accompany
this proposal. [E1, E2]

## Current evidence and scale problem

`planet_economy.state.Reservation` is exactly 8,642,560 requested bytes: a
4,096-slot pool with 256-byte payload plus 16-byte slot metadata; two 2,048-entry
u64 directories; fixed history/staging, command, scratch and control blocks.
It reserves 1,216 addressed pages, including terrain and dense claims. The source
asserts the offsets and sizes. [E3] Thus 512 reservations alone request
`512 * 8,642,560 = 4,424,990,720` bytes. This is allocation request arithmetic,
not measured resident memory. Coordinated v0 separately reserves 12,582,912 bytes
and admits four planets, maximum 47,153,152 bytes total. [E4]

The approved coordinator supplies phased preparation, all-world barriers,
immutable descriptor leases, one publication point and bounded cleanup. Its
shared `AlgorithmContext` already supplies planet and civilization identity to
pure local rules. These are reuse seams; its four-world storage and wire remain
unchanged. [E5, E6] The seeded design requires 64 systems, 512 planets and 16
civilizations, explicitly refusing to treat metadata generation as simulation
capacity. [E7]

Retained strategic qualification targets include 2,048 industries/routes, 4,096
freight shipments, 256 fleets and 2,048 hulls, plus 36,000 mixed and 100,000
endurance ticks. Interpret industries/routes conservatively as capacity for
2,048 of each simultaneously. These are required witnesses, not achieved
measurements or imported old gameplay rules. [E8]

## Capacity profile

A full-profile authority owns the reservation below even for a small manifest.
Lower-capacity product profiles require separately identified, reviewed profiles;
no adaptive cap based on hardware or free memory enters canonical rules.
All configured quotas are serialized in genesis. Counts and byte calculations
use checked u64 arithmetic before conversion to allocation/index widths.

| Dimension | Full profile admission |
| --- | --- |
| Systems / worlds / civilizations | 1..64 / 1..512 / 1..16; manifest relationships and IDs unchanged |
| Surface side | 4..32; at most 6,144 logical cells/world, 3,145,728 total |
| Settlements | 4,096 global, at most 64/world |
| Local buildings/projects | 16,384 global, at most 256/world; unfinished and finished share quota |
| Occupied cell claims | 36,864 global, at most 576/world and actual cell count |
| Local freight shipments | 4,096 global, at most 64/world |
| Local path cells | 65/shipment, stored as up to three 256-byte pages of u64 local cells |
| Strategic industries | Reserved 2,048 global, at most 256/world |
| Strategic routes | Reserved 2,048 global, at most 256 originating at one world |
| Strategic freight | Reserved 4,096 global, at most 4,096 attributed to one world |
| Fleets / hulls | Reserved 256 / 2,048 global; either may concentrate at one world |
| Civilization/research/diplomacy extension metadata | Reserved 2,048 records global; row schemas need semantic review |
| World / civilization mutable metadata | At most 512 / 16 records |
| Public commands | At most 1,024/tick, body at most 1,024 bytes, at most 64 local commands/world |
| Leases / jobs | 64 global leases; one mutation lane through cleanup; one replay target/session |
| Clock / budget | One hour/tick, horizon 1,000,000; allowance 0..4,096, host default 256 |

Local and strategic freight quotas are separate, so the later strategic system
can meet its 4,096-shipment fixture even when local freight is full. A cargo
transfer between domains must eventually move one logical cargo record with
conservation receipts; it must not silently count the same cargo twice. No
interplanetary transfer command is introduced here.

The claim cap follows current maximum footprints: one hall/settlement plus at
most two cells/building gives `4,096 + 2*16,384 = 36,864`; per-world bound is
`64 + 2*256 = 576`. Future terrain edits, roads, cities or tactical occupancy
cannot consume unaccounted pages or reinterpret claim records. Their reviewed
extension must partition remaining capacity or choose a new storage profile.
The 65-cell local path is the existing codec bound, not a strategic route rule.
[E6, E9]

Global quotas constrain sums; every world is not promised its local maximum
simultaneously. The API exposes both remaining global and per-world quotas.
There is no arbitrary first-world reservation or eviction: canonical command
order determines who consumes remaining capacity. A lower numeric PlanetId is
not allowed to steal quota through an uncharged prepass.

## Logical indexing and sparse storage

Use realm-qualified keys `{realm:u32, kind:u16, local:u64}`. Realm zero denotes
galaxy-wide entities, realm 1..512 denotes the corresponding PlanetId. Kind tags
are explicit, versioned and distinct across settlement, building, cell claim,
local shipment, world metadata and future strategic entity families. Public
references retain their typed realm and kind; a local number never resolves
across worlds or entity kinds. Civilization and system IDs retain separate types.

Each root owns a sorted array of at most 131,072 index rows. A row has explicit
24-byte storage layout: realm:u32, kind:u16, reserved:u16=0, local:u64,
page:u32, reserved:u32=0. The comparator is numerical `(realm,kind,local)`;
wire encoding is independent of this host layout. Rows are compact, contain no
holes, and only occupied records get rows/pages. Binary searches, insertion,
removal and moves are cursor operations charged per comparison or 24-byte move;
no unmetered array sort or memmove. Insertions can be linear work: no logarithmic
mutation or tick-latency claim is made. Stable IDs never depend on row positions.
Deletion compacts an index but never renumbers surviving entities.

One logical record uses one 256-byte payload page, with explicit field encoders
and zeroed unused bytes. A shipment record additionally owns up to three path
pages; those pages contain local coordinates, not duplicate cell entity records.
Their handles are private storage references, excluded from wire/digests. Parent
and child pages are each retained/released exactly once per owning root through
a resumable traversal. Ordinary keys use checked monotonic local u64 counters;
cell claim keys use the immutable local cell coordinate. Removing/reclaiming a
claim permits another claim at the same cell because the coordinate identifies
the cell, not a never-reused settlement. Any future cross-world movable entity
uses a stable realm-zero ID and a mutable location, rather than changing identity.

Terrain has no page or index row: use `rules.terrain(planet_seed, qualified_cell)`
with the seeded manifest formula and existing cube topology. Sparse missing
claim means unoccupied; it does not mean unknown terrain or an unloaded world.
Queries validate realm and cell before derivation. Immutable manifest metadata is
stored once and referenced by both roots. Presentation meshes/caches live outside
authority accounting and never replace canonical terrain. [E6, E7]

The declared record maxima total 74,512 rows. Local routes add at most
`4,096 * ceil(65*8/256) = 12,288` pages, producing 86,800 occupied pages at these
storage envelopes. Strategic reserved rows count at most 256 bytes each; any
future schema needing more must explicitly partition remaining pages or revise
the profile. This arithmetic proves an envelope, not that unspecified strategic
schemas fit. Before their implementation, compile-time layout and independent
maximum-record encoding witnesses must prove the envelope or reject the design.

Each root may own at most 131,071 pages, leaving 44,271 pages above the declared
86,800-page envelope for reviewed extensions. That headroom is unavailable to
unreviewed mechanics. The shared pool has 262,144 slots: two maximally distinct
roots plus two slots for one in-progress COW/new-record transition. Only one such
transition may own unattached pages at once; multi-page writes link each page
into the unpublished candidate before allocating the next. Removed/replaced
candidate pages are released before another transition starts. Requiring more
simultaneous detached pages is an invariant error, not an overflow allocation.

## Resumable index and child ownership (F1)

The compact/sorted invariant applies at **stable operation boundaries**. During
one mutation, controls store `extent:u32`, `hole:?u32`, `pending_row:?Row`,
`pending_child_mask:u8`, `release_child_cursor:u8`, and two transition handles
`new_ref/old_ref` with explicit ownership bits. Logical count is published only
when the operation stabilizes. Candidate-owned pages include pending rows and
linked children; these are part of its 131,071-page bound, not extra detached
physical pages. Raw bytes in a hole or outside extent own nothing. No code may
infer reference ownership from the presence of nonzero bytes alone.

For insertion into n rows: preflight n+1 and page quotas; create the new parent
as the candidate-owned pending row, with zero initialized child handles. Each
child allocation is initialized and linked before another allocation starts.
An allocation not yet linked occupies `new_ref`; linking transfers ownership
without a retain. Set extent=n+1 and hole=n. Repeatedly move row[hole-1] into the
hole, and move the hole left. One charged move copies 24 bytes and updates the
hole in the same bounded operation; the source becomes non-owning even if stale
bytes remain. This is a transfer, never a retain. At the insertion position,
transfer pending_row into the hole, clear its ownership bit and hole, and publish
the new candidate logical count in one charged bounded operation. At every stop,
exactly the rows in `[0,extent)` except the hole, plus pending_row, own refs.
For `[A,B,C]`, the interrupted state is `[A,B,hole,C]`, never two owning Bs.

Deletion transfers the victim row into pending_row and marks its slot as hole
in one unit. Release its child ownership one child/unit, then parent one/unit;
the release cursor/mask changes with each decrement, never before or after a
separate yield. Clear pending ownership after its last release. Shift each next
row left into the hole with ownership transfer, then shrink extent when the hole
reaches the end. Replacing a key is deletion followed by insertion, with no
nested shift or second pending row; initial economy never needs mutable keys.
Cleanup need not repair sorting: enumerate `[0,extent)` excluding the hole, then
pending_row, then owned transition handles, using their explicit ownership bits.
Each enumerated row is marked non-owning atomically with its parent release.
An abort during that traversal resumes from the exact child cursor and mask.

Root preparation has separate `source_row`, `parent_retained`, and a three-bit
`children_retained` mask. Retain the parent into pending_row (one unit); retain
its existing children in ordinal order, updating one mask bit with each retain.
Only after all child retains does one unit append/move this row into the prepared
candidate prefix, transferring all ownership and clearing pending controls.
An abort releases only mask-selected children and the parent if retained. It
never follows unretained child references just because a shared parent names them.
The untouched source suffix is not candidate-owned. For later stable candidate
rows, all referenced children are owned; for the pending preparation row only
the mask determines ownership. These modes are explicit, not guessed from bytes.

Parent COW allocates/copies a new parent but **does not retain its children**:
the candidate already owns one child reference apiece. Swapping parent handles
transfers that same child ownership to the new parent; release the old parent
without traversing children. Child COW first makes its parent private, drains
that transition, then allocates/copies one child, swaps its handle, and releases
the old child. A new/ref-replaced page can occupy `new_ref`, and the superseded
reference can occupy `old_ref`; no next allocation starts until both ownership
bits are drained. Child changes cannot overlap parent COW. At most two detached
physical slots exist, and reference transitions never create a third root owner.
A pending deletion can still own a parent and three children, but all count in
the candidate's root page total until released. The global bound remains two
131,071-page root ownership sets plus two transition slots, with sharing reducing
physical use. Controls and masks fit within the existing 2 MiB block subject to
the required layout witness; no new allocation is authorized.

All transfer/retain/link/unlink/release actions require one shared work unit and
complete their bounded handle/ownership updates before yielding. Budget zero
changes none of them. Required cut-point tests interrupt after every action,
including abort while aborting, full roots, root preparation with 0..3 children,
insertion at first/middle/last positions, and child COW following parent COW.
These are specified proof obligations; they have not been executed.

## Requested memory accounting

Use separate exactly sized allocations with explicit owned-allocation bits,
and offsets aligned within each block. PageSlot is an extern 272-byte layout:
256 payload, references:u32, next_free:u32, flags:u32, reserved:u32. Private
handles are one-based u32 indexes; zero is absent. Refcounts are checked and
bounded by two roots; leases pin a root, not individual extra page references.

| Allocation | Calculation | Requested bytes |
| --- | --- | ---: |
| Shared page slots | 262,144 * 272 | 71,303,168 |
| Two root index arrays | 2 * 131,072 * 24 | 6,291,456 |
| Committed history segment cache | 64 * 1 MiB | 67,108,864 |
| One candidate history stream | 32 * 1 MiB | 33,554,432 |
| Shared command/replay/validation/codec scratch | Fixed 8 MiB | 8,388,608 |
| Both descriptors, per-world counters, lengths, leases, jobs, segment metadata and controls | Fixed 2 MiB | 2,097,152 |
| Owned immutable seeded manifest, including alignment/control | Fixed 64 KiB | 65,536 |
| **One authority including both roots** | **Sum** | **188,809,216 (180.0625 MiB)** |
| **Source plus one replay/replacement target** | **2 * sum** | **377,618,432 (360.125 MiB)** |

These are chosen exact reservations, not empirical `@sizeOf` findings for new
code. Before implementation-plan approval, a Zig layout witness must instantiate
all proposed controls, descriptors, indexes and cursor storage under the pinned
compiler and prove fit/offset/alignment, including synchronous command snapshot
metadata. Refuse an overflowing design; do not silently raise these blocks.
No controls, per-world scratch, transcript copy, third root, command allocation,
query cache or hidden source clone may sit outside this accounting. Small owner
handles also occupy the control allowance and are included in the witness.

Normal ticking, maximally dirty candidates, abort and old-root retirement use
the same reservation. A replay/import takes a separate whole authority; source
leases can keep its old root alive, already counted. No second detached target
or concurrent comparison authority is admitted. The target installs by owner
transfer and the obsolete source is then drained/destroyed before another target
starts. The manifest is owned by each authority; a setup builder retained during
handoff is additional ownership and must be drained first or separately charged.

The session budget for these two authorities is 384 MiB (402,653,184 bytes), with
25,034,752 bytes margin. Store/transfer/UI/GPU allocations are separate reported
owners. The existing durable draft allows 128 MiB store/transfer transient bytes;
combining its full allowance with source+target is 511,836,160 bytes before host
memory. This is not a process RSS promise or a browser capacity qualification.
Allocator overhead, committed physical pages and process memory require named-host
measurement. [E10]

## One global transaction

One owner/thread executes all calls without callbacks/reentrancy. Threaded hosts
marshal to that owner. One global tick/revision/policy/sequence watermark and two
generational descriptors replace independently owned planet active markers.
A descriptor points to one whole root plus the committed history prefix, global
state digest, manifest digest and tick metadata. Its per-world metadata is built
incrementally before publication; all enrolled worlds report the same tick and
revision. No query reads a candidate or internal cursor.

Reuse coordinated v0 phase boundaries: prepare/route, commands, arrivals,
construction, production, population/supply, validation/history, prepared barrier,
publication, retirement. A phase visits every PlanetId ascending, including empty
worlds, before the next phase begins. Within each world use local numeric entity
ID order; production remains settlement then building order. Empty worlds take
an explicit charged phase-entry step. No selection, loaded status, display
visibility, packet cadence or worker timing controls simulation participation.
Future strategic phases require their own reviewed insertion order and semantics.
[E5]

Public command sequence is globally unique, positive and increasing beyond the
published watermark. Incrementally sort the whole input by sequence and validate
routing before gameplay. In this initial local-economy identity, commands execute
by ascending world then sequence as in coordinated v0; global quota contention
therefore uses that specified order, not arrival/allocator order. This ordering
must be included in fixtures because cross-world quota sharing makes it observable.
Realm-zero commands are not yet admitted. Their future phase/order must be reviewed.

Begin admits counts and snapshots at most 1,024*(1,024+4)=1,052,672 input bytes;
this bounded copy is the sole synchronous begin exception. Body lengths and
command metadata occupy the scratch/control blocks. Sorting, routing, policy and
structural checks are polled. Before mutation, reserve history space and candidate
counters. Each accepted command preflights all logical quotas, IDs, path storage
and resources; on gameplay rejection it changes none of them. Global and local
quota failures produce distinct `GlobalCapacity` and `PlanetCapacity` receipts.
Published rejections consume sequence; whole-tick errors/cancel do not.

Prepare a candidate by copying occupied index rows and retaining their pages,
all charged. COW mutation never changes a page reachable through the committed
root. The barrier verifies all worlds, counts, references, aggregate capacities,
history bytes/digests and descriptor generations. One work unit switches the
committed descriptor index. Publication cannot allocate, hash, validate, traverse
worlds or fail. This shared-root implementation needs no per-world active-marker
installation; the same globally atomic visibility contract is preserved.

A view token carries owner namespace, descriptor slot/generation and lease
slot/generation. Public queries return copies of at most 64 bounded records.
Any lookup work requiring a long scan uses a polled query cursor, not an unlimited
synchronous call. Tokens never expose pool handles or pointers. Prepublication
leases continue observing the complete old root after publication. Old leases
block retirement and the next mutation lane, not the current publication; new
leases do not block old-root cleanup. Generation overflow never wraps.

## Global history and endurance boundary

Use one globally ordered complete transcript, not 512 independently reserved
receipt arrays. Local event receipts carry realm, source phase and entity identity.
There is one tick input and one global commit per tick; no mandatory 512-copy
per-tick snapshot list. Every local event, including gameplay rejection, is still
encoded and hashed. World views filter the complete stream through bounded
queries; filtering does not create an unaccounted secondary transcript.

History byte capacity is shared: candidate stream at most 32 MiB/tick, individual
receipt bytes at most 2,048 before their four-byte frame, command body at most 1,024. The normative companion
[initial economy codec](2026-10-11-nyon-zig-fullscale-authority-codec.md) derives
47,106 records and 4,057,265 framed bytes as a conservative maximum initial tick.
Reserve that fixed byte bound before each initial-rules tick; it fits the 32 MiB
candidate stream. This is a source-derived arithmetic proof, not implemented
maximum-fixture evidence. Future rules must supply their own reviewed event/byte
bound within 32 MiB or choose a different profile. Truncation and valid-command
rejection to conceal insufficient history are forbidden.

A finite 64 MiB resident transcript cannot promise arbitrary 100,000-tick histories
with full command/event load. Select segmented complete history as a required
session extension: immutable 1 MiB segments, monotonically increasing u64 segment
ordinal, length and SHA256; records may cross segments and commit marks the exact
visible prefix. Byte offsets/counts in global history are u64. Segment boundaries,
cache eviction, host acknowledgments and export schedules do not enter logical
event bytes or the history chain; their digests validate transport only.

Before begin, reserve enough currently unused cache slots for the complete
worst-case next tick. Candidate staging is copied into those slots incrementally;
all tails remain invisible until descriptor publication. Cancellation discards
unpublished tails. The current partial tail remains resident; append may write beyond its old
visible prefix but never changes an old visible byte or prefix length. At most
32 new segment slots are needed for a 32 MiB tick in addition to that resident
tail, so admission reserves32 free slots (or the smaller exact initial bound).
No unsealed current tail is evicted. With insufficient free space return `HistoryBackpressure` before beginning,
leaving clock, sequences and state unchanged. A partially filled segment that was
visible to an old descriptor cannot be overwritten; its old prefix stays readable.

A sealed segment may leave the resident cache only after a session-owned archive
sink acknowledges the exact identity/ordinal/length/digest as durably retained
and no view/export/replay cursor pins its bytes. Pure authority performs no I/O:
the host provides typed completion facts; it cannot invent successful durability.
No acknowledgment deletes logical history. An unavailable sink returns explicit
backpressure. In-memory-only hosts therefore stop at their finite history bound.
Export/replay requests for evicted bytes enter a bounded source-provider lane,
validate returned identity/digest and consume metered input; absence/corruption
fails without replacing resident state. No synchronous blocking storage read is
hidden inside a value query. A full-history lease pins only its current bounded
segment window; the durable session owns lifetime of the full immutable stream.

### Bounded provider and retention protocol (F2)

Define pure value types, with no store/OS/browser imports:
`Prefix={genesis_digest:[32]u8,tick:u64,byte_count:u64,history_head:[32]u8,
transport_root:[32]u8}` and `Ticket={owner_namespace:u64,owner_generation:u64,
job_id:u64,request_id:u64}`. Each counter is checked and never reused. Prefix
identity is `SHA256("NYON-FS1-PREFIX" || fields in the displayed order)`.
This is transport metadata, excluded from canonical gameplay bytes. An immutable
retention token includes Ticket, exact Prefix identity and an opaque u64 host pin
ID; matching a random opaque ID alone conveys no authority. The authority accepts
only responses to its own outstanding exact request and prefix.

The unbounded durable segment index belongs to the session. Authority keeps two
descriptor-owned transport contexts, each with a 32-level append frontier and
current-tail hash state as specified below. At most one 32-sibling verification
path per source lane occupies the existing 8 MiB scratch.
Segments use ordinal <2^32 and 1 MiB payload except the final partial segment.
Transport tree has 2^32 leaves. Leaf hash is
`H("NYON-FS1-SEG" || genesis_digest || ordinal:u64 || length:u32 || H(bytes))`.
Empty leaf is `H("NYON-FS1-EMPTY")`; internal node at height h=1..32 is
`H("NYON-FS1-NODE" || h:u8 || left_hash || right_hash)`; empty subtrees recur
by this same rule. Appending sealed leaves updates only the candidate context's binary frontier
peaks; fold those candidate peaks with its current partial leaf and empty suffix
to produce its exact prefix root. A partial tail is never added twice. Each hash block/fold is metered.
The tree/index may be persisted by any adapter; this fixes only its authenticated
content, not physical object storage. A prefix's root is calculated locally from
its committed bytes and is trusted because the descriptor owns that prefix,
not because a provider sent a matching digest next to arbitrary bytes.

#### Descriptor-owned transport transaction state

There are exactly two `TransportContext` slots, selected by the same descriptor
slot/generation as the two authority roots. Each owns sealed_leaf_count:u64,
frontier_occupied:u32, frontier_peaks:[32][32]u8, tail_ordinal:u64,
tail_length:u32, an incremental tail SHA256 context, byte_count:u64,
prepared_transport_root:[32]u8, and prefix metadata. Each also owns its pending
leaf/carry/fold hashes, SHA block context, level/index/copy cursors and valid/ready
flags. A cached height32 root handles the full-tree case without an extra frontier
peak; count=2^32 admits no further leaf or partial tail. Counts and increments
are checked. Internal SHA state is scheduling metadata and never canonical wire.
The committed slot is immutable throughout candidate work, including its tail
hash buffer, byte counter, occupancy bits and intermediate-free ready state.

Reserve **4,096 bytes per slot, 8,192 total inside the existing 2,097,152-byte
control block**, leaving 2,088,960 control bytes for all other listed controls.
Each slot's 1,024-byte peak array, explicit scalar/hash fields, <=256-byte tail
SHA context and <=1,024-byte carry/fold/copy state must fit that fixed envelope.
The pinned-compiler layout witness must prove the chosen hash implementation's
context size/alignment and all fields' actual offsets; these are selected bounds,
not measured Zig layouts. If it does not fit, reject the representation before
implementation approval rather than allocate overflow storage. Source proof/input
windows remain in the existing scratch block; no third transport context or
history rescan is permitted. The seven allocations and all requested totals stay
unchanged. Stale bytes in an invalid slot are not an append state.

Creation initializes both slots with metered <=256-byte writes before readiness;
only the genesis descriptor's slot becomes valid. To begin a tick, the obsolete
slot must already have retired. The prepare cursor reserves its new generation,
sets it invalid, then copies the committed ready context into it in at most
16 charged <=256-byte chunks. `transport_copy_offset` is outside the destination
range being copied, in ordinary lane controls. After the last chunk, one charged
bounded transition binds the candidate generation, resets its operation cursors,
and marks it valid but not ready. No candidate hash/append uses a partial copy.
Budget zero neither copies nor changes the flags/cursor. This copy occurs during
polled preparation, not the synchronous command snapshot exception.

Append, tail hashing, seal, binary carry and final fold mutate only that valid
candidate slot. The stored tail SHA context always represents the **unfinalized**
bytes through tail_length so a committed partial tail can resume appending.
Computing H(tail_bytes) copies that <=256-byte context into the slot's temporary
hash workspace in one charged unit, then finalizes the copy with charged padding
blocks; it never pads or consumes the appendable tail context. A zero-length tail
contributes no partial leaf. Sealing its current tail appends one leaf into its
own frontier, then initializes only its own next-tail context. A carry can clear/overwrite any
candidate peak without an undo log because all committed peaks remain intact.
Leaf creation, SHA compression, each peak/occupancy transition, tail reset and
each fold advance consume shared units; a hash compression consumes one unit and
records its new hash/cursor together before yielding. No loop over levels hides
inside publication. After all bytes, carries, folds and global checks finish,
one charged bounded transition marks the candidate context ready. The prepared
barrier requires its byte count, tail length/ordinal and root to equal the
candidate Prefix/descriptor, and no in-flight transport hash operation remains.
The single existing descriptor-index switch then selects **both** gameplay root
and already-prepared transport context. It performs no context copying, hashing,
folding, allocation, provider request or fallible operation.

Cache ownership follows context visibility. Candidate writes may append beyond
the committed tail's visible prefix, but cannot change committed bytes or its
stored tail length/hash state. A tail sealed only by the candidate remains
unsealed from the committed context's perspective, so it cannot be evicted or
repurposed before publication. Newly sealed candidate segments and their prefix
are not eligible for durable-prefix acknowledgment or eviction before publication.
Cache metadata distinguishes committed and candidate reachability and preserves
both visible lengths where the same physical tail slot is shared. After the
switch, the new descriptor's context alone defines current append state; an old
lease still resolves its old prefix/context and pins the necessary resident
window or durable closure. No next candidate starts until obsolete ownership
and context retirement drain.

Abort first closes candidate transport advancement, then discards only candidate
cache ownership/invisible tails with its existing metered cache cursor, and
invalidates/zeros the copied candidate-context range in <=256-byte charged
chunks. A partial prepare copy uses its recorded copied range; a fully valid
candidate zeros its entire 4,096-byte slot (16 units). One final charged bounded
transition clears candidate generation/valid/ready ownership after those steps.
This does not write, reconstruct or invert the committed frontier, does not read
any formerly evicted segment, and needs no provider or sink availability. Retry
copies the unchanged committed ready context and must produce identical roots,
receipt bytes and digests. Old bytes beyond its visible tail length are ignored
and overwritten before future exposure, never treated as committed input.

After publication, old-root retirement also drains old-context ownership once
old leases/pins transfer or release; it clears that context in charged chunks.
It never rebuilds the selected context. Normal shutdown/cancellation drains local
context/copy/hash cursors and cache ownership before destruction. Fault shutdown
instead invalidates local admission, waits for local leases and transfers the
bounded provider dispositions to the broker, then frees the owned blocks without
requiring damaged context traversal. Delayed provider I/O is broker-owned and
cannot mutate either context after namespace closure.

Required interruption witness: commit seven sealed leaves and a partial tail,
evict previously sealed bytes under a valid retained prefix, then have a candidate
seal that tail and another full segment. Cancel after each seal, carry overwrite,
fold, copy chunk and ready transition, including zero-budget observations. The
committed seven-leaf frontier/tail must remain byte-identical, cancellation must
finish with the provider unavailable, and retry must match uninterrupted transport
root and canonical stream exactly. Also interrupt after publication to prove only
old context retirement occurs. These are mandatory implementation tests, not
claimed execution evidence.

`requestPin(prefix)` returns an outstanding Ticket; `completePin(ticket,token)`
is accepted only for that exact prefix after the adapter's durable closure exists.
The token retains **all** segments/index nodes through that exact prefix, including
future windows of an export/replay. Branch delete, undo, replacement and GC cannot
release its closure. An acknowledgment for a segment alone cannot authorize
cache eviction: the committed prefix that contains it must have an owned pin.
The session may coalesce older pins only after all their users transfer/release.
At most two descriptor pins and one replay-source pin are authority-owned;
64 value leases reference these shared pins, not 64 new durable closures. The
session retains a source closure while a new prefix pin is pending; backpressure
bounds this overlap. Authority caches at most 64 segment identities in controls.

`requestSegment(ticket,pin,prefix,ordinal)` has one outstanding source lane per
session/replay pair. Response control header fixes ordinal/length; its proof consists of exactly32
sibling hashes ingested in <=256-byte chunks into the fixed proof scratch area. The authority computes expected length from prefix.byte_count and ordinal,
checks the ordinal range, and authenticates the recomputed leaf through the path
against the **locally retained prefix root**, left/right selected by ordinal bits.
Source bytes arrive via `ingest(ticket,chunk<=256,*WorkBudget)` into a fixed1 MiB
scratch window; it consumes one unit/copy and may refuse with WouldBlock. Hashing,
path checks, output consumption and cursor advancement use the same allowance.
No untrusted chunk is exposed to replay until the full segment verifies. At most
one segment+proof is buffered; two additional <=256-byte request/header values
are control data. No whole transcript or list of all digests is retained.

A provider completion never carries an authority pointer. The session broker
owns its request mailbox and immutable pin records; it calls ingest with copied
Ticket values. Wrong/stale/abandoned Tickets are rejected before copying or
mutation, and provider-owned buffers are disposed by that broker. `cancelSource`
invalidates the active Ticket immediately, clears the bounded local window through
metered cleanup, and emits one cancel/release disposition. Waiting for external I/O
is not needed to free authority memory because the broker cannot dereference it.
The broker retains pins until its already-started work drains; it reports disposal
separately from request cancellation. An unavailable broker does not falsely
report released storage. It keeps one bounded abandoned ticket/pin set, blocks a
replacement source request until reclaimed, and exposes ProviderUnavailable.

`takeShutdownDispositions(out[4])` transfers remaining ownership tokens exactly
once to the session broker (two descriptors, one replay-source, one active request);
after local window/cursors drain the authority may free its seven blocks. The
broker becomes the sole owner of delayed releases and cannot callback into the
closed namespace. A host unable to accept the bounded disposition set receives
Busy; no tokens or allocations are lost. Fault shutdown uses the same transfer
without traversing corrupt page refs. Host orphan/pin counts are part of its
existing bounded store/transfer ownership accounting, not free authority memory.

A replay first pins the complete immutable source Prefix. Its target regenerates
canonical bytes and compares them byte-for-byte with authenticated source windows,
including inputs, events and commits. Target output is **not archived again**.
Verified target cache segments may be discarded using a borrowed reference to
that still-owned source pin. Until byte equality is established they cannot be
evicted. The target tracks its verified byte prefix; a target query is bounded to
its own published byte_count even if its source pin includes later source ticks.
For a target tail ending inside a source segment, authenticate the full source
segment and compare/expose only the allowed prefix. Source transport root validates
that full segment; the target independently derives its own shorter prefix root.
Cancellation releases target plus replay pin; successful take transfers that pin
to the detached target as its history backing. Installation transfers target owner
and pin together, then retires obsolete source owner/pins before another target.
No third authority or output archive is created. A fork requesting new simulation
must first obtain its own exact-prefix pin from the session (same authenticated
bytes, new ownership token); append admission stays Busy until granted. Undo/redo
and branch comparison use the same immutable source pins and one target lane.

### Moving durable-session dependency (M1)

Observed 2026-10-11 during this fix wave: Proposed session design SHA256
`0014afd69b1f33bc83842d46148361ad5e8a421a240250b7dc10dc7131942fda`
now specifies 144 GiB portable containers, 131,072 transaction nodes, 1 MiB/tick,
512 GiB retained namespace and 128 MiB store/transfer transient memory. Its
lines264-268,459-479,584-602 replace the earlier snapshot described in the first
draft. This is a moving **Proposed** dependency, not implemented or approved fact.
The 1 MiB/tick limit is incompatible with this initial profile's 4,057,265-byte
bound and full 32 MiB extension envelope.

Controller decision selected here: revise that still-unimplemented Proposed
session contract before implementation approval, rather than invent a decoder
upgrade to an already shipped version. The compatible session must admit complete
32 MiB transactions through bounded chunks, this exact-prefix pin/provider
protocol and reference-safe deletion/recovery. Its graph/portable/namespace byte
bounds and two-buffer reservation must be recalculated together; 144 GiB does
not imply room for every 32 MiB tick at every node. That separate owner must
review the resulting profile and real100,000-tick retention fixture. No change to
the other specification is made here. If any incompatible format is implemented
or frozen before reconciliation, select a new explicit version instead. No old
archive is widened silently, and backpressure does not waive endurance. [E8,E10]

## Canonical identity, digest and replay

Never encode allocator page indexes, index padding, free-list order, refcounts,
lease counters, scheduling meters or cache addresses. New genesis binds the
exact seeded manifest wire/digest, rules identity, storage profile, all admitted
quotas and initial global role state. The seeded wire identity remains separate;
neither it nor coordinated/planet-economy v0 receives appended fields.

All canonical integers use explicit little-endian widths, enums explicit tags,
optionals presence byte then value, length-prefixed arrays and no trailing bytes.
The normative companion `2026-10-11-nyon-zig-fullscale-authority-codec.md`
fixes every initial local-economy field/tag/order; physical page encoding is not
that codec. New strategic kinds are forbidden under this initial identity. Canonical state traversal is metadata,
then occupied entity keys in `(realm,kind,local)` order. Path coordinates are
encoded in path order inside their owner record. A cell without a claim is
represented by the manifest-derived surface, not millions of empty records.
Counters, policies, inventories and reservations are included; storage scheduling
metadata is excluded. Numeric ordering, not bytewise little-endian ordering,
determines entities. No duplicate key or dangling reference is canonical.

Use domain-separated SHA256: `S = H("NYON-FS1-STATE" || canonical_state)`;
initial history head `H("NYON-FS1-HISTORY")`; each framed receipt updates
`H(previous_head || u32(byte_length) || receipt_bytes)`. Tick commit includes
completed tick, revision, published sequence and S, but S excludes the history
head to avoid recursion. Complete emitted event bytes and input bytes determine
the history chain. Tick/phase/realm/event ordinal use explicit counters with
checked overflow; event ordinal is u32, and phase tags are frozen by rules.
Hash preparation/compression is incremental, at most 64 bytes/unit.

Replay reconstructs a fresh target from exact genesis and complete canonical
inputs, re-executes all phases/worlds and compares every receipt plus each commit
state digest. A snapshot hash or durably acknowledged segment is not proof that
commands were valid. Compare logical records incrementally too, including
coordinates and resource totals. Distinct physical allocation/free-list histories
must yield identical bytes. Target failure/cancellation drains only target owners;
source views and durable branch references remain valid. Budgets, cache/provider
chunk sizes and polling schedules cannot change canonical results.

## Exhaustion, cleanup and meters

Creation allocates the seven declared blocks, recording each success before the
next allocation. Failure frees every acquired block once and publishes nothing.
Initialization and zeroing are polled; all tick/replay/abort operations allocate
zero bytes after setup. `OutOfMemory` is a creation/replacement outcome.
Exceeding configured entity quotas is a deterministic gameplay rejection.
Exhausting the physical pool while all advertised logical/page bounds hold is
`InvariantViolation`, aborting precommit and faulting admission if ownership is
suspect; no overflow allocation or automatic entity eviction is allowed.

Cancellation before publication latches the result and incrementally releases
candidate index references, route children, detached transition refs and invisible
history tails. The lane remains Busy until ownership drains. After publication,
cancel reports `already_published`; only obsolete roots can retire. Zero-budget
poll changes nothing, including terminal-delivery flags. Cleanup reference defects
enter permanent `CleanupFault`, preserve readable committed state and close mutation.
An explicit fault shutdown closes acquisition, waits for local leases and
transfer of the bounded ownership dispositions to the broker, then frees whole
owned blocks from the independent allocation ledger; delayed provider I/O drains
under broker ownership without retaining an authority pointer;
it must not traverse corrupted page metadata. [E5]

Every clock/revision/ID/sequence/generation addition is checked. Preserve v0's
counter distinctions: entity-next-ID exhaustion rejects that command without
resources/ID consumption; job/revision/descriptor exhaustion rejects admission;
sequence max can publish once while subsequent empty ticks remain legal; tick
horizon rejects before mutation; exhausted lease slots are never reused. An
aborted candidate does not consume canonical entity IDs or sequence watermarks.
Realm/namespace recreation uses a fresh owner generation. Scheduling counters
may be consumed by a failed admission and are excluded from canonical digests.

One caller allowance is threaded through routing, index movement, phases,
validation, encoding, hashing, publication, replay source verification and cleanup.
One unit covers at most a 256-byte copy/zero or the bounded 24-byte row/ownership transfer specified above, one <=64-byte hash block, one index
comparison/move, one reference/free-list change, one topology edge or a bounded
single-record operation. Multi-record joins/scans and variable paths have cursors.
No per-world fresh budget exists; free-list work cannot hide a pool-wide scan.

Expose requested bytes by owner; initialized bytes; live/peak pages; committed
and candidate page counts; detached refs; index rows; each global/per-world quota;
COW copies; history resident/staged/retained-durable bytes; segment pin/ack counts;
work by phase; phase/world/entity cursor; snapshot bytes/time; lease blockers;
replay/source wait reason; cleanup state. Meters are diagnostic and excluded from
authority digests. Test counter overflow separately from gameplay behavior.

## Alternatives considered

**A — compact independent per-world stores under one coordinator.** Reduce each
island's history/scratch and allocate a smaller pool/world. This preserves local
ownership boundaries and can reduce contention, but still reserves spare COW
capacity 512 times, needs global quota transfer/accounting, and duplicates
metadata/history plumbing. It also permits heavily unequal world populations to
strand unused memory. Rejected for this full profile; retained v0 remains useful
as the small qualification reference.

**B — sparse B-tree/radix indexes over a shared pool.** Avoid copying/moving a
large flat index and improve asymptotic mutations. Requires bounded split/merge
reserves, independently qualified balancing/canonical iteration, and additional
node/refcount failure paths. It is a plausible later implementation replacement
if measured flat-index work misses requirements; physical index structure is not
canonical. Selected first design is sorted flat root indexes because their exact
maximum bytes and lifecycle are simpler to audit. Bulk maximum-world construction/insertion must be measured against the retained
**full-step p95 <=5 ms** target before implementation acceptance, not merely the
256-unit poll target. Metering never establishes that full-step target. Failure
requires a reviewed chunked/indexed or stable-keyspace replacement with equivalent
ownership and canonical ordering; it cannot be reclassified as acceptable because
each poll is short. Measurements may force a
reviewed algorithm change before delivery; they cannot waive latency targets.

Unloaded-world simulation or multiple unrelated four-world coordinators is not
an alternative satisfying the single-clock/all-world atomic requirement.

## Required review and acceptance evidence

1. Independent review checks quota sums, two-root plus detached-reference proof,
   cleanup at every cursor boundary, old leases during publication, canonical
   cross-world quota ordering and segmented-history ownership.
2. Pinned-compiler layout witness proves exact blocks and control fit. Counting
   allocator verifies seven allocations, every failure cut point, peak requested
   bytes, zero poll allocations and complete frees. Test Debug/ReleaseSafe and
   applicable 32-bit Wasm sizes; no architecture-dependent wire encoding.
3. Storage maximum fixture populates all declared families with schema-qualified
   records (future families only after their semantic contracts), concentrates
   legal worlds, maximally dirties both roots, retains old leases and aborts at
   each COW interval. Test logical exhaustion without physical overflow and exact
   fit/one-short history reservations. Before future schemas exist, their capacity
   proof is storage-envelope evidence only and cannot satisfy gameplay acceptance.
4. Full 64/512/16 fixture demonstrates every world participates in every phase,
   with empty worlds and due activity on unselected worlds. Native and Wasm
   vectors agree across budgets 0/1/7/256/4,096 and randomized schedules.
5. Independent canonical fixtures cover same local IDs on different worlds,
   coordinate claims, cross-world last-quota contention, page allocation order,
   deleted entity rows, global sequence exhaustion, stale tokens and canceled
   first ticks. Existing island/coordinated v0 fixtures remain unchanged.
6. Global history witnesses verify complete events, exact per-phase worst-case
   stream bounds, split records, late/mismatched durability acknowledgment,
   cache pinning, provider corruption, unavailable segments and source retention.
   The reconciled, reviewed durable session contract must pass crash/CAS/export/import/replay and
   36,000/100,000-tick retention fixtures before full-game acceptance.
7. Measure at least 10,000 polls on named hardware: inherited poll target
   p95 <=2 ms/max <=8 ms at budget256, plus retained full-step p95 <=5 ms at
   declared strategic caps. These are separate requirements. Report full-step,
   request-copy, hashing/index/candidate cleanup costs and peak requested/RSS
   separately. No measurement is claimed here; if targets conflict with the
   chosen algorithm, revise/review the implementation before calling it qualified.
8. Run the then-current Rust-free repository gate plus focused new lifecycle,
   allocator and replay tests; preserve independent native/browser/manual and
   platform proof gaps. A documentation draft or a green small fixture does not
   deliver cities, tactics, durable sessions, hosts or full-game scale.

## Evidence references

All references were read from the canonical checkout on 2026-10-11. Source
statements are Current observations; new layouts and interfaces remain Proposed.

- E1: `docs/qualification/2026-10-11-delegated-game-design-authority.md:3-40`
  records delegated decisions, review obligation and qualified Rust retirement.
- E2: `docs/superpowers/plans/2026-10-11-expanded-zig-rewrite.md:9-23,32-58,83-102`
  retains full features, clean identities, single clock, shared work and reviewers.
- E3: `zig/src/planet_economy/state.zig:6-54,105-157,159-193` fixes reservation,
  sparse-storage replacement motivation, root metadata and current terrain pages.
- E4: `zig/src/coordinated_galaxy/state.zig:6-37,47-79` fixes coordinator allocation
  and ownership; `types.zig:26-45` fixes v0 limits/identity.
- E5: `docs/superpowers/specs/2026-10-11-nyon-zig-coordinated-galaxy-design.md:269-375,377-480`
  defines all-world phases, descriptor publication, fault shutdown, replay/budget
  and counter exhaustion; these are approved contracts, not new implementation proof.
- E6: `zig/src/planet_economy/rules.zig:8-56` defines AlgorithmContext, terrain,
  admitted dimensions and one/two-cell footprints.
- E7: `docs/superpowers/specs/2026-10-11-nyon-zig-seeded-galaxy-design.md:26-45,47-93,231-246`
  defines full metadata scale, keyed generation and version-preserving bridge.
- E8: `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-qualification.md:163-174`
  records retained strategic capacity, timing and endurance requirements;
  `docs/qualification/2026-10-09-zig-strategic-source-inventory.md:30` preserves them.
- E9: `zig/src/planet_economy/command.zig:133-134,383-391` bounds local paths at65;
  `state.zig:413-467` exposes detached COW-reference intervals requiring cleanup.
- E10: `docs/superpowers/specs/2026-10-11-nyon-zig-expanded-durable-sessions-design.md:264-268,459-479,584-602`
  at the M1 hash records moving Proposed archive/node/tick/transient limits; the
  incompatible per-tick contract requires coordinated review before implementation.
