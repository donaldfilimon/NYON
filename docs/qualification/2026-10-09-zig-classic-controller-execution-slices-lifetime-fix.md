# Proposed Task3I registry and revision lifetime correction

2026-10-09. Scoped additive closure of review Important I1. Original slices/report/review remain immutable. This selects a caller-owned single admitted coordinator lifetime with explicit runtime address and terminal-state checks, not a globally unique lifetime primitive. Queue8×4096 remains Proposed pending parent disposition. No product authority.

## Exact registry-lifetime override

The original index/generation-only SlotTicket is superseded by:

```zig
pub const SlotTicket = struct {
    registry:*const ReferenceRegistry,index:u8,generation:u64,
};
// Add to ReferenceRegistry private storage:
closed:bool=false,
// reserve error union adds RegistryClosed; other lifecycle error unions
// remain InvalidTicket. No reset/reinit/clone/reopen API is provided.
```

ReferenceRegistry.init creates one fresh value BEFORE its address is admitted. Caller then places it in stable coordinator-owned storage. From the first reserve through terminal deinit, its address and contents cannot be moved, copied as an independent registry, restored from saved bytes, replaced by another init result, or reset. This is a caller precondition in Zig, not compiler-enforced noncopyability. init is not an operation on an existing live/closed instance. No later parent seam may reinitialize this admitted registry at the same address. Every ticket and ordinary ticket copy is scoped to this exact registry lifetime; consumers may keep a stale ticket only for same-live-instance rejection tests, never after terminal shutdown. [task-3i-execution-slices-review.md, § Important I1]

reserve first rejects closed with RegistryClosed, then selects a free entry and checked-increments that entry generation. If increment would overflow, return TicketExhausted with entry/occupancy unchanged; do not reset generation, rotate identity or reopen the registry. Conservative whole-reserve failure on a selected exhausted entry is permitted by this fixed contract even if another entry is free; no fallback reset. Issued ticket stores self address,index,new generation. markOwned/cancel/release first require !closed and ticket.registry==self, index<32 BEFORE indexing, matching generation and exact required state. Otherwise InvalidTicket without mutation. Same-instance canceled/released old generations cannot release a later owned slot. Two distinct simultaneously live stable registries reject each other's tickets by address. [task-3i-execution-slices.md, § Reference ledger, requested bytes and noABA]

Terminal deinit requires every entry free, no pending ownership work and every queue/consumer shut down; it sets closed=true and preserves generations. Use explicit contract assertion for violated shutdown preconditions, rather than silently freeing referenced entries. After deinit, reserve rejects RegistryClosed and lifecycle mutations reject InvalidTicket. No copied ticket may be sent to a new registry or survive its originating coordinator's terminal lifetime. Runtime cannot distinguish a forged/copied old ticket from a new registry deliberately initialized at the SAME reused address with repeated generations. That scenario violates the no-reinit/ticket-nonescape contract; this package makes **no arbitrary address-reuse stale-ticket rejection guarantee**. Address checks are cross-instance checks only while stable admitted lifetimes exist. No process-global counter, token or invented uniqueness primitive is added.

An independent fresh coordinator is allowed only after ALL old producers/queues/prepared/frames/holds/root owners, CurrentViews/event borrows and every ticket copy are destroyed. If allocation later reuses an address, there are no permitted surviving old tickets to compare. A client requiring tickets to survive coordinator teardown needs a separately reviewed nonreused discriminator mechanism; it cannot infer that guarantee here.

## Revision-clock lifecycle and stale-source boundary

RevisionClock belongs to the SAME coordinator lifetime, stable and noncopyable by caller convention from first CurrentView production. Add private closed=false, no reset/restore/reopen API. next adds ClockClosed to its error union; commit adds ClockClosed alongside RevisionConflict. next/commit reject closed before arithmetic; commit requires expected current and checked exact successor, failed checks leave value unchanged. Reaching u64 max yields RevisionExhausted from next; caller enters explicit non-dispatching exhausted state and preserves source/current frame, never wraps or restarts at0 under the same owner domain. [task-3i-execution-slices-review.md, § Reviewed first-slice contracts]

Proposed `RevisionClock.deinit(self:*RevisionClock) void` is terminal, sets closed=true ONLY after caller proves no current/queued/published source-revision domain remains: stop producers, end borrowed current/frame/event views, drain queue, destroy frames/prepared and holds, release root capabilities/tickets, then close clock/registry/livebudget in documented teardown. Do not reset clock on guide/settings/modal/open-close, input edits, failed publication, resident replacement or draft replacement. These are effective source changes within one monotonic clock domain. New independent clock0 is permitted only for a wholly fresh coordinator with no old domain values surviving, not a recovery shortcut.

Neither registry nor clock detects external mutation, reinitialization by direct assignment, source epoch provenance, hidden copied values or concurrent use. Those are explicit caller obligations to be wired and qualified in I3. Actual retained TokenRefs still independently prevent token allocation ABA while queued events own references; that fact does not solve registry storage reuse after teardown. Full owner/context/hold/draft AND revision matching remain mandatory per event. [zig/src/retained_semantics.zig:96; zig/src/retained_semantics.zig:107; task-3i-execution-slices-review.md, § Important I1]

## Layout and queue error changes

The added registry pointer increases actual SlotTicket and hence EventSlot metadata. Recompute target ticket/entry/registry/clock/slot/QueueHeader sizes during implementation; metadata<=256 and EventSlot<=4096 remain mandatory, no silent cap growth. The queue QueueError union adds RegistryClosed; propagation from registry reserve must unwind any first reserved ticket/ref before returning. Closed queue registry is a caller lifetime violation but its explicit error remains deterministic on a still-live closed registry. Existing QueueHeader allocator request and token32/1280 population accounting remain independent. No extra lifetime token allocation or owning registry retain is introduced. [task-3i-execution-slices.md, § Eight4096-byte slots: explicit new Proposed storage choice]

## Independent fixtures matching the actual guarantee

- Same live registry: reserve/owned/release, retain copied old ticket, reserve reused index with greater generation; old cancel/markOwned/release reject InvalidTicket and preserve new entry/usage. Canceled reservation analog; invalid index checked before access.
- Two distinct LIVE pinned registries A/B at different addresses: first ticket generation may equal; A ticket used against B rejects by registry address without changing B. No claim about freed address reuse.
- Same live registry max generation fixture: reserve returns TicketExhausted, all ownership/occupancy unchanged, no reset. Fully free terminal deinit sets closed; subsequent reserve RegistryClosed and ticket lifecycle InvalidTicket. Test no deinit while owned through an appropriate internal contract-failure harness, not undefined reuse.
- Caller shutdown fixture destroys all borrowed views, queue events, frame/prepared/hold/root owning refs and tickets before deinit; actual references/budget return to zero. A new independent lifetime is tested WITHOUT transporting old tickets/views. Do not create an allegedly supported same-address reinit regression expecting magical rejection; document forbidden operation as a caller protocol obligation.
- Clock next/commit conflict/overflow/closed failures preserve value; guide/modal/resident transitions retain monotonic value, same-token edits reject old revisions; coordinator shutdown invalidates every event/frame/current view before independent clock initialization. These are synthetic foundation/coordinator-lifetime fixtures, not actual host acceptance.

No other routing/focus/guide-wholeadvance/touch/exit decision changes. Required final actual3H/guide APIs, producer delivery policy, stale-display/source protocol and host lifecycle qualification remain separate I3 prerequisites. Fresh scoped review must assess these honest lifecycle preconditions before implementation dispatch. No compiler/test/gate/transient integration read or source write.
