# Task 3I2 independent preparation review

2026-10-09. Read-only review of the proposed second chunk after the proposed routing foundation. This is not current product qualification or host-policy approval.

## Verdicts

- **Specification compliance: NEEDS MINOR CORRECTION.** The pure interaction contract preserves the recovered Rust behavior. One caller lifecycle statement incorrectly attributes suspend cancellation to that source.
- **Quality: PASS WITH ONE MINOR FINDING for preparation.** The bounded generic API, deterministic contact selection, explicit rejection and ownership boundaries are coherent. Implementability and runtime qualification remain unproved; the new eight-contact policy needs parent acceptance before dispatch.

## Findings

### M1 — Suspend cancellation is described as existing source behavior

- **Severity:** Minor (preparation/source-parity defect).
- **File:line:** `task-3i2-interaction-dispatch-binding.md:68`; contrary evidence `src/app.rs:2130`–2143; actual focus-loss cancellation `src/app.rs:2411`.
- **Description:** “Caller guide-open/reset/focus-loss/suspend clears controller under source rules” includes suspend, but the Rust `suspended` callback clears `InputState`, frame and GPU state without cancelling or replacing `self.interaction`. The app-owned controller therefore retains its contacts through this callback. An implementation following this sentence would add lifecycle behavior while claiming source parity. Pure `cancel` clearing all contacts and its harmless repetition are correctly specified.
- **Suggestion:** Separate verified caller behavior from proposed lifecycle policy. Remove suspend from the source-parity list, or identify suspend cancellation as a deliberate new policy requiring parent disposition. Give any separate “reset” claim its concrete caller reference rather than implying every reset pathway clears contacts.
- **Status:** Open; documentation correction required before implementation. No source changes made.

No Important or Critical defects found in the reviewed preparation.

## Reviewed contract and evidence

Read the complete binding and preparation report against the accepted routing intake, execution brief and FIX1 review, complete Rust interaction controller including tests, pointer/gesture routing and touch/lifecycle callers, and stable Zig scalar math, picking and typed Session boundaries. Verified the eight binding-table source identities and both preparation hashes with a read-only hash command (exit 0).

The proposal correctly retains exact owner precedence; raw shared u64 IDs and cross-source collisions; finite-position admission before duplicate/capacity checks; sticky distance >=6; last-update orbit deltas; release-only secondary launch; end-only threshold suppression; nonfinite update no-op and nonfinite end removal using the retained position. Derived finite-input overflow is distinguished from admission validity, avoiding an unsupported finite-output promise.

Pinch behavior matches the source: only exactly two non-HUD touch contacts are assigned on touch begin; mouse/HUD contacts are excluded; ascending IDs preserve deterministic pair order; baseline replacement precedes invalid-distance rejection; zoom uses the logarithm difference times 600; coincident transitions suppress emission and permit later recovery. A third scene touch remains independent, a HUD touch can reset the existing pair baseline, ending a non-Pinch third does not reform the pair, and ending a Pinch contact suppresses its surviving partner with moved TapOnly ownership. Cancel clears the whole model. Per-event current context and capability validation remain consumer obligations rather than being falsely attributed to the pure model.

The eight-contact cap is a new bound over unbounded Rust BTreeMap storage. The proposal makes ContactLimit explicit and guarantees pure-state rollback without claiming rollback of caller focus effects. It prohibits eviction/coalescing and acknowledges admission ordering and host response as unresolved. No silent-loss or host-device-limit claim is accepted here. Self-contained immutable HudAction values and explicit generic specialization constrain lifetimes; entry/controller size ceilings are qualification requirements, not measurements.

## Verifiable gaps and prerequisites

- Parent acceptance of max_contacts=8 and the target storage ceilings is still required. More than eight distinct simultaneous raw IDs can exist in the source model; capacity usability and failure handling cannot be inferred from a presumed device maximum.
- The future router must define ContactLimit handling and admission before caller effects, raw-ID collision handling, finite-derived-value handling, current HUD capability/context checks, and cancel/lifecycle delivery. This review authorizes none of those choices.
- No interaction implementation, executed literal fixtures, allocator/storage measurements, compiler/test/gate results or native/browser host delivery evidence exist for this preparation. Fixtures and API declarations are Proposed only.
- The separately accepted queue ruling is parent-provided context: eight slots of at most 4096 bytes, 32768 slot storage plus measured Header, explicit EventLimit/QueueFull and no raw IME cap. Binding line 85's queue-Proposed wording predates that ruling; it does not change the interaction cap's still-pending status.

## Frozen identities and scope

- Binding SHA-256: `378b2300aa874b3a16317999be49daf736ac54e213607e95f48121f9df93e768`.
- Preparation report SHA-256: `9a052f05f4267f4001a972807a9f98f2fd4a7cea024a1654f05192191f7849f5`.

Only this new review file was written. No transient guide/root README, product, Git, dependencies, assets, compiler, tests, gate or subagents were used or changed for this review.
