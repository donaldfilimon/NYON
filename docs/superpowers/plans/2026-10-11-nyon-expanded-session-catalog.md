# Expanded Session Catalog Implementation Plan

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Historical; Zig continuation canceled 2026-10-19.** Preserve this source contract and evidence. Active recovery targets Rust stable1.99.1; see the 2026-10-19 Rust designs/plans. Historical review labels do not approve new Rust behavior. Tests and CI gates remain paused.
<!-- END 2026-10-19 rewrite decision -->


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement and independently qualify the canonical catalog, allocation
ledger and metadata mutation rules consumed by the expanded durable-session store.

**Architecture:** Pure Zig codecs own validated metadata and canonical bytes.
An allocator-owned catalog exposes copied records and a pure prepare/commit model;
filesystem and browser transactions later consume that model. This plan does not
implement history persistence, a host adapter or playable Continue.

**Tech Stack:** Exact Zig `0.18.0-dev.146+35accc06e`, std allocator and SHA-256;
no new dependencies.

**Spec:** [expanded durable sessions](../specs/2026-10-11-nyon-zig-expanded-durable-sessions-design.md),
sections Namespace/catalog/atomic transactions and allocation ledger.

**Status:** Proposed detailed plan; independent review and controller approval
under verified delegated design authority precede implementation. The streaming
history revision is concurrent; freeze this plan's consumed metadata requirements
against its reviewed revision before dispatch. No code or test result is claimed.

## Global Constraints

- Canonical checkout; preserve other owners. No commits, pushes or instruction edits.
- New native namespace `nyon-expanded/session-v1/`; browser database
  `nyon-expanded-session-v1`. This pure slice neither accesses them nor old data.
- Catalog maximum 1 MiB, 64 slots, 64 branches per slot, labels 1..128 UTF-8 bytes
  with no C0 controls. IDs nonzero; checked u64 counters begin at 1.
- Exact `NYONCAT1` and `NYONAL01` framing from the spec, little-endian numbers,
  body SHA-256 and ascending IDs; no trailing bytes or permissive decoding.
- One mutation lane is a store responsibility. Pure preparation never claims a
  storage CAS committed, a write cancelled, or an abandoned write reversed.
- Own only new `zig/src/expanded_session/` metadata files listed below. Do not
  register imports in shared `root.zig` or mutate `build.zig` during this slice.
- Every task records a behavioral RED, GREEN, source hash and fresh review.
- Before every focused run, execute `test "$(zig version)" = "$(cat .zigversion)"`
  from `zig/`; mismatch stops execution. Focused suites remain outside
  `tools/check.sh` until coordinated root/build registration. Registration and a
  fresh repository gate are explicit later integration prerequisites.

## Review Focus

1. Correctly hashed malformed catalog: semantic validation still rejects it (Task 1).
2. Allocation failure halfway through decoding: no leaked buffers or caller mutation (Task 1).
3. Valid allocator chain with exhausted IDs: no wrap or reused identifiers (Task 2).
4. Metadata races and abandoned outcomes: pure preparation cannot promise durable success (Task 3).
5. Removing the selected slot and failed selection: atomic selection consistency (Task 3).

## File map and common interface

All paths are proposed. `types.zig` owns `Digest=[32]u8`, `SlotId=u64`,
`BranchId=u64`, `Allocator=std.mem.Allocator`, and
`Error=Allocator.Error || error{Invalid,TooLarge,IdExhausted,Conflict}`.
`Label` owns its bytes. `Branch` owns a label plus ID, tip and cursor digests.
`Slot` owns its label and branch array plus ID, archived flag and manifest digest.
`Selection` is a value `{slot_id,branch_id,cursor_digest,manifest_digest}`.
`Catalog` owns slots and has generation, allocator_watermark and optional selection;
`deinit(*Catalog,Allocator) void` releases nested ownership exactly once.
`limits.zig` centralizes only these metadata caps; future history limits belong
to the independently reviewed streaming plan, not guessed constants here.

### Task 1: Canonical catalog codec and owned metadata

**Files:** Create `zig/src/expanded_session/{types,limits,catalog,catalog_tests}.zig`.

**Interfaces:** `catalog.decode(a:Allocator,bytes:[]const u8) Error!Catalog`,
`catalog.encode(a:Allocator,value:*const Catalog) Error![]u8`,
`catalog.validate(value:*const Catalog) Error!void`. Returned catalog/bytes belong
to the caller. Decode validates all structural metadata before returning; ancestry
and agreement with immutable manifests require a later graph validator and are
never claimed by this codec. Encode rejects noncanonical input rather than sorting
or repairing it silently.

- [ ] Write `empty_literal`, `selected_literal`, `semantic_rejection`,
  `capacity_edges` and `decode_allocation_failures` in `catalog_tests.zig`.
  Pin literal expected wire independently with reviewed handwritten field offsets
  and SHA fixtures, not output copied from the implementation. Empty catalog body
  is 19 bytes and framed catalog 67 bytes. Assert initial generation/watermark 0,
  no slots and absent selection. Assert a selected catalog round-trips identical
  bytes and owns labels even after the input buffer is overwritten.
- [ ] Rehash malformed fixtures to test duplicate/descending IDs, zero IDs,
  invalid UTF-8, C0 labels, lengths 0/129, counts 65, flag value 2, missing selected
  branch, cursor/manifest mismatch, trailing data, truncation and bad digest.
  Reject generation0 with slots/selection, nonzero generation with watermark0,
  initial generation0 with nonzero watermark, and namespace branch IDs duplicated
  across different slots. Initial catalog is exactly empty metadata with both
  counters0; branch IDs in this catalog are namespace-global.
  Boundary labels128, slot64 and branch64 must work; bytes above1MiB fail before
  allocations. Walk every injected allocation failure and check no leak.
- [ ] Run `zig test src/expanded_session/catalog_tests.zig` from `zig/` and record
  the actual failing behavior. Missing files initially are scaffolding failures;
  require a behavioral failure once imports/types exist before codec implementation.
- [ ] Implement the exact framed codec with checked offsets and owned nested
  allocations. Failed decode/encode preserves caller input and supplied catalog.
- [ ] Run that command in Debug and with `-O ReleaseSafe`, then
  `zig fmt --check src/expanded_session`. Both suites and formatting must exit0;
  freeze the owned source and obtain independent spec and quality review.

### Task 2: Allocation ledger records and chain validation

**Files:** Create `zig/src/expanded_session/{allocator_ledger,allocator_ledger_tests}.zig`.

**Interfaces:** `LedgerRecord` contains ordinal, previous_digest,
next_generation, next_slot_id, next_branch_id, next_request_id and digest.
`encodeRecord(record:LedgerRecord) Error![112]u8`,
`decodeRecord(bytes:[]const u8) Error!LedgerRecord`,
`Chain.init(target_watermark:u64) Chain`,
`Chain.accept(*Chain,record:LedgerRecord) Error!void`,
`Chain.finish(*const Chain) Error!LedgerBinding`,
`reserve(current:LedgerRecord,kind:AllocationKind) Error!Reservation`, where
`AllocationKind=enum{generation,slot,branch,request}` and `Reservation` contains
allocated value plus the next record. Pure reservation is not durable allocation.
All next counters start1; first ordinal1 and zero previous digest. Empty-chain
acceptance requires all four counters exactly1, ordinal1, zero predecessor and
valid framing/digest. Later ordinals are contiguous, counters never decrease and
at least one counter strictly advances. Rejection leaves Chain unchanged. Chain maximum
1,048,576 records. Advancing an ID also advances ordinal and chains prior digest.

- [ ] Write literal first-record fixture; assert112-byte size, digest of first80
  bytes, exact little-endian offsets, initial counters and accepted chain identity.
- [ ] Add `broken_chain`, `partial_record`, `monotonic_high_water`,
  `exhaustion_preserves_record` and `journal_capacity` tests. Rehashed records
  with decreasing counters, skipped/decreasing ordinals or false predecessor fail;
  independently advanced counters may contain gaps. u64 exhaustion fails without
  modifying prior chain or record. Wrong-sized112±1 records fail.
  Add separately rehashed false-origin fixtures changing each of the four
  counters, ordinal and predecessor, and an unchanged-counter later record.
- [ ] Define `LedgerBinding` as the result of a complete validated chain scan:
  it contains the validated head, exact record at target watermark and that
  record's predecessor. `Chain` retains just these three records, not the journal.
  `finish` rejects an
  absent requested ordinal. Watermark0 uses a virtual origin with next counters1;
  it may bind only the initial empty catalog, even if the journal has later records.
  Callers finish only after transport establishes exact journal end; later
  adapters cannot skip a corrupt/partial suffix. No caller-supplied validity flag.
- [ ] Implement `validateAgainstLedger(catalog:*const Catalog,binding:*const LedgerBinding)
  Error!void` in `allocator_ledger.zig`. Require exact watermark correspondence,
  watermark at/below validated head and catalog generation/slot/namespace branch
  IDs strictly below the corresponding next counters at that watermark. Test
  future/missing watermarks, each ID/counter equality and excess, valid exact
  binding, valid older recovery watermark in a longer validated chain, and a
  corrupt later suffix. This pure contract precedes every adapter publication.
  Every catalog mutation reserves its generation last, with its commit watermark
  equal to that reservation record's ordinal. Require the bound record's
  next_generation to equal predecessor.next_generation+1 and catalog.generation
  to equal bound_record.next_generation-1, both checked. Initial empty catalog0/0
  is the sole virtual-origin exception. Add `stale_in_range_generation`: current
  generation1 and later ledger next_generation5 cannot validate proposal gen2 at
  that newer watermark. Test a watermark record advancing only a request counter
  rejects noninitial publication even if its generation remains in range.
- [ ] Record behavioral RED, implement fixed record codec and incremental chain
  validator, then run `zig test src/expanded_session/allocator_ledger_tests.zig`
  in Debug and ReleaseSafe plus formatting. Freeze and independently review.
  Native flush/partial-tail recovery and IndexedDB atomicity belong to later adapters.

### Task 3: Pure metadata preparation and generation checks

**Files:** Create `zig/src/expanded_session/{metadata,metadata_tests}.zig`.

**Interfaces:** `MetadataMutation=union(enum){select:Selection,
rename_slot:struct{slot_id:SlotId,label:[]const u8},
rename_branch:struct{slot_id:SlotId,branch_id:BranchId,label:[]const u8},
archive_slot:struct{slot_id:SlotId,archived:bool},remove_slot:SlotId}`.
`prepare(a:Allocator,current:*const Catalog,expected_generation:u64,
reserved_generation:u64,reserved_watermark:u64,mutation:MetadataMutation) Error!Catalog`
returns a detached owned proposal. Reserved generation must exceed current;
watermark must exceed current watermark. Store adapters must validate the
corresponding durable ledger reservation and execute CAS; pure preparation cannot.
Before publication, adapters must pass the proposal and completed chain binding
through Task2 `validateAgainstLedger`, proving reservation numbers belong to the
exact durable allocation ordinal. Native preparation flushes allocation first;
browser validation includes its candidate ledger record in the same atomic
transaction. Raw integers alone never authorize publication.
Adapters reserve catalog generation last after request/slot/branch allocation;
the exact generation-increment record defines its commit watermark. Repair follows
the same rule; validating a historical recovery catalog uses its original matched
record and predecessor, never the latest head's counters as substitute proof.
Creation/save/import and node cursor changes require validated graph closures and
are deliberately not accepted as metadata-only commands.

- [ ] Write `stale_expected`, `select_exact`, `remove_selected`,
  `rename_copies_input`, `archive_retains_capacity`, `prepare_allocation_failures`.
  Assert stale generation and malformed selection fail, current bytes stay exact,
  successful proposal owns all bytes, removing selected slot clears selection in
  the same proposal, and unselected removal preserves it. Archiving frees no slots.
  Add `forged_reservation_binding`: plausible raw numbers produce only a proposal;
  Task2 binding rejects unallocated generation/watermark and IDs. A proposal from
  an accepted exact allocation record passes binding; competing store generation
  still needs CAS and cannot be called committed by this pure API.
- [ ] Record RED, implement detached preparation with validated input and explicit
  ownership, then run `zig test src/expanded_session/metadata_tests.zig` Debug and
  ReleaseSafe, all previous task commands and formatting. Freeze owned sources;
  obtain fresh review of the complete slice.
- [ ] Publish a scoped qualification receipt containing actual commands/counts,
  source hashes and remaining graph/store/runtime gaps. The shared full gate may
  run only after coordinating its owner; no pure metadata result qualifies saves.

## Self-review and subsequent work

This slice covers catalog framing, owned labels/records, allocation high-water
records and metadata preparation. Each review-focus row has named tests above.
Host locking/flush/CAS, streamed objects and authenticated history, resident pins,
GC, import/export/replay and runtime Continue require the reconciled full-session
spec and subsequent detailed tasks. No substitute implementation is selected for
those surfaces. A later adapter uses these exact owned metadata interfaces.
