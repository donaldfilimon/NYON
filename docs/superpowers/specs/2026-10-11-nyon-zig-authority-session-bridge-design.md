# Authority/session Task 0 binding and reconciliation addendum

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Historical; Zig continuation canceled 2026-10-19.** Preserve this source contract and evidence. Active recovery targets Rust stable1.99.1; see the 2026-10-19 Rust designs/plans. Historical review labels do not approve new Rust behavior. Tests and CI gates remain paused.
<!-- END 2026-10-19 rewrite decision -->


Date: 2026-10-11. Status: **Proposed; exact-version joint review required. Task 0
is not accepted by this document.** Compiler `0.18.0-dev.146+35accc06e`.
This addendum binds the existing `nyon.fullscale-durable-adapter.v1` composed
profile. It is not a competing adapter, gameplay identity or persistent format.
Only this new document is written by this task. Separately owned specifications,
raw reviews, source, plans, instructions and user profiles remain untouched.

## Inputs, concurrent revision and authority

The initial read of S matched the joint review. S changed during inspection and
added the separate D annex below; its final observed revision also pins D's hash
and corrects copy-admission wording. The affected bridge/provider/partition
sections were reread against that final revision. This addendum adopts D's paired selector,
authority-computed session hash, three publication purposes and fully metered
2 MiB input staging. No assumption is made that D has already passed review.

| Input | Exact observed SHA256 |
| --- | --- |
| A full-scale authority | `a8bc0d41767722b3215e2e6c334d06d5cf5cacc3107f28f83b3b4219ba6ce2f5` |
| C initial economy codec | `c116655859eae999ee1529180521254a575570ba471099fc0ca2e00c8778b6b9` |
| S session design at initial read / joint review | `ce8f58f9a4427a741b1fa72586975cee534ce18977669370af35edc78970b4f5` |
| S intermediate read after D adoption | `e4aa9a269a035b85cbb503b494e17211f2c18908b99319cbbf5442b274b4a87c` |
| S adopted end-read revision | `a3d5b5e04204ecf16fe729a12306bfe868723af6195be48f4692e2ab7b147637` |
| D composed adapter annex | `acf3592befd3c3f28febcf18a5ce4ed78427e0fef181f335c1622c22ec032959` |

Paths: A=`2026-10-11-nyon-zig-fullscale-authority-design.md`,
C=`2026-10-11-nyon-zig-fullscale-authority-codec.md`,
S=`2026-10-11-nyon-zig-expanded-durable-sessions-design.md`,
D=`2026-10-11-nyon-zig-durable-authority-bridge-design.md`, all in this directory.
The findings are in
`../../qualification/2026-10-11-authority-session-contract-review.md`.

For the four Task 0 interfaces only, this proposed addendum refines D, then D
refines S/A as already stated in D. C remains the exact gameplay byte authority.
This precedence takes effect only when the controller records a jointly reviewed
hash tuple including all five files; it is not permission to dispatch against
contradictory moving drafts. The external S/D owner must adopt the concrete
bindings listed at the end, without changing C's literals. Existing standalone A
behavior is unaffected outside this composed profile.

## Pure ownership boundary

The integration writer owns proposed `authority_session_bridge` and its controller.
Shared `authority_session_types` contains only the value types below and pure
Prefix/session-envelope hash grammars. Authority and graph modules may import
those types, but neither imports the other owner or a store module. The pure
bridge imports authority backend and pure graph preparation and owns all
cross-owner validation and publication. A session broker consumes value requests
from the bridge and talks to adapters; no provider callback has a backend pointer.
This dependency graph is acyclic. Implementation file paths/dispatch remain the
controller's separately reviewed plan responsibility.

D's stable resident PairController and one detached PairController are the only
publication domains. Each has two pair slots. A pair contains an AuthorityRef
(root plus same-generation transport context), GraphRef, logical boundary and
Prefix. Composed backends have no independently writable committed selector.
Every gameplay/graph/public snapshot read resolves the one selected pair.

There is one mutation lane per domain; one shared store insertion lane globally.
Replay uses no insertion lane. The resident and detached domains share the outer
0..1024 WorkBudget and 64 public PairLease slots. One global source-read lane
serves both; I/O permits remain at most two as in S. No fresh sibling allowance,
callback during engine calls, third authority or independent session switch exists.

## Concrete shared value declarations

These are complete field/signature contracts expressed in Zig notation, not
claimed compiled declarations. `opaque {}` owners resolve only through private
registries. Value handles never authorize dereferencing destroyed owners.
Native layouts must satisfy the envelopes below; no host struct is serialized.
All namespace/generation/job/request/seal/pass counters are checked u64, nonzero
when issued and never reused. The fixed slot widths do not authorize invalid
slot numbers.

```zig
pub const Hash = [32]u8;
pub const Owner = struct { namespace: u64, generation: u64 };
pub const Ticket = struct {
    owner_namespace: u64, owner_generation: u64, job_id: u64, request_id: u64,
};
pub const Prefix = struct {
    genesis_digest: Hash, tick: u64, byte_count: u64,
    history_head: Hash, transport_root: Hash,
};
pub const RetentionToken = struct {
    ticket: Ticket, prefix_identity: Hash, host_pin_id: u64,
};
pub const PairKey = struct {
    session_namespace: u64, session_generation: u64,
    pair_slot: u8, pair_generation: u64,
};
pub const AuthorityRef = struct {
    owner_namespace: u64, owner_generation: u64,
    root_slot: u8, root_generation: u64,
};
pub const GraphRef = struct {
    session_namespace: u64, slot: u32, generation: u64,
};
pub const Range = struct { start: u64, length: u64 };
pub const Purpose = enum(u8) { live_append = 0, replay_existing = 1, genesis = 2 };
pub const AuthorityBinding = struct {
    owner: Owner, job_id: u64, expected_revision: u64,
    next_tick: u64, next_revision: u64, candidate: AuthorityRef,
};
pub const ResidentBinding = struct {
    owner: Owner, resident_generation: u64, graph_generation: u64,
    graph: GraphRef, catalog_slot_id: u64, branch_id: u64,
    selected_cursor: Hash, selected_tip: Hash, parent_digest: Hash,
};
pub const ReservationKey = struct {
    owner: Owner, slot: u8, generation: u64, id: u64,
};
pub const SessionAppendReservation = struct {
    key: ReservationKey, input: Ticket, expected_pair: PairKey,
    authority: AuthorityBinding, resident: ResidentBinding,
    maximum_payload_length: u32, prepared_graph: GraphRef,
};
pub const FrozenTranscript = struct {
    purpose: Purpose, authority: AuthorityBinding,
    reservation: ?ReservationKey, exact_payload_length: u32,
    ancestry_range: Range, prefix: Prefix, expected_logical_digest: Hash,
};
pub const GraphPrepared = struct {
    purpose: Purpose, reservation: ?ReservationKey,
    authority: AuthorityBinding, resident: ResidentBinding,
    graph: GraphRef, seal_generation: u64,
    exact_payload_length: u32, prefix: Prefix,
    logical_digest: Hash, physical_boundary_digest: Hash,
    index_root: ?Hash, manifest_digest: Hash, node_count: u32,
};
pub const AppendHandle = ReservationKey;
pub const FrozenHandle = struct { ticket: Ticket, generation: u64 };
pub const GraphReadyToken = struct {
    ticket: Ticket, candidate_generation: u64,
    graph: GraphRef, seal_generation: u64,
};
pub const AffineHandle = struct { owner: Owner, id: u64, generation: u64 };
pub const GraphCaptureHandle = AffineHandle;
pub const ReplayHandle = AffineHandle;
pub const TargetMove = AffineHandle;
pub const InstallSeal = AffineHandle;
pub const ShutdownBatchKey = AffineHandle;
pub const ResidentDisposition = union(enum(u8)) {
    saved_exact_capture: AffineHandle,
    explicit_discard: AffineHandle,
    no_unsaved_state: AffineHandle,
};
pub const SessionPolicy = struct { role: u8, civilization: ?u32 };
pub const SealedPublication = struct {
    bridge: Owner, seal_id: u64, seal_generation: u64, purpose: Purpose,
};
pub const WindowKey = struct { owner: Owner, slot: u8, generation: u64 };
pub const SourceKind = enum(u8) {
    candidate_tick = 0, committed_window = 1,
    genesis_object = 2, genesis_stream = 3,
};
pub const ByteCursor = struct {
    authority: Owner, ticket: Ticket, source: SourceKind,
    cursor_slot: u8, cursor_epoch: u64, pass: u64,
    range: Range, window: ?WindowKey,
};
pub const Chunk = struct { len: u16, bytes: [256]u8 };
pub const PairLease = struct { key: PairKey, slot: u8, generation: u64 };
pub const WorkBudget = struct { remaining: u16 };
pub const Bridge = opaque {};
pub const AuthorityBackend = opaque {};
pub const GraphOwner = opaque {};
pub const Broker = opaque {};
pub const PublishGuard = opaque {};
```

The large SessionAppendReservation, FrozenTranscript, GraphPrepared and
ReplayBinding records reside only in private fixed registries. Public/internal
module-boundary results return their small handles, not unmetered by-value copies
of the whole record. Producers/validators populate/read those registries through
<=256-byte charged field operations. A reservation is present only for live_append;
replay_existing/genesis use their purpose-specific binding and encode no dummy
live reservation. Their graph seals still carry the exact authority job binding.

GraphRef resolution also verifies the graph registry's session generation from
ResidentBinding; a matching numeric slot in another generation cannot pass. Pair
and root slots are0/1, append descriptor slot0/1, window slot0..7, cursor slot0..3.
`ResidentBinding.catalog_slot_id=0` is allowed only for a not-yet-installed new
session; branch_id is always nonzero. A live append requires selected_cursor=
selected_tip=parent_digest. Forking from an older cursor must finish its separate
branch selection/exact-prefix preparation first; the tick does not implicitly
choose a new parent during publication.

Prefix wire is exactly112 bytes: G32,tick8,byte_count8,history_head32,root32.
Its identity is `H("NYON-FS1-PREFIX"||those bytes)`. Ticket's diagnostic transport
encoding, if needed, is its four u64 in the displayed order (32 bytes).
RetentionToken's copied encoding is Ticket32,Prefix identity32,host_pin_id8
(72 bytes). These are pure protocol values; Ticket is **not** S's persistent
operation ID. Broker maps each Ticket to an independently ledger-allocated store
operation and persisted S pin. It never copies an in-memory ticket/GraphRef/seal
into a portable archive or changes S's NYONPIN1/NYONOBJ1 grammar.

## Signature inventory and failure contract

`Progress(T)` means `pending | ready:T | invalid:BridgeError | failed:BridgeError`;
only charged polling can deliver a ready/failed outcome. `invalid` rejects the
call without consuming/changing its valid owner's state. `Step` means pending/done.
All copy spans are temporary for one call. No returned borrowed slice exists.
The following function signatures are the required module boundaries:

```zig
openInput(b: *Bridge, expected: PairKey, policy: SessionPolicy,
          count: u16, staged_length: u32, budget: *WorkBudget) Progress(Ticket);
writeInput(b: *Bridge, input: Ticket, offset: u32,
           bytes: []const u8, budget: *WorkBudget) Progress(u16);
sealInput(b: *Bridge, input: Ticket, budget: *WorkBudget) Progress(void);
reserveAppend(b: *Bridge, input: Ticket, expected: PairKey,
              budget: *WorkBudget) Progress(AppendHandle);
beginFromOwnedInput(b: *Bridge, r: AppendHandle,
                    budget: *WorkBudget) Progress(Ticket);
pollCandidate(b: *Bridge, job: Ticket, budget: *WorkBudget) Progress(FrozenHandle);
openPreparedCursor(b: *Bridge, frozen: FrozenHandle, source: SourceKind,
                   range: Range, budget: *WorkBudget) Progress(ByteCursor);
acquireHistoryWindow(b: *Bridge, pair: PairLease, range: Range,
                     budget: *WorkBudget) Progress(WindowKey);
openWindowCursor(b: *Bridge, window: WindowKey, range: Range,
                 budget: *WorkBudget) Progress(ByteCursor);
readBytes(b: *Bridge, cursor: ByteCursor, maximum: u16,
          out: *Chunk, budget: *WorkBudget) Progress(void);
closeCursor(b: *Bridge, cursor: ByteCursor, budget: *WorkBudget) Progress(void);
releaseWindow(b: *Bridge, window: WindowKey, budget: *WorkBudget) Progress(void);
offerGraphPrepared(b: *Bridge, ready: GraphReadyToken,
                   budget: *WorkBudget) Progress(void);
pollSeal(b: *Bridge, job: Ticket, budget: *WorkBudget) Progress(SealedPublication);
pollPublish(b: *Bridge, seal: SealedPublication,
            budget: *WorkBudget) Progress(PairKey);
cancel(b: *Bridge, job: Ticket, budget: *WorkBudget) Progress(CancelOutcome);
```

`SessionPolicy` is C's role/optional civilization value. BridgeError is the closed
set `InvalidBudget, InvalidLength, InvalidRange, InvalidPurpose, InvalidPolicy,
Busy, StaleOwner, StalePair, StaleJob, StaleReservation, StaleCursor, StaleWindow,
AlreadyOffered, AlreadyConsumed, NotReady, CounterExhausted, CapacityUnsupported,
ArchiveFull, StoreFull, QuotaUnavailable, ProviderUnavailable, CorruptSegment,
GraphMismatch, PrefixMismatch, InvariantViolation, Shutdown` plus the existing
registered authority's typed prepublication failure as a tagged `authority` error
payload (not remapped into a gameplay rejection). CancelOutcome is requested,
already_published, or drained; stale cancellation is invalid:StaleJob.
Zero budget returns pending without state, copies, ID consumption or delivery.
Calling a function that would require more than one primitive advances its stored
cursor only through supplied units. Incoming values are bounded; large payloads
must use D's metered input stream. Begin never allocates or recopies that stream.

## Issue 1: sealed pair, identity checks and one selector

`reserveAppend` snapshots every ResidentBinding field from the selected pair,
reserves the candidate AuthorityBinding/job/generation and the session reservation
key, then reserves D/S's insertion/window/store peaks. The authority job is a
reserved admission identity at this point, not an executing candidate. A failed
session reservation releases the reserved identity and leaves sealed input ready;
consumed scheduling IDs are never reused. All reservation ownership is in the
bridge registries; copying the returned value does not clone it.

Each resident/graph/branch/cursor mutation increments its checked generation and
invalidates any earlier reservation **before** becoming visible. During ordinary
preparation the next charged validation detects stale binding and aborts only that
job. Once an offered seal enters final validation, the bridge holds a local
publication fence: branch edits, role changes, replacement, new input and selected
pair mutation return Busy. Snapshot release, provider receipt ingestion and save
CAS completions can proceed only if they do not change the fenced identities;
otherwise queue their bounded notification for after publish/abort. Cancellation
is allowed and revokes readiness in a charged transition. There is no callback or
reentrant opportunity inside validation/publication.

Authority validates its expected revision/tick/candidate generation, codec
invariants, frozen exact bytes/length, global history head, state digest and Prefix.
It independently computes `H("NYON-ES1-NODE"||parent_digest||tick:u64||
length:u32||payload)` using its own bytes and the trusted parent binding, as D
requires. That hash is pure framing, not authority permission to select a branch.
For live_append, session validates ResidentBinding against its live selected graph, parent node
existence/tick/ancestry, branch/cursor/tip, node-count increment, physical object
hashes, graph root/manifest, graph seal owner and reservation capacity. Bridge
compares both owners' exact reservation/A-binding, Prefix, length and logical
hash, plus all owner/generation fields. A provider's supplied hash is never a seal.
For replay_existing the resident binding identifies the immutable captured source
graph, not the mutable resident selection; genesis identifies the new graph
initializer. Their exact purpose-specific checks appear below. Each comparison
of bounded fields/hash blocks uses the shared allowance.

`offerGraphPrepared` first checks the private immutable graph-seal registry. It
moves GraphRef ownership to pending acknowledgment once. Wrong/foreign/stale
handles leave caller ownership intact; a duplicate moved handle yields
AlreadyOffered without another retain. Matching-job metadata mismatch enters
abort; foreign input cannot cancel a different job. Hash/path/codec proof belongs
to the graph producer; acceptance checks its immutable registry result and all
cross-owner bindings before moving it into the unselected pair.

At `pollSeal` completion every generation comparison/counter increment, cursor
closure, old-tail/window acquisition, graph move, Prefix construction and future
unsaved-descriptor binding is finished. The internal seal registry stores the
entire validated A/ResidentBinding/reservation and destination pair, not merely a
caller-supplied digest. PairReady owns root/context+GraphRef and a prepared cache
window. SealedPublication is a lookup token into that entry. It becomes stale
immediately on cancellation or a completed publication. A public duplicate publish
returns AlreadyConsumed; wrong owner/generation returns StaleOwner/StaleJob.

The only publication primitive is bridge-private:

```zig
publishSealedNoFail(b: *Bridge, guard: *PublishGuard) void;
```

PublishGuard is not exported or persisted. `pollPublish` resolves the still-ready
sealed entry under the fence, consumes one remaining unit, and invokes this
primitive without a yield. The primitive changes only `selected_pair_slot` and
bounded published/ownership flags; all root/context/graph/prefix/window metadata
already exists in the unselected pair/unsaved descriptor. It performs no lookup,
comparison, increment, allocation, hash, callback, I/O or failure. The selected
pair is the sole read authority for both modules; no second graph or authority
switch exists. The fence releases after this step. Cancellation after it reports
already_published and cannot restore the old graph or root.

Catalog CAS belongs to later durable save. A conflict or uncertain durability
leaves the committed resident pair dirty and protected by its cache/pins; it is
never a justification for rolling back gameplay or clearing the unsaved queue.

## Issue 2: exact byte sources, cache windows and ownership transfer

Public PairLease APIs remain committed-only. The private bridge capability alone
can request candidate_tick from a matching frozen job. Its range is relative to
the tick `[0,exact_payload_length)` and its ancestry origin is the parent's
Prefix.byte_count. It cannot read arbitrary scratch or committed predecessors.
The one prepared cursor can repeat a pass only by closing it, then reopening with
a new checked epoch/pass. Offset is authority-owned cursor state, never accepted
from a mutable caller token. Reads advance only after the <=256-byte copy; at end,
a charged ready result with Chunk.len=0 marks end, and close is still required.
Canceled, published or replaced job identity makes all prepared cursors stale.

A committed history window is a separate cache capability, **not a gameplay/pair
lease**. Its immutable metadata contains authority owner, window generation,
ancestry range, exact captured Prefix identity, 64-bit segment-slot ownership mask
and per-window refcount. A cache slot cannot be reused while any mask owns it;
slot refcounts are incremented/decremented one/unit with explicit acquire/release
cursors. No interval becomes readable until all of its cache references are owned.
Window range length is at most64 MiB and must already resolve to at most64
resident cache slots; exceeding this bound returns CapacityUnsupported before
any refs change. A window owns bytes only within its captured end, even if a later tick extends
the same physical tail slot. It pins neither an old gameplay root nor a GraphRef.
A caller first obtains a PairLease, acquires needed independent graph/window
ownership, then releases the PairLease. This allows old root retirement while
an unsaved transcript remains readable. Public paired snapshots still retain both
old gameplay and graph and can block the next tick exactly as D specifies.

There are eight WindowKey slots **per authority**, separate from the shared64
PairLease slots: up to two unsaved descriptors, two save/export capture windows,
one old-tail preparation window, one genesis bootstrap window, one replay-check
window and one retiring-transfer window. A role cannot borrow another role's slot
or grow the table. Acquiring unavailable bytes returns NotReady/ProviderUnavailable;
it does not allocate or silently expand a window to a whole history. Public
history queries beyond cache use the authenticated provider, not these windows.
A same-range capture may retain an existing window by an explicitly checked
reference increment; it does not secretly consume a second gameplay lease.

For live append, let `old_end=parent.Prefix.byte_count` and
`tail_start=floor(old_end/1048576)*1048576`. Acquire and complete the old-tail
window `[tail_start,old_end)` before candidate begin; if old_end is aligned the
range is empty and no cache reference is needed. D/A retain the current partial
tail in cache, so this requires no provider I/O. A window cursor reads that exact
prefix for physical first-leaf reconstruction. Session can stream old-tail bytes
then candidate bytes through its existing1 MiB assembly window; records/ticks can
cross boundaries. It never obtains an extra full tick copy. Old tail ownership
persists through all repeated object/path hash passes until transfer or abort.

Before seal, authority installs a **prepared committed window** covering
`[tail_start,proposed.Prefix.byte_count)` into the reserved unsaved slot. It pins
all affected cache slots (at most33 for a32 MiB append straddling a tail); a
candidate-specific visibility flag prevents public access. Existing candidate
cache reservation supplies those bytes. This preparation transfers/reuses old-tail
slot refs without double ownership; any mask delta is applied one slot/unit before
PairReady. Publication changes its visibility/owner flag to the unsaved descriptor
without walking slots or copying bytes, and invalidates candidate cursors.
Postpublication physical staging rereads only this immutable window, reconstructing
leaves/paths/batches as needed. An earlier unsaved window can share its first/last
cache slots under separate refcounts and shorter immutable ranges.

On prepublication abort, close prepared admission first, release graph insertion
pages/manifest/path scratch through their explicit session cursors, then release
prepared/old-tail window masks and candidate cache/root/context ownership through
authority cursors. No new insertion starts until its reserve fully returns.
Published windows release only after exact-prefix pin transfer or explicit
replacement/discard with outstanding captures handled. A delayed cursor's window
generation cannot resolve a reused slot. Completed physical batches remain in
the broker's inventory/pin ownership until reconciled; abort never deletes them
merely because a logical candidate disappeared.

Genesis exposes two tagged private sources: genesis_object `[0,registered_length)`
from the owned manifest/genesis allocation, and genesis_stream `[0,136)` for C
(or the exact registered initial length for another reviewed codec) from the
initial cache. Both use the same copied cursor/close rules. The genesis bootstrap
window pins the initial stream until exact-prefix token or resident ownership
transfer; neither source is treated as an ordinary tick payload.

| Stage | Authority ownership | Session / broker ownership |
| --- | --- | --- |
| Input ready | 2 MiB staged input | No graph insertion yet |
| Append reserved | Scheduled A-binding, tail/window/cache capacity | Unsaved slot; one insertion reserve and manifest; physical peak permits |
| Frozen / graph preparing | Candidate32 MiB+cache, frozen cursor, old-tail window | Graph/path/hash scratch; copied256-byte output only |
| Offered / sealed | Candidate and prepared window; closed cursor | Moved graph seal into unselected pair; insertion pages now graph-owned |
| Published | Selected root/context; committed byte window | Selected graph; unsaved descriptor owns that window; dirty boundary |
| Prefix durable | Selected root/context independent of save | Exact pin moves/references to byte owner; window refs drain; inventory owns physical batches |
| Retired | Obsolete roots/contexts freed after PairLeases | Superseded graph/capture refs drain; insertion reserve reusable only after ownership transfer |

## Issue 3: live, replay and genesis purpose-specific admission

All modes use the same field validators, frozen cursors, graph seal, private fence
and single-selector primitive. Mode is registry state, not an arbitrary caller
flag that bypasses reservation checks.

**live_append:** only the resident domain; use owned input, full SessionAppendReservation,
node-count+1/insertion/unsaved-window reservations and exact parent binding above.
The initial registered payload bound is4,057,265 bytes; future reviewed registries
may require up to32 MiB. At a full graph cap, admission returns ArchiveFull before
candidate execution. Forking first finishes branch selection and an exact-prefix
pin; a longer replay-source pin is not an append parent token.

**replay_existing:** only the detached domain. A ReplayBinding (below) references
a retained source graph/prefix token and an existing authenticated node boundary.
It reserves candidate/cache/input/verification work but **zero** graph nodes,
insertion pages, unsaved descriptors or output archive batches. Source graph
validation supplies the expected parent/tick/digest/payload range/Prefix from the
immutable source closure. Authority regenerates complete bytes, authenticates
source windows and compares every byte/event plus session hash. GraphPrepared
references the existing GraphRef/index/node count; only target-local prospective
cursor metadata changes. Source and resident graphs remain unchanged.
Replay can therefore advance existing ticks when source.node_count=131072.
A cold replay target also bootstraps tick0 in replay_existing mode: independently
validate source genesis object/G/E, regenerate and compare the initial stream,
and bind its initialized root to the captured existing source GraphRef. In that
ReplayBinding, expected_tick=0, expected_parent is exactly32 zero bytes,
expected_node=E, payload_range is the initial stream, and target revision/tick
remain0. This is the only no-parent replay boundary. Source node_count/index stay
unchanged even if already full; no new genesis object/manifest/output archive is
staged. It does not use genesis mode's new-empty-graph node_count0 initializer.
Private genesis-object/stream cursors are available for this checked tick0 replay
binding as well as genuine new genesis. Later replay steps use their existing
parent node and complete tick payload as usual.

The retained source Prefix can extend beyond the target boundary. Source transport
proofs authenticate complete leaves against that longer locally retained root;
only the expected target range is exposed or compared. Target derives its exact
shorter Prefix, even inside a source segment. Verified output cache may be evicted
under the replay pin; it is not staged a second time. Cancellation after a target
step was published means target already_published plus eventual target disposal;
it never mutates resident dirty state. The source closure and broker request refs
remain owned until their explicit release.

**genesis:** only an uninitialized detached/new-session domain with no selected
pair; no input or tick-node reservation exists. A GenesisBinding uses initialization
job/generation and fixed graph-initializer branch metadata. Authority computes G,
initial S/Prefix and session E independently from its owned genesis bytes. Session
validates node_count0, absent logical index and exact initial E/branch/cursor/tip.
It reserves genesis graph/manifest/bootstrap-window ownership, not a live append
slot. Publication initializes one ready pair. If replacing an existing resident,
this remains detached until normal final-install preconditions and replacement
resident-pin durability from S are satisfied. It cannot erase resident unsaved data
or create a second selected genesis by issuing the same token twice.

```zig
pub const ReplayBinding = struct {
    target: AuthorityBinding, source: ResidentBinding,
    source_prefix: Prefix, source_pin: RetentionToken,
    expected_parent: Hash, expected_node: Hash,
    expected_tick: u64, payload_range: Range, expected_prefix: Prefix,
};
pub const GenesisBinding = struct {
    initialization: Ticket, target: AuthorityBinding,
    session: Owner, graph_generation: u64, initial_branch_id: u64,
    initial_next_branch_id: u64,
};
prepareReplayBinding(b: *Bridge, source: GraphCaptureHandle, node: Hash,
                     target: Owner, budget: *WorkBudget) Progress(ReplayHandle);
beginReplay(b: *Bridge, binding: ReplayHandle, budget: *WorkBudget) Progress(Ticket);
beginGenesis(b: *Bridge, binding: GenesisBinding, budget: *WorkBudget) Progress(Ticket);
takeTarget(b: *Bridge, target_job: Ticket, budget: *WorkBudget) Progress(TargetMove);
prepareInstall(b: *Bridge, target: TargetMove, expected: ResidentBinding,
               disposition: ResidentDisposition, budget: *WorkBudget) Progress(InstallSeal);
pollInstall(b: *Bridge, seal: InstallSeal, budget: *WorkBudget) Progress(PairKey);
```

TargetMove is an affine registry handle `{owner:Owner,id:u64,generation:u64}` to
one already-ready pair and its source/history pins; copying the value does not
copy ownership. InstallSeal has the same handle fields and refers to a private
validated target+old-resident record. ResidentDisposition is a tagged opaque
session proof `saved_exact_capture | explicit_discard | no_unsaved_state`, each
with owner/generation/id; it is issued only by S's existing Save/Discard decision
path, not invented by authority. A stale proof or replacement generation fails
before any TargetMove consumption.

prepareInstall checks old resident binding/selection and disposition, target
rules/graph/prefix completeness, source pin and any required new resident durable
pin, no target old pair/lease, available resident unselected slot and all counters.
Under the same publication fence it moves the final target refs into the resident
unselected pair; one selector step installs them. The detached domain becomes
closed for admission; old resident pair/leases remain until retirement. No third
authority is allocated. Failed/stale preparation leaves target ownership with its
caller unless an explicit accepted move already occurred, in which case bridge
abort owns its cleanup; the returned status identifies that move. A late final
install completion can neither install nor free a transferred target twice.

The standalone fake host implements the **same** mode-specific graph/hash/pin
registries and limits in bounded test storage. It may model confirmed in-memory
retention explicitly as test evidence, but cannot claim disk durability or bypass
sealing. Replay fixture setup supplies authenticated existing graph/source bytes;
no synthetic acceptance of arbitrary Prefix/hash fields is allowed. Fake capacity
admission and requested bytes remain counted; an unavailable full fixture is a
proof gap, not an excuse to skip its mode checks.

## Issue 4: pending requests, broker ledger and shutdown grammar

Each authority domain has four ownership positions: descriptor-prefix0,
descriptor-prefix1, replay-source2, and active-source-request3. These positions are
independent of physical root-slot reuse: a pinned old byte owner may outlive that
root through a window/session capture. Prefix owners must transfer/release before
their position is reused. A capture outside these positions belongs explicitly to
S's bounded graph/capture registry; it cannot be an uncounted authority token.
Each of the first three positions is empty, pending_pin or owned_token. Position3
is empty or pending_source. Pending requests are ownership even before success.

```zig
pub const PinPosition = struct { authority: Owner, slot: u8, generation: u64 };
pub const PendingPin = struct {
    position: PinPosition, ticket: Ticket, prefix: Prefix,
};
pub const PinReference = struct {
    token: RetentionToken, broker_ref_id: u64, broker_ref_generation: u64,
};
pub const PendingSource = struct {
    position: PinPosition, ticket: Ticket, prefix: Prefix, ordinal: u64,
    pin: PinReference,
};
pub const Disposition = union(enum(u8)) {
    pending_pin: PendingPin,
    owned_token: struct { position: PinPosition, token: RetentionToken },
    pending_source: PendingSource,
};
pub const DispositionBatch = struct {
    shutdown: Ticket, count: u8, items: [4]Disposition,
};
pub const PinCompletion = union(enum(u8)) {
    success: RetentionToken, failed: ProviderFailure, uncertain,
};
pub const ProviderFailure = enum(u8) {
    unavailable, missing, corrupt, quota, io_failure, canceled_before_start,
};
pub const SourceHeader = struct { ordinal: u64, length: u32 };
pub const IngressPart = union(enum(u8)) {
    header: SourceHeader, proof: struct { offset: u16, chunk: Chunk },
    body: struct { offset: u32, chunk: Chunk }, end,
};
requestPin(b: *Bridge, position: PinPosition, prefix: Prefix,
           budget: *WorkBudget) Progress(Ticket);
completePin(b: *Bridge, ticket: Ticket, result: PinCompletion,
            budget: *WorkBudget) Progress(void);
requestSegment(b: *Bridge, position: PinPosition, pin: RetentionToken,
               prefix: Prefix, ordinal: u64, budget: *WorkBudget) Progress(Ticket);
ingest(b: *Bridge, ticket: Ticket, part: IngressPart,
       budget: *WorkBudget) Progress(void);
cancelSource(b: *Bridge, ticket: Ticket, budget: *WorkBudget) Progress(void);
takeShutdownDispositions(b: *Bridge, shutdown: Ticket, out: *DispositionBatch,
                         budget: *WorkBudget) Progress(void);
acceptShutdown(broker: *Broker, batch: ShutdownBatchKey,
               budget: *WorkBudget) Progress(void);
```

Token.ticket identifies the **pin-acquisition request**. A separate source Ticket
identifies the read; it never overwrites token.ticket. Broker allocates a checked
independent request reference to that token for each started read and holds it
through physical disposal, even if the caller releases its original token.
PinReference is not another durable pin file: it is an explicit refcounted owner
in the broker's bounded token registry. All public handle equality includes every
owner/generation field.

Before creating the first authority, reserve four broker ledger positions for
it; before creating the sole target, reserve another four. Reserve one additional
shared abandoned-request position, **nine ledger positions total**. These are
control records, not nine concurrent I/O operations: at most two physical adapter
requests and one global source read remain allowed. The abandoned position can
hold one request plus its single pin-reference/result disposition; while occupied,
no replacement source read starts. A canceled pending pin may stay in its original
reserved position rather than consuming this position. Admission never creates
a fresh authority domain if its four ledger positions are still occupied by a
closed predecessor. Shutdown updates already reserved ledger entries; it does
not allocate four more request records.

requestPin registers PendingPin locally and in the broker before physical start.
The broker is the sole physical owner until a success is durably confirmed; then
completePin checks exact Ticket/position/generation/Prefix identity and performs
a single move to owned_token. The broker records successful delivery as consumed.
Duplicate identical completion is an idempotent no-transfer observation; it cannot
increment references. A different success token for the same Ticket is a broker
invariant fault: retain implicated physical pins for reconciliation, block admission,
and never guess which pin is safe to delete. Failure before physical start drains
the pending request; uncertain outcome retains it and reports uncertainty.

On cancel/shutdown, a PendingPin is transferred as **pending_pin**, even though
no RetentionToken exists yet. The broker marks its recipient closed and retains
the request through physical completion. A later success is never sent to the dead
namespace: the broker takes the newly created token into its own disposal state,
releases that pin exactly once using S's durable release protocol, then marks the
request physically disposed. Late failure drains its request; late duplicate success
with the same token observes the completed disposition and cannot release again.
Closing a namespace does not reset request or host-pin IDs. Broker tombstones can
compact only below a physically disposed checked request watermark; new or unknown
completion below it is stale and cannot recreate ownership. Persistent uncertain
outcomes remain in S's recovery/ledger, not an unbounded in-memory tombstone list.

`takeShutdownDispositions` closes new local admission, drains local cursors and
creates a fixed batch with exactly the occupied positions in ascending slot order.
It records a transfer-prepared state; it does not silently release pending pins.
The out buffer is the one fixed store-side batch staging record, written through
charged <=256-byte chunks. Its registered ShutdownBatchKey is the small handle
passed to acceptShutdown; the whole tagged array is never copied by value through
an unmetered call. The authority cannot free/reuse that staging record while the
transfer is pending.
`acceptShutdown` first checks the whole bounded batch, reserved ledger ownership
and duplicate shutdown Ticket with charged comparisons. On Busy it consumes
nothing: authority still owns all four positions and cannot deinitialize. After
validation one charged bounded ownership transition accepts the complete batch
by its pre-stored ledger handles; a copied completion reports transfer accepted.
The authority then clears its local owner flags through metered steps and may
free blocks once local leases/cursors drain. If acknowledgment delivery is lost,
retry of the same shutdown Ticket returns the recorded accepted status without
another transfer. No provider request holds an authority address. Delayed I/O
continues under broker ownership; unavailable broker prevents transfer but cannot
lose the pending token/request. The one already-abandoned set is broker-owned
and is not duplicated in the authority's four-item batch.

Provider ingress state is closed: awaiting_header -> proof -> body -> verifying
-> verified -> consuming -> disposed. Header appears once and must equal requested
ordinal and exact length from the retained Prefix. Proof receives exactly1024
bytes (32 sibling hashes bottom-up) at monotonically contiguous offsets, chunk.len
1..256; body likewise receives exactly header.length<=1 MiB. An explicit end is
accepted only after both exact lengths, with no trailing chunk. Repeated header,
offset gap/overlap, early end, extra bytes or wrong stage produce CorruptSegment
for that matching request and invalidate its unexposed local buffer. Stale Ticket
rejects before copying and cannot poison another request. Zero budget consumes no
part. Hash/path verification uses the same budget; no byte reaches replay until
verification succeeds. Verified output consumption copies<=256/unit. Cancellation
invalidates admission first, drains the local window incrementally and moves the
request to broker disposal; public canceled status never claims physical disposal.

## Fixed memory inventory and work accounting

No eighth authority allocation or extra complete tick buffer is introduced.
These partitions refine D's existing4 KiB authority bridge controls and32 KiB
store controls; they do not spend S's adapter margin.

| Authority bridge controls inside existing4,096 bytes | Count × envelope | Bytes |
| --- | --- | ---: |
| Input/candidate owner binding slots | 2 ×256 | 512 |
| Prepared-byte cursor | 1 ×256 | 256 |
| History-window registry | 8 ×192 | 1,536 |
| Window/source read cursors | 4 ×128 | 512 |
| Expected session-envelope SHA context | 1 ×256 | 256 |
| Window-acquisition/shutdown local cursors and flags | Fixed | 512 |
| **Assigned / spare** | | **3,584 /512** |

Large ResidentBinding/GraphPrepared values and broker request entries live in the
store-side records below, not duplicated in each authority control slot. Authority
stores bounded local binding IDs plus its own independent byte/hash/Prefix facts;
private registry resolution compares the full values through metered bridge calls.
Slot masks refer to existing64 cache-slot records, whose owner counts prevent
reuse. They are not arrays of arbitrary historical segment digests. Existing
base controls retain root/page/counter/free-list metadata; their simultaneous
fit still belongs to the mandatory whole-control layout witness.

| Store bridge controls inside existing32,768 bytes | Count × envelope | Bytes |
| --- | --- | ---: |
| Resident/detached controller headers | 2 ×512 | 1,024 |
| Paired descriptor slots | 4 ×256 | 1,024 |
| Shared public PairLease table | 64 ×64 | 4,096 |
| Live/replay/genesis job, validation/fence and install records | 2 ×2,048 | 4,096 |
| Immutable GraphPrepared/seal records | 2 ×1,024 | 2,048 |
| Unsaved descriptors | 2 ×512 | 1,024 |
| Broker ledger positions, including abandoned set | 9 ×512 | 4,608 |
| Broker token-reference registry | 16 ×128 | 2,048 |
| Four-item shutdown transfer staging plus bounded header | 4 ×512 +64 | 2,112 |
| Provider ingress / transfer validation cursors | 2 ×1,024 | 2,048 |
| Copied graph/request output windows including Chunk length/alignment | 2 ×272 | 544 |
| **Assigned / spare** | | **24,672 /8,096** |

The nine ledger records include pending-pin/late-success disposal and source pin
references through completion. The16 token-reference slots are checked; exceeding
them returns Busy before creating an owner. Authority byte ownership can be stored
in this pure shared registry without moving its payload allocation. A batch's
large Disposition values remain here; authority's small shutdown cursor stores only
slot/generation/transfer state and never creates a second [4] array on a heap. Its64-byte envelope header includes the shutdown
Ticket/count and small staging-generation fields; each disposition variant must
fit512 bytes, and Chunk must fit272 including its length/alignment.

Two4,096-byte transport contexts plus authority bridge4,096 consume12,288 of the
2 MiB authority controls, leaving2,084,864. Authority scratch remains2 MiB staged
input +1 MiB verified provider window +5 MiB other scratch. Maximum staged input
is1,054,720 bytes plus16,384 index bytes=1,071,104, within2 MiB; at least4,120 copy
units are required before parse/encoding. Store bridge32 KiB is inside its1 MiB
broker/control row, leaving1,015,808. Existing session assembly/batch/path/index
buffers remain exactly S's assigned109,819,824 bytes and24,397,904 margin under
134,217,728. Source+target stay377,618,432; with the full store cap511,836,160.
These are selected envelope sums, not verified compiler layouts or RSS findings.
Task1 must instantiate each union's largest variant, alignment, two simultaneous
owners and every cleanup state under the pinned compiler/native/Wasm. Failure
rejects the design before implementation, never authorizes hidden extra memory.

All registry scans are bounded/cursored, each field comparison/ref transition
charged; hashes<=64 input bytes/unit, copies<=256/unit. Publication alone is the
prepared single-selector constant step. Parent/graph checking, cancellation and
terminal delivery consume budget. Source I/O waits use no fictional CPU units;
callbacks enqueue copied tickets only. The retained full-step p95<=5 ms target
and separate poll target remain mandatory; no memory/cursor argument proves them.

## Joint cut-point fixtures and acceptance

1. Reserve -> begin -> frozen -> repeated cursor pass -> graph offer -> seal ->
   selector -> durable pin -> window retirement. Interrupt every hash/copy/ref move
   at budgets0/1/7/256/1024. Record pair/root/graph/Prefix and all owner counts.
2. Change each owner, resident/graph generation, branch/cursor/tip, reservation slot
   generation, candidate generation, exact byte length and Prefix field separately.
   All stale checks fail before selected pair changes; a moved graph releases once.
   Duplicate offer/seal/publish never duplicates node/ref ownership.
3. old_end=7 MiB+17, candidate length=32 MiB: read17 old-tail bytes then stream
   new bytes; exactly33 changed segment slots, no whole tick session copy. Hold
   the old unsaved window through next publication, release the gameplay PairLease,
   and prove root retirement succeeds while cache refs remain pinned. A reused
   cursor/window token rejects before reading the newly occupied cache slot.
4. Cancel at each graph-insertion move and prepared-window mask transition. Confirm
   insertion reserve returns once; no next append starts early; broker-owned
   completed batches remain retained until reconciliation. Retry canonical C
   literals and transport roots must match unchanged expected bytes.
5. Replay existing node at source graph count131072; no logical node/index insertion
   or unsaved slot is reserved. End target inside a source segment; compare only
   allowed target bytes after full source-leaf authentication. Cancel after target
   publication, then attempt stale final install; resident pair/dirty marker remain.
6. Genesis mode uses object415/stream136-byte C fixture and node_count0; no tick
   reservation. Duplicate genesis cannot initialize another selected pair. Replace
   only after validated ResidentDisposition and required resident pin; stale target
   moves cannot free an installed owner. At most two authorities exist throughout.
7. Shut down while requestPin is pending, then deliver late success, failure and
   duplicate success separately. Observe one physical pin release after success,
   no release before actual disposal, and no callback to the destroyed namespace.
   Test Busy transfer retains all owners; accepted retry transfers none twice.
8. Cancel a source with a full/partial body in flight, vary duplicate header/proof,
   short/long body/end and stale Ticket. No unverified bytes reach replay; the
   broker read-reference survives until physical disposal. Fill the abandoned slot
   and attempt new source/authority admission; bounded Busy is required.
9. Save CAS conflict after successful selector: selected pair stays committed/dirty
   and owned. Save-N versus published-N+1 retains S's coverage/GC race invariants.
10. Pinned-compiler layout, allocator/failure gates, full C independent fixtures,
    public API integration and real provider/native/browser/endurance/performance
    remain separate required proof. This document executes none of those tests.

## Required amendments by the other owner and controller

- S/D owner: link this exact-version refinement and replace underspecified
  reservation/GraphSeal binding descriptions with the declared ResidentBinding,
  AuthorityBinding and immutable seal-registry contract. Preserve D's sole paired
  selector and authority-computed envelope hash; do not reintroduce dual switches.
- S/D owner: adopt history windows that pin cache slots independently of gameplay
  roots, their exact old-tail/genesis access, eight-slot ownership limits and the
  prepared-to-committed window transfer before the selector. Adopt the ownership
  table and completed-batch cleanup rule.
- S/D owner: make replay_existing explicitly exempt from append node/insertion/
  unsaved-output admission and bind expected source boundaries; keep genesis and
  final-install proof/target moves distinct. No output re-archive or third authority.
- S/D broker owner: adopt tagged pending-pin/source dispositions, nine reserved
  ledger positions,16 checked reference positions, late-success disposal and
  exact staged ingress. Update the four-disposition wording; pending requests
  must be represented even when no token has yet been produced.
- Controller: freeze A/C/S/D/this addendum and update the separately owned Task0
  plan/integration receipt; obtain independent joint review before PASS. If S/D
  changes again, compare the affected sections and recalculate/refreeze that tuple.
  None of these requests authorizes this writer to edit those owned files.

Decisions use delegated game-design authority; implementation approval remains
open until the four Important findings pass exact-version review. No new dependency,
commit, source modification, durable save, host result or full-game completion is
claimed by these declarations or arithmetic.
