# NYON Workshop V1 Library and Portable Transfer Addendum

**Status:** Accepted for implementation

**Date:** 2026-09-04

**Applies to:** Offline Galaxy Workshop V1 only
**Does not change:** RulesV1, Classic Sector, canonical Workshop formats, simulation authority, or Living Galaxy V2 ownership

## 1. Decision

Galaxy Workshop V1 gains a first-class **Library** surface. The Library is the only product route for listing and managing Workshop slots, deliberately opening a particular existing slot, choosing a different existing slot as the future Continue target, and starting portable content-pack or Workshop-archive transfers. A successful save of the active Workshop still generation-checks and selects the newly committed generation so durable exit retains its accepted V1 behavior.

The Library does not decode archives itself and does not mutate authoritative Workshop state. It presents immutable client state and submits typed client intents. A single exact-catalog Library client performs validation, bounded replay, recovery, and generation-checked Continue selection before `ClientRuntime` may replace the active session.

Browser transfer success is described as **handed to browser** or **download started**. Only a native file implementation that has completed its write, flush, synchronization, and final placement may report a durable file save. Neither result is a Workshop store generation.

This addendum supersedes only the earlier implementation-ledger statements that a multi-slot chooser and portable file transfer required an additional design decision. The accepted recovery repair, original Workshop V1 design, RulesV1 boundary, and Living Galaxy V2 ownership remain in force. The active V1 design and progress ledger link back to this authority.

## 2. User-facing routes

Library is available from the main menu and from an active Workshop. Closing it returns to the exact prior screen and preserves selection, camera, drawer, and focus state when those objects still exist.

The surface contains:

- all bounded Workshop slots, including a distinct archived section or filter;
- slot name, stable slot identifier, head generation, predecessor availability, archived state, and Continue marker;
- Open, Rename, Archive, Unarchive, Use for Continue, and Export actions;
- Import Workshop Archive, Import Content Pack, Export Active Archive, and Export Active Content Pack actions;
- explicit empty, loading, validation, retry, cancellation, conflict, and capacity states;
- stable focus restoration after every modal and asynchronous outcome.

Compact presentation uses the shared virtual-list and drawer rules. Row identity is derived from `SlotId`, never from a current list index. Pointer and keyboard activation submit the same typed intent.

## 3. Slot lifecycle

Archive is reversible. `ArchiveSlot` sets only the archived flag and clears Continue when that slot was selected. `UnarchiveSlot` sets only the archived flag back to false; it does not open the slot or restore Continue selection. Both operations preserve the exact name, head generation, retained predecessor generation, archive bytes, and catalog references.

The currently resident slot cannot be archived through the Library. Doing so would leave a live authoritative session whose next save must fail as archived. An archived row cannot be opened or selected for Continue until it is explicitly unarchived.

Rename continues to validate through the canonical `SlotName` type. Destructive or lifecycle-changing actions use explicit confirmation text, including whether Archive will clear Continue.

## 4. Exact-catalog open and Continue

The existing startup Continue algorithm is the canonical open algorithm and must be extracted for reuse, not duplicated.

Every Library Open, Use for Continue, and row Export intent carries the head generation observed in the activated row. For every candidate archive the client must:

1. load the requested slot, compare the returned head generation with the row's observed generation, and report a refreshable conflict instead of silently accepting a newer head;
2. inspect the bounded authenticated archive envelope;
3. read the archive-declared catalog hash;
4. use the built-in pack only when its hash exactly matches;
5. otherwise load that exact stored pack;
6. decode, canonicalize, and require exact canonical bytes and hash;
7. replay the archive incrementally through the bounded decoder;
8. offer only the same slot's retained predecessor when the head is invalid;
9. preserve the fully validated candidate if final installation becomes temporarily blocked;
10. replace the active session only after all required store mutations succeed.

The current same-catalog session `RequestLoad` and `RequestImport` actions are not Library entry points. They decode with the resident session catalog and therefore cannot safely open or import an archive declaring a different catalog.

Explicit Open also selects that exact validated generation for Continue before session installation. `SelectContinue` uses compare-and-swap semantics:

```rust
WorkshopStoreRequest::SelectContinue {
    slot: SlotId,
    expected_generation: SaveGeneration,
}

WorkshopStoreResult::ContinueSelected {
    slot: SlotId,
    generation: SaveGeneration,
}
```

Memory, native, and IndexedDB adapters compare the head generation inside the same serialized mutation or transaction that writes the selection. Every successful `CommitSlot` and `PromoteRecoveredSlot` atomically clears `selected_continue` when it names the mutated slot. The subsequent generation-checked `SelectContinue` restores the marker only after the new head is known and validated. A conflict preserves the resident session and validated candidate and offers a retry after refresh; it never silently selects a newer head.

The automatic active-session save sequence remains Commit, validate the returned generation, then generation-checked Select Continue. If the process stops between those operations, startup exposes no selected Continue candidate rather than opening a generation that was never explicitly selected.

When head generation N+1 is invalid but its retained predecessor N fully replays, recovery keeps the already-reviewed promotion obligation and uses this exact order:

1. replay predecessor N entirely in temporary state;
2. show the explicit same-slot recovery offer;
3. after acceptance, call `PromoteRecoveredSlot` with expected corrupt head N+1 and recovered predecessor N;
4. receive new valid head N+2 while retaining predecessor N;
5. call `SelectContinue { expected_generation: N+2 }`;
6. install the already-validated history as resident generation/head N+2.

Startup Continue and explicit Library Open use the same sequence. Failure to promote or select retains the active session and validated candidate and remains retryable.

Use for Continue performs the same exact-catalog resolution and full replay but does not replace the active session. It is unavailable while the resident Workshop replacement invariant or a recovery-persistence obligation is active.

Row Export also carries the observed generation, resolves the exact catalog, and fully replays before bytes become Ready. It does not mutate storage, select Continue, or install/replace a session. An invalid head never silently exports its predecessor: the Library presents an explicit export-recovery choice, and acceptance prepares the already-validated predecessor bytes with the distinct source label `Recovered predecessor; stored head and Continue unchanged`. It does not call `PromoteRecoveredSlot` or `SelectContinue`; repair/open remains a separate user action using the recovery sequence above. Cancellation or failure leaves the active session, stored slot, and Continue marker untouched.

## 5. Imported archives

A portable archive is first validated against its exact declared catalog and fully replayed in temporary client state. Only then may it become an imported Workshop session.

An imported Workshop is:

- paused;
- not associated with a resident slot;
- dirty and explicitly labeled `Imported; not saved`;
- ineligible as a Continue target until its first durable slot save completes;
- protected by the same replacement invariant as any other unsaved session.

If the exact pack is missing, import remains a retryable Library operation. Supplying a different pack, a noncanonical pack, or a pack whose hash does not match rejects without changing the active session. Once the matching pack is durably stored, the retained archive candidate resumes bounded replay rather than asking the user to reselect the file.

Retained archive bytes are bounded by the existing archive-size limit and released on explicit Cancel. A validated matching pack resumes the archive only after `PutPack` has completed durably; an in-memory validation result alone is insufficient.

## 6. Retryable catalog import

Catalog import is an explicit client state machine, not a Boolean and not a global recovery screen. Its public snapshot contains bounded typed facts only:

- idle;
- storing with expected hash and whether a new Workshop is requested;
- stored;
- retryable store failure with diagnostic code;
- start blocked because the resident Workshop is not replaceable.

Internal retry material may retain the validated pack and its canonical bytes within the existing pack-size limit. Raw imported content and user paths never enter diagnostics, save data, or logs.

A store failure offers Retry and Cancel. A successful store followed by blocked session replacement retains the stored pack and offers Start When Safe and Cancel. Cancel clears only pending client intent; it never deletes a pack that has already been stored. Every failure preserves the active session, digest, queued work, and recovery obligations.

Archived slots continue to count toward the bounded slot capacity. Rename and Unarchive for unrelated slots may proceed while an active session is not replaceable only when they do not contend with an occupied store mutation lane. Library Open, Workshop Archive import, Use for Continue, and recovery of another slot remain disabled until replacement and persistence invariants are safe. Content-pack storage may proceed when it does not request session replacement, and the resident Workshop's own recovery-persistence obligation always remains eligible to run because completing it is what makes replacement safe.

## 7. Portable transfer boundary

Canonical pack/archive generation and validation remain in the Workshop core, session, and exact-catalog Library client. Platform file APIs are hidden behind a small object-safe asynchronous transfer protocol with bounded jobs:

- Choose Import with declared kind and maximum bytes;
- Hand Off Export with kind, sanitized suggested name, and validated canonical bytes;
- Import Chosen, Export Handed Off, Cancelled, or a bounded opaque failure.

Export is two-stage. The client first prepares and validates canonical bytes and shows Ready. A second direct user activation invokes platform handoff, preserving browser picker/download activation requirements.

Suggested names are predictable and sanitized:

- `<slot-name>.nyonworkshop.json`
- `<catalog-hash-prefix>.nyonpack.json`

Extensions and MIME types are convenience hints only. The decoder remains authoritative.

## 8. Native boundary

Native import/export uses a maintained dialog capability behind the transfer trait; no raw Cocoa, Win32, or GTK logic belongs in `app.rs`. The concrete dependency and pinned version are selected only after a target compatibility spike proves macOS, Windows, Linux, portal, event-loop, license, and packaging behavior. That dependency decision is an implementation gate, not permission to weaken the transfer contract.

Native import checks the reported size before allocation and reads at most the maximum plus one byte. Native export writes to a same-directory temporary file, flushes and synchronizes that file, atomically places or replaces the destination only after overwrite confirmation, and synchronizes the parent directory where the platform supports that guarantee. If the platform cannot prove that durability sequence, the result says `written` or `handed to operating system`, not `durably saved`. Cancellation is ordinary and non-diagnostic. Full user paths and file contents are never logged.

## 9. Browser boundary

Browser import uses a semantic file input with exact accept hints, verifies `File.size` before copying, and reads asynchronously. Browser export creates a JSON Blob, temporary object URL, and download anchor during direct activation. After `anchor.click()` initiates handoff, bounded deferred cleanup removes the anchor and revokes the URL on the next suitable task or a later platform-confirmed point; it must not revoke synchronously in a way that can race download consumption. Closures, listeners, and object URLs are bounded and releasable.

There is no network request. IndexedDB persistence remains separate from portable transfer. Browser success cannot claim filesystem durability or user-selected final location.

## 10. Safety invariants

- Library presentation state and transfer progress never enter `StateDigest`.
- No archive or catalog becomes active before complete validation.
- No arbitrary slot Open substitutes the resident catalog.
- No operation clears or replaces the active in-memory Workshop on failure.
- No retained recovery obligation is discarded by opening Library.
- No archived slot is opened or selected for Continue.
- No offscreen virtualized row receives sighted output, pointer input, or adapter interactivity.
- No diagnostic includes raw imported bytes, untrusted error text, or full file paths.
- Classic and RulesV1 behavior remain byte-for-byte outside this surface.
- Active Archive Export may include current dirty in-memory state, but its label states `Portable export; Workshop not saved for Continue` until a store commit and generation-checked selection complete.

## 11. Implementation order

1. Retryable catalog-import state.
2. `UnarchiveSlot` plus generation-checked Continue selection in every adapter.
3. One exact-catalog `WorkshopLibraryClient`, with startup Continue migrated onto it.
4. Library screen and responsive/accessible presentation after the inspector and SDF slices stabilize.
5. Platform-neutral transfer jobs.
6. Native and browser transfer adapters after the native dependency spike is accepted.

Each slice receives RED/GREEN tests, an independent review, focused gates, and exact ownership boundaries before the next begins.

## 12. Acceptance

Release qualification requires native and browser journeys that create multiple slots; rename, select, archive, unarchive, and reopen exact generations; quit and Continue the same item; export and re-import exact packs and archives; save imports as distinct slots; and prove matching catalog hash, revision graph, branch, tick, and digest.

Negative journeys cover missing, wrong, noncanonical, and corrupt packs; corrupt head with same-slot predecessor recovery; store failure; cancellation; stale generation; and blocked session replacement without active-session loss.

Race qualification includes row generation N followed by a concurrent N+1 commit before Open/Use/Export, and process failure after Commit or Promote but before Select Continue. Both must reject or expose no selected candidate rather than silently opening an unvalidated generation.

Browser acceptance covers natural WebGPU and forced WebGL2 with networking blocked, plus honest download-handoff language. Native acceptance separately proves picker interaction and durable write mechanics. Local browser success does not substitute for the named Safari, Chrome, Edge, Firefox, macOS, Windows, and Linux release matrix.
