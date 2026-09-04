# Workshop V1 Library Addendum Review — 2026-09-04

## Verdict

**APPROVE FOR IMPLEMENTATION PLANNING.** No P0, P1, or P2 finding remains after two correction rounds. The addendum is consistent with the accepted Workshop V1 authority, current store/session/runtime direction, and the independently reviewed recovery repair.

The review was read-only. It did not edit product code, stage changes, build artifacts, run tests, or exercise UI.

## Scope

Reviewed:

- `docs/superpowers/specs/2026-09-04-nyon-workshop-library-addendum.md`
- its authority link in `docs/superpowers/specs/2026-09-02-nyon-v2-design.md`
- its historical-ledger clarifications in `.superpowers/sdd/nyon-v2/progress.md`
- current `src/app/client_runtime.rs`
- current `src/workshop/session.rs`
- current memory/native/IndexedDB store contracts and implementations
- the accepted Workshop recovery and baseline reviews

## Resolved review findings

### Implementation authority

The addendum now says `Accepted for implementation`, links from the active V1 design, and precisely supersedes only the prior statements that a multi-slot chooser and portable transfer still required a design decision. It does not supersede recovery, evidence, RulesV1, or Living Galaxy boundaries.

### Exact-generation races

Every Library Open, Use for Continue, and row Export intent carries the observed row generation. A different generation returned by load is a conflict, never an implicit upgrade. Store selection becomes generation-checked, and every successful Commit or recovery Promotion atomically clears Continue selection for that slot before a later exact-generation Select restores it.

The acceptance contract covers both material races:

- list generation N, then receive concurrent head N+1 before activation completes;
- commit or promote a new head, then stop before exact-generation selection.

Neither case may open a generation that was not explicitly validated and selected.

### Recovered predecessor ordering

Startup Continue and Library Open share the coherent N/N+1/N+2 sequence: fully replay predecessor N, receive explicit recovery acceptance, promote it against corrupt head N+1 into new valid head N+2, select N+2, then install the already-validated history. Failure retains the active session and validated candidate.

### Side-effect-free row export

Row Export has its own recovery contract. It may prepare explicitly labeled bytes from a validated predecessor, but it never promotes storage, selects Continue, or installs/replaces a session. Repair/Open is a separate user action.

### Automatic active save

Library is the exclusive route for choosing a different existing slot, while a successful save of the active Workshop preserves the accepted Commit → validate returned generation → exact-generation Select Continue behavior required for durable exit.

### Imported and retryable state

Imported archives are paused, slotless, dirty, visibly unsaved, and not Continue-ready until their first durable slot save. Missing exact-pack import retains bounded archive bytes until Cancel and resumes only after the matching pack is durably stored. Catalog-store failure and post-store replacement blocking are explicit retryable states that preserve the resident session and never collapse into a global recovery screen.

### Store-operation concurrency

The addendum distinguishes replacement-capable archive work from pack-only storage and resident recovery:

- Open, Workshop Archive import, Use for Continue, and recovery of another slot remain blocked while replacement is unsafe;
- content-pack-only storage may proceed when it requests no replacement;
- the resident session's recovery-persistence obligation remains eligible because it is the operation that restores safety.

### Transfer truthfulness

Native durable-save wording requires same-directory temporary output, flush, file synchronization, atomic placement, and parent-directory synchronization where supported. Otherwise the product reports only written or handed to the operating system.

Browser export reports handoff/download initiation only. Object URL and anchor cleanup is bounded but deferred past the synchronous click so cleanup cannot race download consumption.

## Native picker decision

Deferring the concrete native picker dependency is accepted. Slices through the platform-neutral transfer protocol may proceed without it. The native adapter may not be approved until a compatibility spike establishes macOS, Windows, Linux, portal/event-loop behavior, licensing, and packaging for the pinned dependency. A fixed directory, clipboard substitute, blocking placeholder, or raw platform code in `app.rs` is not an acceptable substitute.

## Evidence boundary

This review approves design authority and implementation planning only. It does not claim that any Library/store/transfer source has been implemented, tested, built, or exercised. Every ordered source slice still requires RED/GREEN evidence, independent review, artifact gates where applicable, and live native/browser acceptance.
